//! Stores data-packages to the SD-card.
//!
//! Every data-package is stored to the SD-card and queued for the Notecard. It should also be
//! possible to request a range of old packages.

use chrono::{Datelike, NaiveDateTime, Timelike};
use core::sync::atomic::Ordering;
use embedded_sdmmc::{
    BlockDevice, Controller, Error as GenericSdMmcError, File, Mode, SdMmcError, SdMmcSpi,
    TimeSource, Timestamp, Volume, VolumeIdx,
};

use ambiq_hal::gpio::pin::{Mode as SpiMode, P35 as CS};
use ambiq_hal::spi::Spi0 as Spi;

use crate::axl::AxlPacket;
use crate::COUNT;

pub enum StorageErr {
    SdMmcErr(SdMmcError),
    GenericSdMmmcErr(GenericSdMmcError<SdMmcError>),
}

impl From<SdMmcError> for StorageErr {
    fn from(e: SdMmcError) -> Self {
        StorageErr::SdMmcErr(e)
    }
}

impl From<embedded_sdmmc::Error<SdMmcError>> for StorageErr {
    fn from(e: embedded_sdmmc::Error<SdMmcError>) -> Self {
        StorageErr::GenericSdMmmcErr(e)
    }
}

/// A FileHandle that requires exclusive access to the Controller and Volume. Not a big loss
/// for us, but it allows use to make _more_ sure the file-handle is closed, and not
/// accidently left open because of early-returns `(?)`.
struct FileHandle<'c, 'v, D: BlockDevice, T: TimeSource> {
    ctrl: &'c mut Controller<D, T>,
    vol: &'v mut Volume,
    file: Option<File>,
}

impl<'c, 'v, D: BlockDevice, T: TimeSource> FileHandle<'c, 'v, D, T> {
    pub fn new(ctrl: &'c mut Controller<D, T>, vol: &'v mut Volume, file: File) -> Self {
        FileHandle {
            ctrl,
            vol,
            file: Some(file),
        }
    }

    // pub fn open_in_dir(

    pub fn close(self) {
        drop(self);
    }

    pub fn read(&mut self, buf: &mut [u8]) {
        let file = self.file.as_mut().unwrap();
        self.ctrl.read(&self.vol, file, buf);
    }

    pub fn write(&mut self, buf: &[u8]) {
        let file = self.file.as_mut().unwrap();
        self.ctrl.write(&mut self.vol, file, buf);
    }
}

impl<D: BlockDevice, T: TimeSource> Drop for FileHandle<'_, '_, D, T> {
    fn drop(&mut self) {
        let file = self.file.take();

        if let Some(file) = file {
            self.ctrl.close_file(&self.vol, file);
        }
    }
}

const ID_FILE: &'static str = "sfy.id";

pub struct Storage {
    sd: SdMmcSpi<Spi, CS<{ SpiMode::Output }>>,
    /// Last written ID.
    current_id: u32,
}

impl Storage {
    pub fn open(spi: Spi, cs: CS<{ SpiMode::Output }>) -> Result<Storage, StorageErr> {
        // Get last id (or create file with 0, verify it's free, or scan)
        defmt::info!("Opening SD card..");

        let mut sd = SdMmcSpi::new(spi, cs);
        // TODO: Re-clock SPI

        defmt::info!("Initialize SD-card..");
        {
            let block = sd.acquire()?;
            let sz = block.card_size_bytes()? / 1024_u64.pow(2);
            defmt::info!("SD card size: {} mb", sz);

            let mut c = Controller::new(block, CountClock);
            let mut v = c.get_volume(VolumeIdx(0))?;

            let root = c.open_root_dir(&v)?;

            let idf = c.open_file_in_dir(&mut v, &root, ID_FILE, Mode::ReadOnly);

            match idf {
                Ok(f) => {
                    let mut buf = [0u8; 128];
                    let mut fh = FileHandle::new(&mut c, &mut v, f);
                    fh.read(&mut buf);

                    Ok(0)
                }
                Err(GenericSdMmcError::FileNotFound) => Ok(0),
                Err(e) => Err(e),
            }?;
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
