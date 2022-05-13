//! Stores data-packages to the SD-card.
//!
//! Every data-package is stored to the SD-card and queued for the Notecard. It should also be
//! possible to request a range of old packages.

use chrono::{Datelike, NaiveDateTime, Timelike};
use core::sync::atomic::Ordering;
use embedded_hal::spi;
use embedded_sdmmc::{SdMmcError, SdMmcSpi, TimeSource, Timestamp};

use ambiq_hal::gpio::pin::{Mode, P35 as CS};
use ambiq_hal::spi::{Freq, Spi0 as Spi};

use crate::axl::AxlPacket;
use crate::COUNT;

pub enum StorageErr {
    SdMmcErr(SdMmcError),
}

impl From<SdMmcError> for StorageErr {
    fn from(e: SdMmcError) -> Self {
        StorageErr::SdMmcErr(e)
    }
}

pub struct Storage {
    sd: SdMmcSpi<Spi, CS<{ Mode::Output }>>,
    /// Last written ID.
    current_id: u32,
}

impl Storage {
    pub fn open(spi: Spi, cs: CS<{ Mode::Output }>) -> Result<Storage, StorageErr> {
        // Get last id (or create file with 0, verify it's free, or scan)
        defmt::info!("Opening SD card..");

        let mut sd = SdMmcSpi::new(spi, cs);

        defmt::info!("Initialize SD-card..");
        {
            let block = sd.acquire()?;

            let sz = block.card_size_bytes()? / 1024_u64.pow(2);

            defmt::info!("SD card size: {} mb", sz);
        }

        Ok(Storage { sd, current_id: 0 })
    }

    /// Takes IMU queue and stores items.
    pub fn drain_queue(&mut self) -> Result<(), ()> {
        todo!()
    }

    pub fn current_id(&self) -> u32 {
        self.current_id
    }

    // Deserialize and return AxlPacket (without modifying sent status).
    pub fn get(&self, id: u32) -> Result<AxlPacket, StorageErr> {
        unimplemented!()
    }

    // Mark package as sent
    pub fn mark_sent(&mut self, id: u32) -> Result<(), StorageErr> {
        unimplemented!()
    }

    // Store a new package and mark it as unsent.
    pub fn store(&mut self, pck: AxlPacket) -> Result<u32, StorageErr> {
        // Store to id
        // Store unsent-status
        // Update current ID on disk
        // Update current ID in self
        unimplemented!()
    }
}

struct NullClock;

impl TimeSource for NullClock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// Accesses `core::COUNT` to get globally updated timestamp from RTC interrupt, which is set by
/// the GPS.
struct CountClock;

impl TimeSource for CountClock {
    fn get_timestamp(&self) -> Timestamp {
        let dt = NaiveDateTime::from_timestamp(COUNT.load(Ordering::Relaxed) as i64, 0);
        Timestamp {
            year_since_1970: (dt.year() - 1970) as u8,
            zero_indexed_month: dt.month0() as u8,
            zero_indexed_day: dt.day0() as u8,
            hours: dt.hour() as u8,
            minutes: dt.minute() as u8,
            seconds: dt.second() as u8,
        }
    }
}
