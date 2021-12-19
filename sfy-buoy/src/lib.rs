#![feature(derive_default_enum)]
#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

use ambiq_hal::rtc::Rtc;
use chrono::NaiveDateTime;
use core::cell::RefCell;
use core::ops::DerefMut;
use core::fmt::Debug;
use cortex_m::interrupt::{free, Mutex};
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

pub mod note;
pub mod waves;

pub struct SharedState {
    pub rtc: Rtc,
    pub lon: f32,
    pub lat: f32,
}

#[derive(Clone)]
pub enum LocationState {
    Trying(i64),
    Retrieved(i64),
}

#[derive(Clone)]
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
        state: &Mutex<RefCell<Option<SharedState>>>,
        delay: &mut impl DelayMs<u16>,
        note: &mut note::Notecarrier<T>,
    ) -> Result<(), notecard::NoteError> {
        use notecard::card::res::Location;
        use LocationState::*;

        const LOCATION_DIFF: i64 = 1 * 60_000; // ms

        let now = free(|cs| {
            let state = state.borrow(cs).borrow();
            let state = state.as_ref().unwrap();

            state.rtc.now().timestamp_millis()
        });
        defmt::trace!("now: {}", now);

        match self.state {
            Retrieved(t) | Trying(t) if (now - t) > LOCATION_DIFF => {
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

                    free(|cs| {
                        let mut state = state.borrow(cs).borrow_mut();
                        let state: &mut _ = state.deref_mut().as_mut().unwrap();

                        state.rtc.set(NaiveDateTime::from_timestamp(time as i64, 0));
                        state.lat = lat;
                        state.lon = lon;

                        self.state = Retrieved(state.rtc.now().timestamp_millis());
                    });
                } else {
                    self.state = Trying(now);
                }
            }
            _ => (),
        }

        Ok(())
    }
}

pub struct Imu<E: Debug, I: Write<Error = E> + WriteRead<Error = E>> {
    pub queue: heapless::spsc::Producer<'static, note::AxlPacket, 16>,
    pub last_poll: i64,
    waves: waves::Waves<I>,
}

impl<E: Debug, I: Write<Error = E> + WriteRead<Error = E>> Imu<E, I> {
    pub fn new(
        waves: waves::Waves<I>,
        queue: heapless::spsc::Producer<'static, note::AxlPacket, 16>,
    ) -> Imu<E, I> {
        Imu {
            queue,
            last_poll: 0,
            waves,
        }
    }

    pub fn check_retrieve(&mut self, now: i64, lon: f32, lat: f32) -> Result<(), E> {
        const IMU_BUF_DIFF: i64 = 1000; // ms

        if (now - self.last_poll) > IMU_BUF_DIFF {
            info!(
                "Polling IMU.. (now: {}, last_poll: {})",
                now, self.last_poll
            );

            self.waves.read_and_filter()?;

            if self.waves.axl.is_full() {
                info!("waves buffer is full, pushing to queue..");
                let pck = self.waves.take_buf(now as u32, lon, lat)?;
                self.queue.enqueue(pck).unwrap(); // TODO: fix
            } else {
                self.last_poll = now;
            }
        }

        Ok(())
    }
}
