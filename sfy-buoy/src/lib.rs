#![feature(derive_default_enum)]
#![cfg_attr(not(test), no_std)]

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

// we use this for defs of sinf etc.
extern crate cmsis_dsp;

use ambiq_hal::rtc::Rtc;
use chrono::NaiveDateTime;
use core::fmt::Debug;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

pub mod note;
pub mod waves;

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

        const LOCATION_DIFF: i64 = 1 * 60_000; // ms
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

enum QueueDrain {
    Waiting,
    Draining,
}

pub struct Imu {
    pub dequeue: heapless::Deque<note::AxlPacket, 60>,
    pub last_poll: i64,
    queue_state: QueueDrain,
}

impl Imu {
    pub fn new() -> Imu {
        Imu {
            dequeue: heapless::Deque::new(),
            last_poll: 0,
            queue_state: QueueDrain::Waiting,
        }
    }

    pub fn check_retrieve<E: Debug, I: Write<Error = E> + WriteRead<Error = E>>(
        &mut self,
        rtc: &mut Rtc,
        waves: &mut waves::Waves<I>,
        location: &Location,
    ) -> Result<(), E> {
        const IMU_BUF_DIFF: i64 = 100; // ms
        let now = rtc.now().timestamp_millis();

        if (now - self.last_poll) > IMU_BUF_DIFF {
            info!("Polling IMU..");
            self.last_poll = now;

            waves.read_and_filter()?;

            if waves.axl.is_full() {
                info!("waves buffer is full, pushing to queue..");
                let pck = waves.take_buf(now as u32, location.lon, location.lat)?;
                self.dequeue.push_back(pck).unwrap(); // TODO: fix
            }
        }

        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn check_drain_queue<T: Write + Read>(
        &mut self,
        note: &mut note::Notecarrier<T>,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<(), notecard::NoteError> {
        use QueueDrain::*;

        let DRAIN_LOWER: usize = 0;
        let DRAIN_UPPER: usize = self.dequeue.capacity() / 3 * 2;

        let n = self.dequeue.len();

        match self.queue_state {
            Waiting => {
                if n >= DRAIN_UPPER {
                    defmt::info!("starting to drain queue..");
                    self.queue_state = Draining;
                }
            },
            Draining => {
                if n <= DRAIN_LOWER {
                    defmt::info!("queue almost empty, stopping.");
                    self.queue_state = Waiting;
                }

                if let Some(pck) = self.dequeue.pop_front() {
                    defmt::debug!("scheduling package to be sent: {:?}", pck);
                    note.send(pck, delay)?;
                }
            }
        }

        Ok(())
    }
}
