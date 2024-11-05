//! GPS interface
//!
use chrono::{NaiveDate, NaiveDateTime};
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use embedded_hal::serial::{Read, Write};
use heapless::{
    spsc::{Producer, Queue},
    Vec,
};

pub const GPS_PACKET_V: u8 = 3;
pub const GPS_PACKET_SZ: usize = 124;
/// Maximum length of base64 string from
pub const GPS_OUTN: usize = { 6 * GPS_PACKET_SZ * 2 } * 4 / 3 + 4;

mod wire;
pub use wire::*;

use crate::waves::wire::ScaledF32;
use crate::EPGS_SZ;

/// Queue from GPS to Notecard
pub static mut EGPSQ: Queue<GpsPacket, { crate::EPGS_SZ }> = Queue::new();

#[derive(serde::Deserialize, PartialEq, Clone, defmt::Format)]
pub struct EgpsTime {
    pub time: i64,     // The time received from the GPS (milliseconds).
    pub pps_time: i64, // The time of the RTC at the time of the interrupt.
    pub lon: f64,
    pub lat: f64,
}

#[derive(serde::Deserialize, PartialEq, Clone)]
pub struct Sample {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    sec: u8,
    nano: i32,

    time_acc: u32, // ns

    lon: i32,      // deg * 1e7
    lat: i32,      // deg * 1e7
    msl: i32,      // mm
    hor_acc: u32,  // mm
    vert_acc: u32, // mm

    velN: i32, // mm/s
    velE: i32, // mm/s
    velD: i32, // mm/s
    sAcc: i32, // mm/s

    fix: u8,
    soln: u8,
}

impl Sample {
    pub fn timestamp(&self) -> Option<NaiveDateTime> {
        let mut sec = self.sec;
        let mut nano = self.nano;

        if nano < 0 {
            sec -= 1;
            nano = 1_000_000_000 + nano;
        }

        let nano = nano as u32;

        NaiveDate::from_ymd_opt(self.year.into(), self.month.into(), self.day.into()).and_then(
            |t| {
                t.and_hms_nano_opt(
                    self.hour.into(),
                    self.minute.into(),
                    sec.into(),
                    nano.into(),
                )
            },
        )
    }

    pub fn lonlat(&self) -> (f64, f64) {
        let lon = self.lon as f64 / 1.0e7;
        let lat = self.lat as f64 / 1.0e7;

        (lon, lat)
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
    pub data: Vec<u16, { 6 * GPS_PACKET_SZ }>,

    pub ha_min: f32,
    pub ha_max: f32,
    pub ha_mean: f32,

    pub va_min: f32,
    pub va_max: f32,
    pub va_mean: f32,

    pub fix: [u8; 8],
    pub soln: [u8; 8],
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
    pub vel_range: f32,    // mm/s
    pub length: u32,

    pub ha_min: f32,
    pub ha_max: f32,
    pub ha_mean: f32,

    pub va_min: f32,
    pub va_max: f32,
    pub va_mean: f32,

    pub fix: [u8; 8],
    pub soln: [u8; 8],
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
            vel_range: wire::VEL_RANGE,
            version: GPS_PACKET_V,
            lon: self.lon,
            lat: self.lat,
            msl: self.msl,
            ha_min: self.ha_min,
            ha_max: self.ha_max,
            ha_mean: self.ha_mean,
            va_min: self.va_min,
            va_max: self.va_max,
            va_mean: self.va_mean,
            fix: self.fix,
            soln: self.soln,
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
            defmt::info!(
                "GPS buf is full, collecting into GpsPacket, queue len: {}",
                self.queue.len()
            );
            self.collect();
        }
    }

    pub fn collect(&mut self) {
        if self.buf.is_empty() {
            return;
        }

        // Use first sample as reference of lon, lat and msl
        let s = &self.buf[0];
        let (lon, lat, msl) = (s.lon, s.lat, s.msl);

        // Subtract ref and serialize as interleaved u16's
        let data: Vec<u16, { 6 * GPS_PACKET_SZ }> = self
            .buf
            .iter()
            .map(|s| {
                [
                    Lon16::from_i32(s.lon - lon).to_u16(),
                    Lat16::from_i32(s.lat - lat).to_u16(),
                    Msl16::from_i32(s.msl - msl).to_u16(),
                    Vel16::from_i32(s.velN).to_u16(),
                    Vel16::from_i32(s.velE).to_u16(),
                    Vel16::from_i32(s.velD).to_u16(),
                ]
            })
            .flatten()
            .collect();

        // unwrap: is not added to buf unless timestamp is parsable.
        let timestamp = self.buf[0]
            .timestamp()
            .unwrap()
            .and_utc()
            .timestamp_millis();

        // TODO: skip this iteration, maybe just use the first two..
        let freq: f32 = self
            .buf
            .windows(2)
            .map(|a| {
                a[1].timestamp().unwrap().and_utc().timestamp_millis()
                    - a[0].timestamp().unwrap().and_utc().timestamp_millis()
            })
            .sum::<i64>() as f32
            / self.buf.len() as f32;
        let freq = 1000.0 / freq;

        // Calculate some stats on accuracy and fix
        let (mut ha_min, mut ha_mean, mut ha_max) = (0.0f32, 0.0f32, 0.0f32);
        let (mut va_min, mut va_mean, mut va_max) = (0.0f32, 0.0f32, 0.0f32);
        let mut fix = [0u8; 8];
        let mut soln = [0u8; 8];

        for sample in self.buf.iter() {
            ha_min = ha_min.min(sample.hor_acc);
            ha_max = ha_min.max(sample.hor_acc);
            ha_mean += sample.hor_acc / N;

            va_min = va_min.min(sample.vert_acc);
            va_max = va_min.max(sample.vert_acc);
            va_mean += sample.vert_acc / N;

            fix[sample.fix] += 1;
            soln[sample.soln] += 1;
        }

        self.buf.clear();

        let p = GpsPacket {
            timestamp,
            freq,
            version: GPS_PACKET_V,
            lon,
            lat,
            msl,
            data,

            ha_min,
            ha_max,
            ha_mean,

            va_min,
            va_max,
            va_mean,

            fix,
            soln,
        };

        debug!(
            "Collected egps packet, freq: {}, timestamp: {}, total len (u16s): {}",
            p.freq,
            p.timestamp,
            p.data.len()
        );

        self.queue
            .enqueue(p)
            .inspect_err(|_| defmt::error!("could not enque GpsPacket.."));
    }

    pub fn sample(&mut self) -> Option<&Sample> {
        let mut buf = heapless::Vec::<u8, 1024>::new(); // reduce?

        // defmt::debug!(
        //     "Reading GPS package from serial.. sample buf: {}",
        //     self.buf.len()
        // );
        let mut state = ParseState::StartBracket;

        let mut timeout = 0_u32;

        while !matches!(state, ParseState::EndBracket) {
            if self.buf.len() == self.buf.capacity() {
                defmt::error!("gps telegram buffer is full.");
                break;
            }

            if timeout > 5_000_000 {
                defmt::error!("gps: uart timed out.");
                break;
            }

            match self.gps.read() {
                Ok(w) => {
                    timeout = 0;
                    match state {
                        ParseState::StartBracket => {
                            if w == b'{' {
                                buf.push(w).ok();
                                state = ParseState::Body;
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

                Err(nb::Error::WouldBlock) => {
                    timeout += 1;
                } // TODO: timeout!
                Err(nb::Error::Other(_)) => {
                    defmt::error!("ext-gps: error reading from uart");
                    timeout += 1;
                }
            }
        }

        // ready to parse `buf`.
        // defmt::debug!("Parsing GPS package..: {}", unsafe {
        //     core::str::from_utf8_unchecked(&buf)
        // });

        let sample = serde_json_core::from_slice::<Sample>(&buf);

        let sample = sample.and_then(|(sample, _sz)| {
            if sample.timestamp().is_some() {
                Ok(sample)
            } else {
                Err(serde_json_core::de::Error::CustomError)
            }
        });

        match sample {
            Ok(sample) => {
                // defmt::debug!("Sample: {}", sample);
                self.buf
                    .push(sample)
                    .inspect_err(|_| {
                        defmt::error!("GPS sample buffer is full! Discarding latest sample.")
                    })
                    .ok();
            }
            Err(_) => {
                error!("Failed to parse GPS telegram: {}", unsafe {
                    core::str::from_utf8_unchecked(&buf)
                });

                // collecting package to avoid getting mis-timed samples
                warn!("collecting egps package, to avoid mis-timed samples.");
                self.collect();
                return None;
            }
        }

        // TODO: Not really handling extra data.
        self.buf.last()
    }
}

#[cfg(test)]
mod tests {
    use super::{GpsPacket, Sample, GPS_PACKET_SZ};

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
  "fix": 1 ,"velN":-41,"velE":53,"velD":0,"sAcc":455}
            "#;

        let s: Sample = serde_json_core::from_str(sample).unwrap().0;

        // println!("sample: {s:#?}");
    }

    #[test]
    #[should_panic]
    fn parse_veln() {
        let s = r#"{"year":2024,"month":10,"day":26"year":2024,"month":10,"day":26,"hour":7,"minute":16,"sec":34,"nano":400139504,"time_acc":47,"lat":603283447,"lon":53677011,"msl":91506,"hor_acc":11058,"vert_acc":14322,"soln":0,"fix":3,"velN":-41,"velE":53,"velD":0,"sAcc":455}"#;

        let s: Sample = serde_json_core::from_str(s).unwrap().0;
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
            data: (0..(6 * GPS_PACKET_SZ))
                .map(|v| v as u16)
                .collect::<heapless::Vec<_, { 6 * GPS_PACKET_SZ }>>(),
            ha_mean: 10.0,
            ha_min: 10.0,
            ha_max: 10.0,
            va_mean: 10.0,
            va_min: 10.0,
            va_max: 10.0,
            fix: [0u8; 8],
            soln: [0u8; 8],
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }
}
