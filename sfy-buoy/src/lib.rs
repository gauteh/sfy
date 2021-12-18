#![feature(derive_default_enum)]
#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

use core::fmt::Debug;
use ambiq_hal::rtc::Rtc;
use chrono::NaiveDateTime;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

pub mod note;
pub mod waves;

const LOCATION_DIFF: i64 = 1 * 60_000; // ms

pub enum LocationState {
    Trying(i64),
    Retrieved(i64),
}

pub struct Location {
    pub lat: f32,
    pub lon: f32,
    pub time: u32,

    pub state: LocationState,
}

impl Location {
    pub fn new() -> Location {
        Location {
            lat: 0.0,
            lon: 0.0,
            time: 0,
            state: LocationState::Trying(-999),
        }
    }

    pub fn check_retrieve<T: Read + Write>(
        &mut self,
        rtc: &mut Rtc,
        delay: &mut impl DelayMs<u16>,
        note: &mut note::Notecarrier<T>,
    ) -> Result<(), notecard::NoteError> {
        use notecard::card::res::Location;
        use LocationState::*;

        let now = rtc.now().timestamp_millis();

        match self.state {
            Retrieved(t) | Trying(t) if (now - t) > LOCATION_DIFF => {
                // Try to get time and location
                let gps = note.card().location()?.wait(delay)?;

                info!("Location: {:?}", gps);
                if let Location {
                    lat: Some(lat),
                    lon: Some(lon),
                    time: Some(time),
                    ..
                } = gps
                {
                    info!("got time and location, setting RTC.");

                    self.lat = lat;
                    self.lon = lon;
                    self.time = time;
                    self.state = Retrieved(time as i64);

                    rtc.set(NaiveDateTime::from_timestamp(time as i64, 0));
                }
            }
            _ => (),
        }

        Ok(())
    }
}

const IMU_BUF_DIFF: i64 = 100; // ms

#[derive(Default)]
pub struct Imu {
    pub dequeue: heapless::Deque<note::AxlPacket, 60>,
    pub last_poll: i64,
}

impl Imu {
    pub fn check_retrieve<E: Debug, I: Write<Error = E> + WriteRead<Error = E>>(
        &mut self,
        rtc: &mut Rtc,
        waves: &mut waves::Waves<I>,
    ) -> Result<(), E> {
        let now = rtc.now().timestamp_millis();

        if (now - self.last_poll) > IMU_BUF_DIFF {
            info!("Polling IMU..");
            self.last_poll = now;

            waves.read_and_filter()?;

            if waves.axl.is_full() {
                let pck = waves.take_buf(now as u32)?;
                self.dequeue.push_back(pck).unwrap(); // TODO: fix
            }
        }

        Ok(())
    }
}
