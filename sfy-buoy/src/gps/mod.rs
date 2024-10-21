//! GPS interface
//!
use chrono::{NaiveDate, NaiveDateTime};
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::serial::{Read, Write};
use heapless::{
    spsc::{Producer, Queue},
    Vec,
};

pub const GPS_PACKET_V: u8 = 1;
pub const GPS_PACKET_SZ: usize = 3 * 1024;
pub const GPS_FREQ: f32 = 20.0;
/// Maximum length of base64 string from
pub const GPS_OUTN: usize = { 3 * GPS_PACKET_SZ * 2 } * 4 / 3 + 4;

mod wire;
pub use wire::*;

use crate::waves::wire::ScaledF32;
use crate::EPGS_SZ;

/// Queue from GPS to Notecard
pub static mut EGPSQ: Queue<GpsPacket, { crate::EPGS_SZ }> = Queue::new();

#[derive(serde::Deserialize, PartialEq, defmt::Format, Debug)]
pub struct Sample {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    sec: u8,
    nano: u32,

    time_acc: u32, // ns

    lon: i32, // deg * 1e7
    lat: i32, // deg * 1e7
    msl: i32, // mm
    hor_acc: u32, // mm
    vert_acc: u32, // mm

    fix: u8,
    soln: u8,
}

impl Sample {
    pub fn timestamp(&self) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(self.year.into(), self.month.into(), self.day.into())
            .unwrap()
            .and_hms_nano(
                self.hour.into(),
                self.minute.into(),
                self.sec.into(),
                self.nano.into(),
            )
    }
}

/// A packet of GPS samples
#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GpsPacket {
    /// Timestamp of first sample
    pub timestamp: i64,

    pub freq: f32,

    pub version: u8,

    /// Reference position for which the data is relative to. Mean of all samples.
    pub lon: i32,
    pub lat: i32,
    pub msl: i32,

    /// GPS data. This is moved to payload when transmitting.
    pub data: Vec<u16, { 3 * GPS_PACKET_SZ }>,
}

// XXX: Match with template in note
#[derive(serde::Serialize, Default)]
pub struct GpsPacketMeta {
    /// Timestamp of first sample
    pub timestamp: i64,

    pub freq: f32,

    pub version: u8,

    /// Reference position for which the data is relative to. Mean of all samples.
    pub lon: i32,
    pub lat: i32,
    pub msl: i32,

    pub lonlat_range: f32, // 1e-7 deg
    pub msl_range: f32,    // mm
    pub length: u32,
}

impl GpsPacket {
    pub fn len(&self) -> usize {
        self.data.len() / 3
    }

    pub fn base64(&self) -> Vec<u8, GPS_OUTN> {
        let mut b64: Vec<_, GPS_OUTN> = Vec::new();
        b64.resize_default(GPS_OUTN).unwrap();

        // Check endianness (TODO:  swap order if compiled for big endian machine).
        #[cfg(target_endian = "big")]
        compile_error!("serializied samples are assumed to be in little endian, target platform is big endian and no conversion is implemented.");

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut b64);
        b64.truncate(written);

        b64
    }

    /// Split package into metadata and payload.
    pub fn split(&self) -> (GpsPacketMeta, Vec<u8, GPS_OUTN>) {
        let b64 = self.base64();

        let meta = GpsPacketMeta {
            timestamp: self.timestamp,
            length: b64.len() as u32,
            freq: self.freq,
            lonlat_range: wire::LON_RANGE,
            msl_range: wire::MSL_RANGE,
            version: GPS_PACKET_V,
            lon: self.lon,
            lat: self.lat,
            msl: self.msl,
        };

        (meta, b64)
    }
}

pub struct Gps<U>
where
    U: Read<u8> + Write<u8>,
{
    gps: U,
    queue: Producer<'static, GpsPacket, EPGS_SZ>,
    buf: Vec<Sample, { GPS_PACKET_SZ }>,
}

enum ParseState {
    StartBracket,
    Body,
    EndBracket,
}

impl<U> Gps<U>
where
    U: Read<u8> + Write<u8>,
{
    pub fn new(gps: U, queue: Producer<'static, GpsPacket, EPGS_SZ>) -> Gps<U> {
        Gps {
            gps,
            queue,
            buf: Vec::new(),
        }
    }

    pub fn check_collect(&mut self) {
        if self.buf.is_full() {
            self.collect();
        }
    }

    pub fn collect(&mut self) {
        let N: f64 = self.buf.len() as f64;

        // Mean of lon, lat and msl
        let (lon, lat, msl) = self
            .buf
            .iter()
            .map(|s| (s.lon, s.lat, s.msl))
            .reduce(|(mlon, mlat, mmsl), (lon, lat, msl)| (mlon + lon, mlat + lat, mmsl + msl))
            .unwrap();
        let (lon, lat, msl) = (lon as f64 / N, lat as f64 / N, msl as f64 / N);
        let (lon, lat, msl) = (lon as i32, lat as i32, msl as i32);

        // Subtract mean and serialize as interleaved u16's
        let data: Vec<u16, { 3 * GPS_PACKET_SZ }> = self
            .buf
            .iter()
            .map(|s| {
                [
                    Lon16::from_i32(s.lon - lon).to_u16(),
                    Lat16::from_i32(s.lat - lat).to_u16(),
                    Msl16::from_i32(s.msl - msl).to_u16(),
                ]
            })
            .flatten()
            .collect();

        let timestamp = self.buf[0].timestamp().timestamp_millis();

        self.buf.clear();

        let p = GpsPacket {
            timestamp,
            freq: GPS_FREQ,
            version: GPS_PACKET_V,
            lon,
            lat,
            msl,
            data,
        };

        self.queue.enqueue(p);
    }

    pub fn sample(&mut self) {
        let mut buf = heapless::Vec::<u8, 1024>::new(); // reduce?

        defmt::debug!("Reading GPS package from serial..");
        let mut state = ParseState::StartBracket;

        while !matches!(state, ParseState::EndBracket) {
            if self.buf.len() == self.buf.capacity() {
                defmt::error!("gps telegram buffer is full.");
                break;
            }

            match self.gps.read() {
                Ok(w) => {
                    debug!("read: {}", w);
                    match state {
                        ParseState::StartBracket => {
                            if w == b'{' {
                                buf.push(w).ok();
                                state = ParseState::Body;
                            } else {
                                debug!("extra char discarded: {}", unsafe {
                                    char::from_u32_unchecked(w as u32)
                                });
                            }
                        }
                        ParseState::Body => {
                            if w == b'}' {
                                buf.push(w).ok();
                                state = ParseState::EndBracket;
                            } else {
                                buf.push(w).ok();
                            }
                        }
                        ParseState::EndBracket => {
                            break;
                        }
                    }
                }

                Err(nb::Error::WouldBlock) => { /* wait */ } // TODO: timeout!
                Err(nb::Error::Other(_)) => {
                    defmt::error!("ext-gps: error reading from uart");
                }
            }
        }

        // ready to parse `buf`.
        defmt::debug!("Parsing GPS package..: {}", unsafe {
            core::str::from_utf8_unchecked(&buf)
        });

        match serde_json_core::from_slice::<Sample>(&buf) {
            Ok((sample, _sz)) => {
                defmt::debug!("Sample: {}", sample);
                self.buf
                    .push(sample)
                    .inspect_err(|_| {
                        defmt::error!("GPS sample buffer is full! Discarding latest sample.")
                    })
                    .ok();

                // TODO: set the RTC:
                // let now = ...;
                // let current = sample.time + (now - pps_time);
                // drift = current - now
            }
            Err(_) => error!("Failed to parse GPS telegram: {}", &buf),
        }

        // TODO: Not really handling extra data.
    }
}

#[cfg(test)]
mod tests {
    use super::{Sample, GpsPacket, GPS_PACKET_SZ};

    #[test]
    fn test_deser_sample() {
        // doc["year"] = year;
        // doc["month"] = month;
        // doc["day"] = day;
        // doc["hour"] = hour;
        // doc["minute"] = minute;
        // doc["sec"] = sec;
        // doc["nano_sec"] = nano;
        // doc["Time (UTC)"] = datetime;
        // doc["Time Accuracy (ns)"] = tAcc;
        // doc["Lat (deg * 10e-7)"] = latitude;
        // doc["Lon (deg * 10e-7)"] = longitude;
        // doc["Height above MSL(mm)"] = altitude;
        // doc["Horizontal Accuracy (mm)"] = hAcc;
        // doc["Vertical Accuracy (mm)"] = vAcc;
        // doc["Carrier Soln"] = carrSoln;
        // doc["Fix Type"] = fixType;

        let sample = r#"
{ "year": 2024,
  "month": 10,
  "day": 23,
  "hour" : 12,
  "minute": 13,
  "sec": 54,
  "nano": 12332,
  "time_acc": 504,
  "lat": 654003034,
  "lon": 42342344,
  "msl": 3000,
  "hor_acc": 200,
  "vert_acc": 700,
  "soln": 1,
  "fix": 1 }
            "#;

        let s: Sample = serde_json_core::from_str(sample).unwrap().0;

        println!("sample: {s:#?}");
    }

    #[test]
    fn base64_data_package() {
        let p = GpsPacket {
            timestamp: 0,
            lat: 0,
            lon: 0,
            msl: 20,
            freq: 100.0,
            version: super::GPS_PACKET_V,
            data: (0..(3*GPS_PACKET_SZ))
                .map(|v| v as u16)
                .collect::<heapless::Vec<_, { 3 * GPS_PACKET_SZ }>>(),
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }
}
