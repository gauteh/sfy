#![feature(test)]
#![feature(derive_default_enum)]
#![feature(inline_const)]
#![feature(const_option_ext)]
#![feature(result_option_inspect)]
#![cfg_attr(not(feature = "host-tests"), no_std)]

#[cfg(test)]
extern crate test;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

use ambiq_hal::rtc::Rtc;
use chrono::NaiveDateTime;
use core::cell::RefCell;
use core::fmt::Debug;
use core::ops::DerefMut;
use cortex_m::interrupt::{free, Mutex};
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

pub mod log;
pub mod axl;
pub mod note;
pub mod waves;
pub mod fir;
#[cfg(feature = "storage")]
pub mod storage;

use axl::AxlPacket;

pub struct SharedState {
    pub rtc: Rtc,
    pub lon: f64,
    pub lat: f64,
}

pub trait State {
    fn now(&self) -> NaiveDateTime;
}

impl State for Mutex<RefCell<Option<SharedState>>> {
    fn now(&self) -> NaiveDateTime {
        free(|cs| {
            let state = self.borrow(cs).borrow();
            let state = defmt::unwrap!(state.as_ref());

            state.rtc.now()
        })
    }
}

#[derive(Clone)]
pub enum LocationState {
    Trying(i64),
    Retrieved(i64),
}

#[derive(Clone)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
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
        use notecard::card::res::{Location, Time};
        use LocationState::*;

        const LOCATION_DIFF: i64 = 1 * 60_000; // ms

        let now = state.now().timestamp_millis();
        defmt::trace!("now: {}", now);

        match self.state {
            Retrieved(t) | Trying(t) if (now - t) > LOCATION_DIFF => {
                let gps = note.card().location()?.wait(delay)?;
                let tm = note.card().time()?.wait(delay);

                info!("Location: {:?}, Time: {:?}", gps, tm);
                if let (
                    Location {
                        lat: Some(lat),
                        lon: Some(lon),
                        ..
                    },
                    Ok(Time {
                        time: Some(time), ..
                    }),
                ) = (gps, tm)
                {
                    info!("got time and location, setting RTC.");

                    self.lat = lat;
                    self.lon = lon;
                    self.time = time;

                    free(|cs| {
                        let mut state = state.borrow(cs).borrow_mut();
                        let state: &mut _ = defmt::unwrap!(state.deref_mut().as_mut());

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

pub struct Imu<E: Debug + defmt::Format, I: Write<Error = E> + WriteRead<Error = E>> {
    pub queue: heapless::spsc::Producer<'static, AxlPacket, 32>,
    waves: waves::Waves<I>,
}

impl<E: Debug + defmt::Format, I: Write<Error = E> + WriteRead<Error = E>> Imu<E, I> {
    pub fn new(
        waves: waves::Waves<I>,
        queue: heapless::spsc::Producer<'static, AxlPacket, 32>,
    ) -> Imu<E, I> {
        Imu { queue, waves }
    }

    pub fn check_retrieve(&mut self, now: i64, lon: f64, lat: f64) -> Result<(), waves::ImuError<E>> {
        trace!("Polling IMU.. (now: {})", now,);

        self.waves.read_and_filter()?;

        if self.waves.is_full() {
            trace!("waves buffer is full, pushing to queue..");
            let pck = self.waves.take_buf(now, lon, lat)?;
            match self.queue.enqueue(pck) {
                Ok(_) => (),
                Err(pck) => error!("queue is full, discarding data: {}", pck.data.len()),
            };
        }

        Ok(())
    }
}
