//! GPS interface
//!
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use heapless::Vec;

#[derive(serde::Deserialize, PartialEq, defmt::Format, Debug)]
pub struct Sample {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    sec: u8,
    nano: i32,

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

pub struct Gps {
    buf: Vec<Sample, 1024>,
}

impl Gps {
    pub const fn new() -> Gps {
        Gps { buf: Vec::new() }
    }

    pub fn sample<R>(&mut self, gps: &mut R)
    where
        R: embedded_hal::serial::Read<u8>,
        R::Error: defmt::Format,
    {
        let mut buf = heapless::Vec::<u8, 1024>::new(); // reduce?

        defmt::debug!("Reading GPS package from serial..");
        loop {
            match gps.read() {
                Ok(w) => {
                    debug!("read: {}", w);
                    defmt::flush();
                    match w {
                        b'\n' => {
                            break;
                        }
                        w => match buf.push(w) {
                            Ok(_) => {}
                            Err(_) => {
                                defmt::error!("ext-gps: gps read buf is full.");
                                break;
                            }
                        },
                    }
                }

                Err(nb::Error::WouldBlock) => { /* wait */ } // TODO: timeout!
                Err(nb::Error::Other(e)) => {
                    defmt::error!("ext-gps: error reading from uart: {}", e);
                    break;
                }
            }
        }

        // ready to parse `buf`.
        defmt::debug!("Parsing GPS package..");

        match serde_json_core::from_slice::<Sample>(&buf) {
            Ok((sample, _sz)) => {
                defmt::debug!("Sample: {}", sample);
                self.buf
                    .push(sample)
                    .inspect_err(|_| {
                        defmt::error!("GPS sample buffer is full! Discarding samples.")
                    })
                    .ok();
            }
            Err(_) => error!("Failed to parse GPS telegram: {}", &buf),
        }

        // set the RTC:
        // let now = ...;
        // let current = sample.time + (now - pps_time);
        // drift = current - now

        // make sure there is nothing in the uart now, otherwise it should be drained.
        while let Ok(_) = gps.read() {
            defmt::error!("ext-gps: more data on uart after PPS parsing. discarding.");
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
