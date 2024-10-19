//! GPS interface
//!
use chrono::{NaiveDate, NaiveDateTime};
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use embedded_hal::blocking::delay::DelayMs;
use heapless::Vec;

pub const GPS_PACKET_V: u8 = 1;
pub const GPS_PACKET_SZ: usize = 3 * 1024;
pub const GPS_FREQ: f32 = 20.0;

mod wire;
pub use wire::*;

use crate::waves::wire::ScaledF32;

#[derive(serde::Deserialize, PartialEq, defmt::Format, Debug)]
pub struct Sample {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    sec: u8,
    nano: u32,

    #[serde(alias = "Time Accuracy (ns)")]
    time_acc: f64,

    #[serde(alias = "Lon (deg * 10e-7)")]
    lon: f64,

    #[serde(alias = "Lat (deg * 10e-7)")]
    lat: f64,

    #[serde(alias = "Height above MSL(mm)")]
    msl: f64,

    #[serde(alias = "Horizontal Accuracy (mm)")]
    horizontal_acc: u32,

    #[serde(alias = "Vertical Accuracy (mm)")]
    vertical_acc: u32,

    #[serde(alias = "Fix Type")]
    fix_type: u8,

    #[serde(alias = "Carrier Soln")]
    carrier_solution: u8,
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
    pub lon: f64,
    pub lat: f64,
    pub msl: f64,

    /// GPS data. This is moved to payload when transmitting.
    pub data: Vec<u16, { 3 * GPS_PACKET_SZ }>,
}

impl GpsPacket {
    pub fn len(&self) -> usize {
        self.data.len() / 3
    }
}

pub struct Gps {
    buf: Vec<Sample, { GPS_PACKET_SZ }>,
}

enum ParseState {
    StartBracket,
    Body,
    EndBracket,
}

impl Gps {
    pub const fn new() -> Gps {
        Gps { buf: Vec::new() }
    }

    pub fn collect(&mut self) -> GpsPacket {
        let N: f64 = self.buf.len() as f64;

        let (lon, lat, msl) = self
            .buf
            .iter()
            .map(|s| (s.lon, s.lat, s.msl))
            .reduce(|(mlon, mlat, mmsl), (lon, lat, msl)| (mlon + lon, mlat + lat, mmsl + msl))
            .unwrap();
        let (lon, lat, msl) = (lon / N, lat / N, msl / N);

        let data: Vec<u16, { 3 * GPS_PACKET_SZ }> = self
            .buf
            .iter()
            .map(|s| {
                [
                    Lon16::from_f64(s.lon - lon).to_u16(),
                    Lat16::from_f64(s.lat - lat).to_u16(),
                    Msl16::from_f64(s.msl - msl).to_u16(),
                ]
            })
            .flatten()
            .collect();

        let timestamp = self.buf[0].timestamp().timestamp_millis();

        self.buf.clear();

        GpsPacket {
            timestamp,
            freq: GPS_FREQ,
            version: GPS_PACKET_V,
            lon,
            lat,
            msl,
            data,
        }
    }

    pub fn sample<R>(&mut self, gps: &mut R)
    where
        R: embedded_hal::serial::Read<u8> + embedded_hal::serial::Write<u8>,
        // R::Error: defmt::Format,
    {
        let mut buf = heapless::Vec::<u8, 1024>::new(); // reduce?

        defmt::debug!("Reading GPS package from serial..");
        let mut state = ParseState::StartBracket;

        while !matches!(state, ParseState::EndBracket) {
            if self.buf.len() == self.buf.capacity() {
                defmt::error!("gps telegram buffer is full.");
                break;
            }

            match gps.read() {
                Ok(w) => {
                    debug!("read: {}", w);
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
                        defmt::error!("GPS sample buffer is full! Discarding samples.")
                    })
                    .ok();

                // TODO: set the RTC:
                // let now = ...;
                // let current = sample.time + (now - pps_time);
                // drift = current - now
            }
            Err(_) => error!("Failed to parse GPS telegram: {}", &buf),
        }

        // Make sure there is nothing in the uart now, otherwise it should be drained.
        //
        // It is likely that the latest telegram is responsible for the PPS, so we are now likely
        // out-of-sync with the PPS and telegrams. Should hopefully resolve itself on the next
        // sample.
        while let Ok(w) = gps.read() {
            defmt::debug!("{}", unsafe { char::from_u32_unchecked(w as u32) });
            defmt::error!(
                "ext-gps: more data on uart after PPS parsing. discarding, PPS may be out of sync."
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sample;

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
  "Time (UTC)": "2024-10-23-12-13-54-12332",
  "Time Accuracy (ns)": 504.0,
  "Lat (deg * 10e-7)": 654003034.0,
  "Lon (deg * 10e-7)": 42342344.0,
  "Height above MSL(mm)": 3000,
  "Horizontal Accuracy (mm)": 200,
  "Vertical Accuracy (mm)": 700,
  "Carrier Soln": 1,
  "Fix Type": 1 }
            "#;

        let s: Sample = serde_json_core::from_str(sample).unwrap().0;

        println!("sample: {s:#?}");
    }
}
