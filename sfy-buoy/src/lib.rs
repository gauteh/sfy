#![feature(test)]
#![feature(inline_const)]
#![feature(result_option_inspect)]
#![feature(try_blocks)]
#![feature(portable_simd)]
#![feature(array_chunks)]
#![cfg_attr(not(test), no_std)]

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
use embedded_hal::{blocking::spi::Transfer, digital::v2::OutputPin};

use rtcc::DateTimeAccess;

pub mod axl;
#[cfg(feature = "fir")]
pub mod fir;
pub mod log;
pub mod note;
#[cfg(feature = "storage")]
pub mod storage;
pub mod waves;

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
#[cfg(feature = "raw")]
pub const NOTEQ_SZ: usize = 6;

#[cfg(not(feature = "raw"))]
pub const STORAGEQ_SZ: usize = 12;
#[cfg(all(not(feature = "raw"), feature = "storage"))]
pub const NOTEQ_SZ: usize = 12;

#[cfg(feature = "storage")]
pub const IMUQ_SZ: usize = STORAGEQ_SZ;

#[cfg(all(not(feature = "raw"), not(feature = "storage")))]
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

pub struct SharedState<D: DateTimeAccess> {
    pub rtc: D,
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
}

pub trait State {
    fn now(&self) -> NaiveDateTime;

    /// Returns now, posistion_time, lat, lon.
    fn get(&self) -> (NaiveDateTime, u32, f64, f64);
}

impl<D: DateTimeAccess> SharedState<D> {
    fn now(&mut self) -> NaiveDateTime {
        self.rtc
            .datetime()
            .unwrap_or(NaiveDateTime::from_timestamp_opt(0, 0).unwrap())
    }

    fn get(&mut self) -> (NaiveDateTime, u32, f64, f64) {
        (
            self.rtc
                .datetime()
                .unwrap_or(NaiveDateTime::from_timestamp_opt(0, 0).unwrap()),
            self.position_time,
            self.lat,
            self.lon,
        )
    }
}

impl<D: DateTimeAccess> State for Mutex<RefCell<Option<SharedState<D>>>> {
    fn now(&self) -> NaiveDateTime {
        free(|cs| {
            let mut state = self.borrow(cs).borrow_mut();
            let state: &mut _ = state.deref_mut().as_mut().unwrap();

            state.now()
        })
    }

    fn get(&self) -> (NaiveDateTime, u32, f64, f64) {
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
    pub position_time: u32,
    pub time: u32,

    pub state: LocationState,
}

impl Location {
    pub fn new() -> Location {
        Location {
            lat: 0.0,
            lon: 0.0,
            position_time: 0,
            time: 0,
            state: LocationState::Trying(-999),
        }
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

        let now = state.now().timestamp_millis();

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
                    self.time = time;

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

                if let (Ok(Time { time: Some(_), .. }), Location { lat: Some(_), .. }) = (tm, gps) {
                    info!("Both time and location retrieved.");
                    free(|cs| {
                        let mut state = state.borrow(cs).borrow_mut();
                        let state: &mut _ = state.deref_mut().as_mut().unwrap();
                        self.state = Retrieved(state.now().timestamp_millis());
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
pub struct StorageManager<Spi: Transfer<u8>, CS: OutputPin>
where
    <Spi as Transfer<u8>>::Error: Debug,
{
    storage: Storage<Spi, CS>,
    pub storage_queue: heapless::spsc::Consumer<'static, AxlPacketT, STORAGEQ_SZ>,
    pub note_queue: heapless::spsc::Producer<'static, AxlPacket, NOTEQ_SZ>,
}

#[cfg(feature = "storage")]
impl<Spi: Transfer<u8>, CS: OutputPin> StorageManager<Spi, CS>
where
    <Spi as Transfer<u8>>::Error: Debug,
{
    pub fn new(
        storage: Storage<Spi, CS>,
        storage_queue: heapless::spsc::Consumer<'static, AxlPacketT, STORAGEQ_SZ>,
        note_queue: heapless::spsc::Producer<'static, AxlPacket, NOTEQ_SZ>,
    ) -> StorageManager<Spi, CS> {
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
