#![feature(test)]
#![feature(try_blocks)]
#![feature(portable_simd)]
#![feature(array_chunks)]
#![cfg_attr(not(test), no_std)]
#![allow(non_upper_case_globals)]

#[cfg(test)]
extern crate test;

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

use blues_notecard as notecard;
use chrono::NaiveDateTime;
use core::cell::RefCell;
use core::fmt::Debug;
use core::ops::DerefMut;
use cortex_m::interrupt::{free, Mutex};
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write, WriteRead},
};

#[cfg(feature = "storage")]
use embedded_hal::{
    blocking::delay::DelayUs,
    blocking::spi::{Transfer, Write as DefaultWrite},
    digital::v2::OutputPin,
};

use rtcc::DateTimeAccess;

pub mod axl;
#[cfg(feature = "fir")]
pub mod fir;
pub mod log;
pub mod note;
#[cfg(feature = "storage")]
pub mod storage;
pub mod waves;

#[cfg(feature = "ext-gps")]
pub mod gps;

use axl::AxlPacket;
#[cfg(feature = "storage")]
use storage::Storage;
#[cfg(feature = "storage")]
use waves::AxlPacketT;

#[cfg(feature = "storage")]
pub type ImuAxlPacketT = waves::AxlPacketT;

#[cfg(not(feature = "storage"))]
pub type ImuAxlPacketT = axl::AxlPacket;

// With 'raw' enabled 3 * 2 more samples (compared to processed samples)
// need to be queued.
#[cfg(feature = "raw")]
pub const STORAGEQ_SZ: usize = 3;
#[cfg(all(feature = "raw", not(feature = "ext-gps")))]
pub const NOTEQ_SZ: usize = 6;

#[cfg(not(feature = "raw"))]
pub const STORAGEQ_SZ: usize = 12;
#[cfg(all(not(feature = "raw"), feature = "storage"))]
pub const NOTEQ_SZ: usize = 12;

#[cfg(feature = "storage")]
pub const IMUQ_SZ: usize = STORAGEQ_SZ;

#[cfg(feature = "ext-gps")]
pub const NOTEQ_SZ: usize = 6;

#[cfg(feature = "ext-gps")]
pub const EPGS_SZ: usize = 6;

#[cfg(all(
    not(feature = "raw"),
    not(feature = "storage"),
    not(feature = "ext-gps")
))]
pub const NOTEQ_SZ: usize = 24;

#[cfg(not(feature = "storage"))]
pub const IMUQ_SZ: usize = NOTEQ_SZ;

/// These queues are filled up by the IMU interrupt in read batches of time-series. It is then consumed
/// the main thread and first drained to the SD storage (if enabled), and then queued for the notecard.

/// Queue from IMU to Storage
#[cfg(feature = "storage")]
pub static mut STORAGEQ: heapless::spsc::Queue<AxlPacketT, STORAGEQ_SZ> =
    heapless::spsc::Queue::new();

/// Queue from Storage to Notecard
pub static mut NOTEQ: heapless::spsc::Queue<AxlPacket, NOTEQ_SZ> = heapless::spsc::Queue::new();

pub const FUTURE: NaiveDateTime = NaiveDateTime::from_timestamp(2550564072, 0);

pub struct SharedState<D: DateTimeAccess> {
    pub rtc: D,
    pub position_time: u32, // unix epoch [s]
    pub lon: f64,
    pub lat: f64,
}

pub trait State {
    fn now(&self) -> Option<NaiveDateTime>;

    /// Returns now, posistion_time, lat, lon.
    fn get(&self) -> (Option<NaiveDateTime>, u32, f64, f64);
}

impl<D: DateTimeAccess> SharedState<D> {
    fn now(&mut self) -> Option<NaiveDateTime> {
        self.rtc.datetime().ok()
    }

    fn get(&mut self) -> (Option<NaiveDateTime>, u32, f64, f64) {
        (
            self.rtc.datetime().ok(),
            self.position_time,
            self.lat,
            self.lon,
        )
    }
}

impl<D: DateTimeAccess> State for Mutex<RefCell<Option<SharedState<D>>>> {
    fn now(&self) -> Option<NaiveDateTime> {
        free(|cs| {
            let mut state = self.borrow(cs).borrow_mut();
            let state: &mut _ = state.deref_mut().as_mut().unwrap();

            state.now()
        })
    }

    fn get(&self) -> (Option<NaiveDateTime>, u32, f64, f64) {
        free(|cs| {
            let mut state = self.borrow(cs).borrow_mut();
            let state: &mut _ = state.deref_mut().as_mut().unwrap();

            state.get()
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
    pub position_time: u32, // unix epoch [s]

    pub state: LocationState,
}

impl Location {
    pub fn new() -> Location {
        Location {
            lat: 0.0,
            lon: 0.0,
            position_time: 0,
            state: LocationState::Trying(-999),
        }
    }

    #[cfg(feature = "ext-gps")]
    pub fn set_from_egps<D: DateTimeAccess>(
        &mut self,
        state: &Mutex<RefCell<Option<SharedState<D>>>>,
        egps: &Mutex<RefCell<Option<gps::EgpsTime>>>,
    ) {
        use LocationState::*;

        const LOCATION_DIFF: i64 = 10_000; // [ms]
        let now = state.now().unwrap_or(FUTURE).and_utc().timestamp_millis();

        free(|cs| {
            info!("Setting location from RTC (from EGPS).");
            let mut state = state.borrow(cs).borrow_mut();
            let state: &mut _ = state.deref_mut().as_mut().unwrap();

            let egps = egps.borrow(cs).borrow();
            let egps = egps.as_ref();

            if let Some(egps) = egps {
                info!("Updating postion from ext-gps: {:?}", egps);
                // Update internal state from EGPS
                self.position_time = (egps.time / 1000) as u32;
                self.lat = egps.lat;
                self.lon = egps.lon;

                // Update global STATE from egps
                state.position_time = self.position_time;
                state.lon = self.lon;
                state.lat = self.lat;

                match self.state {
                    Retrieved(t) | Trying(t) if (now - t) > LOCATION_DIFF => {
                        info!(
                            "More than {} passed since last clock set, setting..",
                            LOCATION_DIFF
                        );
                        let diff = now - egps.pps_time;

                        if diff > 5_000 {
                            debug!("egps time is old, not using.");
                        } else {
                            if let Some(dt) = NaiveDateTime::from_timestamp_millis(egps.time + diff)
                            {
                                state.rtc.set_datetime(&dt).ok();
                            } else {
                                error!(
                                    "Could not construct datetime from: {}, diff: {}",
                                    egps.time, diff
                                );
                            }

                            self.state = LocationState::Retrieved(egps.time);
                        }
                    }
                    _ => (),
                }
            }
        });
    }

    /// Get latest time and position.
    ///
    /// > NOTE: This function is called very frequently and should not communicate with the
    /// Notecard in a non-debounced way.
    pub fn check_retrieve<T: Read + Write, D: DateTimeAccess>(
        &mut self,
        state: &Mutex<RefCell<Option<SharedState<D>>>>,
        delay: &mut impl DelayMs<u16>,
        note: &mut note::Notecarrier<T>,
    ) -> Result<(), notecard::NoteError> {
        use notecard::card::res::{Location, Time};
        use LocationState::*;

        const LOCATION_DIFF: i64 = 1 * 60_000; // [ms]: 1 minute

        let now = state.now().unwrap_or(FUTURE).and_utc().timestamp_millis();

        match self.state {
            Retrieved(t) | Trying(t) if (now - t) > LOCATION_DIFF => {
                let gps = note.card().location(delay)?.wait(delay)?;
                let tm = note.card().time(delay)?.wait(delay);

                info!("Location: {:?}, Time: {:?}", gps, tm);

                if let Ok(Time {
                    time: Some(time), ..
                }) = tm
                {
                    info!("Got time, setting RTC.");
                    let dt = NaiveDateTime::from_timestamp_opt(time as i64, 0)
                        .ok_or_else(|| notecard::NoteError::NotecardErr("Bad time".into()))?;

                    free(|cs| {
                        let mut state = state.borrow(cs).borrow_mut();
                        let state: &mut _ = state.deref_mut().as_mut().unwrap();

                        state.rtc.set_datetime(&dt).ok();
                    });
                }

                if let Location {
                    lat: Some(lat),
                    lon: Some(lon),
                    time: Some(position_time),
                    ..
                } = gps
                {
                    info!("Got location, setting position.");

                    self.lat = lat;
                    self.lon = lon;
                    self.position_time = position_time;

                    free(|cs| {
                        let mut state = state.borrow(cs).borrow_mut();
                        let state: &mut _ = state.deref_mut().as_mut().unwrap();

                        state.position_time = position_time;
                        state.lat = lat;
                        state.lon = lon;
                    });
                }

                if let (
                    Ok(Time {
                        time: Some(time), ..
                    }),
                    Location { lat: Some(_), .. },
                ) = (tm, gps)
                {
                    info!("Both time and location retrieved.");
                    self.state = Retrieved((time * 1000) as i64);
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
    pub queue: heapless::spsc::Producer<'static, ImuAxlPacketT, IMUQ_SZ>,
    waves: waves::Waves<I>,
    last_read: i64,
}

impl<E: Debug + defmt::Format, I: Write<Error = E> + WriteRead<Error = E>> Imu<E, I> {
    pub fn new(
        waves: waves::Waves<I>,
        queue: heapless::spsc::Producer<'static, ImuAxlPacketT, IMUQ_SZ>,
    ) -> Imu<E, I> {
        Imu {
            queue,
            waves,
            last_read: 0,
        }
    }

    /// Read samples and check for full buffers. Return number of sample pairs consumed from IMU.
    pub fn check_retrieve(
        &mut self,
        now: i64,
        position_time: u32,
        lon: f64,
        lat: f64,
    ) -> Result<u32, waves::ImuError<E>> {
        trace!("Polling IMU.. (now: {})", now,);

        let mut samples = self.waves.read_and_filter()?;

        if self.waves.is_full() {
            trace!("waves buffer is full, pushing to queue..");
            let pck = self.waves.take_buf(now, position_time, lon, lat)?;

            #[cfg(not(feature = "storage"))]
            let pck = pck.0;

            trace!("collect remaining samples, to avoid overrun.");
            samples += self.waves.read_and_filter()?;

            self.queue
                .enqueue(pck)
                .inspect_err(|_| {
                    error!("queue is full, discarding data.");

                    log::log("Queue is full: discarding package.");
                })
                .ok();
        }

        if samples == 0 {
            let elapsed = now - self.last_read; // ms
                                                // will be a large jump when getting time.
            if elapsed > 3000 && elapsed < 100_0000 {
                error!("Too few samples, IMU may be stuck: {}", elapsed);
                self.last_read = now;
                return Err(waves::ImuError::TooFewSamples(elapsed));
            }
        } else {
            self.last_read = now;
        }

        Ok(samples)
    }

    pub fn reset(
        &mut self,
        now: i64,
        position_time: u32,
        lon: f64,
        lat: f64,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<(), waves::ImuError<E>> {
        self.waves.reset(delay)?;
        self.waves.take_buf(now, position_time, lon, lat)?; // buf is empty, this sets time and offset.
        self.waves.enable_fifo(delay)?;
        self.last_read = now; // prevent TooFewSamples to be triggered.

        Ok(())
    }
}

#[cfg(feature = "storage")]
pub struct StorageManager<Spi: Transfer<u8> + DefaultWrite<u8>, CS: OutputPin, DL: DelayUs<u8>>
where
    <Spi as Transfer<u8>>::Error: Debug,
    <Spi as DefaultWrite<u8>>::Error: Debug,
{
    storage: Storage<Spi, CS, DL>,
    pub storage_queue: heapless::spsc::Consumer<'static, AxlPacketT, STORAGEQ_SZ>,
    pub note_queue: heapless::spsc::Producer<'static, AxlPacket, NOTEQ_SZ>,
}

#[cfg(feature = "storage")]
impl<Spi: Transfer<u8> + DefaultWrite<u8>, CS: OutputPin, DL: DelayUs<u8>>
    StorageManager<Spi, CS, DL>
where
    <Spi as Transfer<u8>>::Error: Debug,
    <Spi as DefaultWrite<u8>>::Error: Debug,
{
    pub fn new(
        storage: Storage<Spi, CS, DL>,
        storage_queue: heapless::spsc::Consumer<'static, AxlPacketT, STORAGEQ_SZ>,
        note_queue: heapless::spsc::Producer<'static, AxlPacket, NOTEQ_SZ>,
    ) -> StorageManager<Spi, CS, DL> {
        StorageManager {
            storage,
            storage_queue,
            note_queue,
        }
    }

    /// Drain data queue from IMU to SD card and queue the processed data for the notecard.
    ///
    /// > NOTE: This function is called very frequently and should not communicate with the Notecard.
    pub fn drain_queue<I2C: Read + Write>(
        &mut self,
        _note: &mut note::Notecarrier<I2C>,
        _delay: &mut impl DelayMs<u16>,
    ) -> Result<Option<u32>, storage::StorageErr> {
        let mut e: Result<Option<u32>, storage::StorageErr> = Ok(None);

        defmt::trace!(
            "Draining storage queue: {} (note queue: {})",
            self.storage_queue.len(),
            self.note_queue.len(),
        );
        if let Some(mut pck) = self.storage_queue.dequeue() {
            defmt::info!(
                "Storing package: {:?} (sz queue length: {})",
                pck.0,
                self.storage_queue.len()
            );
            e = self
                .storage
                .store(&mut pck)
                .inspect_err(|err| {
                    defmt::error!("Failed to save package: {}", err);
                })
                .map(|id| Some(id));

            self.note_queue
                .enqueue(pck.0)
                .inspect_err(|pck| {
                    defmt::error!("queue is full, discarding data: {}", pck.data.len());
                })
                .ok();
        }

        e
    }

    /// XXX: Currently disabled.
    pub fn queue_requested_packages<I2C: Read + Write>(
        &mut self,
        note: &mut note::Notecarrier<I2C>,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<(), storage::StorageErr> {
        // Send additional requested packages from SD-card.
        if let Some(next_id) = self.storage.next_id() {
            if let Ok((
                Some(note::StorageIdInfo { sent_id }),
                Some(note::RequestData {
                    request_start: Some(request_start),
                    request_end: Some(request_end),
                }),
            )) = note.read_storage_info(delay)
            {
                let sent_id = sent_id.unwrap_or(request_start);
                let request_end = request_end.min(next_id.saturating_sub(1));

                if sent_id < request_end {
                    defmt::info!("Request, sending range: {} -> {}", sent_id, request_end);
                    for id in (sent_id..=request_end).take(100) {
                        let pck = self.storage.get(id);

                        defmt::debug!("Sending stored package: {:?}", pck);

                        match pck {
                            Ok(pck) => {
                                match self.note_queue.enqueue(pck) {
                                    Ok(_) => {
                                        // Update range of sent packages.
                                        note.write_storage_info(
                                            delay,
                                            Some(id),
                                            if id >= request_end { true } else { false },
                                        )
                                        .inspect_err(|e| {
                                            defmt::error!("Failed to set storageinfo: {:?}", e)
                                        })
                                        .ok();
                                    }
                                    Err(_) => {
                                        defmt::trace!(
                                            "Notecard queue is full, not adding more packages."
                                        );
                                        break;
                                    } // queue is full.
                                }
                            }
                            Err(storage::StorageErr::GenericSdMmmcErr(
                                embedded_sdmmc::Error::FileNotFound,
                            )) => {
                                let new_id = ((id / storage::COLLECTION_SIZE) + 1)
                                    * storage::COLLECTION_SIZE;

                                defmt::debug!(
                                    "File does not exist, advancing range by full collection: {} -> {}.",
                                    id, new_id
                                );

                                note.write_storage_info(
                                    delay,
                                    Some(new_id),
                                    if new_id >= request_end { true } else { false },
                                )
                                .inspect_err(|e| {
                                    defmt::error!("Failed to set storageinfo: {:?}", e)
                                })
                                .ok();

                                break;
                            }
                            Err(e) => {
                                defmt::error!(
                                    "Failed to read from SD-card: {:?}, clearing request.",
                                    e
                                );
                                note.write_storage_info(delay, None, true)
                                    .inspect_err(|e| {
                                        defmt::error!("Failed to set storageinfo: {:?}", e)
                                    })
                                    .ok();
                                return Err(e);
                            }
                        }
                    }
                } else {
                    // Request done, clearing.
                    defmt::info!("Request complete, deleting request.");
                    note.write_storage_info(delay, None, true)
                        .inspect_err(|e| defmt::error!("Failed to set storageinfo: {:?}", e))
                        .ok();
                }
            }
        }

        Ok(())
    }
}
