//! Stores data-packages to the SD-card.
//!
//! Every data-package is stored to the SD-card and queued for the Notecard. It should also be
//! possible to request a range of old packages.
//!
//! The maximum number of files in a FAT32 directory is 65536. If a data package has ID
//! `1234567` it is put in the directory: `123` and named `4567.axl`. The directory is the full
//! ID stripped of the last 4 digits, and the file name is the last 4 digits. At 52 Hz and 1024
//! length data-package, this should amount to 4389 files per day. Each directory will last a bit
//! longer than two days.
//!
//! The data package needs information about:
//!     * buoy ID / dev
//!

use core::fmt::Write as _;
use embedded_sdmmc::{
    Controller, Error as GenericSdMmcError, Mode, SdMmcError, SdMmcSpi, VolumeIdx,
};
use heapless::{String, Vec};

use ambiq_hal::gpio::pin::{Mode as SpiMode, P35 as CS};
use ambiq_hal::spi::Spi0 as Spi;

use crate::axl::{AxlPacket, AXL_OUTN};

mod clock;
mod handles;

use clock::CountClock;
use handles::*;

#[derive(Debug, defmt::Format)]
pub enum StorageErr {
    SdMmcErr(SdMmcError),
    GenericSdMmmcErr(GenericSdMmcError<SdMmcError>),
    ParseIDFailure,
    WriteIDFailure,
    WriteError,
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

const ID_FILE: &'static str = "sfy.id";
const ID_DIGITS: usize = 10; // 8 is sufficient for 20 years.

pub struct Storage {
    sd: SdMmcSpi<Spi, CS<{ SpiMode::Output }>>,
    /// Last written ID.
    current_id: Option<u32>,
}

impl Storage {
    pub fn open(spi: Spi, cs: CS<{ SpiMode::Output }>) -> Result<Storage, StorageErr> {
        defmt::info!("Opening SD card..");

        let mut sd = SdMmcSpi::new(spi, cs);
        // TODO: Re-clock SPI

        defmt::info!("Initialize SD-card..");
        {
            let block = sd.acquire()?;
            let sz = block.card_size_bytes()? / 1024_u64.pow(2);
            defmt::info!("SD card size: {} mb", sz);
        }

        let mut storage = Storage {
            sd,
            current_id: None,
        };

        storage.current_id = Some(storage.read_id()?);

        Ok(storage)
    }

    /// Read the current ID.
    pub fn read_id(&mut self) -> Result<u32, StorageErr> {
        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, CountClock);
        let mut v = c.get_volume(VolumeIdx(0))?;

        let mut root = DirHandle::open_root(&mut c, &mut v)?;
        let idf = root.open_file(ID_FILE, Mode::ReadOnly);

        let id = match idf {
            Ok(mut idf) => {
                let mut buf = [0u8; ID_DIGITS];
                let sz = idf.read(&mut buf)?;
                let buf = &buf[..sz];

                defmt::trace!(
                    "ID file contents: {:?}",
                    defmt::Debug2Format(&core::str::from_utf8(&buf))
                );

                // TODO: Handle corrupted ID file.
                let buf = core::str::from_utf8(&buf).map_err(|_| StorageErr::ParseIDFailure)?;
                let id = u32::from_str_radix(&buf, 10).map_err(|_| StorageErr::ParseIDFailure)?;

                Ok(id)
            }
            Err(GenericSdMmcError::FileNotFound) => {
                defmt::debug!("No ID file, returing zero.");
                Ok(0)
            }
            Err(e) => Err(e),
        }?;

        Ok(id)
    }

    /// Writes the current ID to SD-card.
    pub fn write_id(&mut self) -> Result<(), StorageErr> {
        let id = self.current_id.unwrap();
        defmt::debug!("Writing id: {}", id);
        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, CountClock);
        let mut v = c.get_volume(VolumeIdx(0))?;

        let mut root = DirHandle::open_root(&mut c, &mut v)?;
        let mut idf = root.open_file(ID_FILE, Mode::ReadWriteCreateOrTruncate)?;
        let mut buf = String::<ID_DIGITS>::new();
        write!(&mut buf, "{}", id).map_err(|e| {
            defmt::error!("Format error: {:?}", defmt::Debug2Format(&e));
            StorageErr::WriteIDFailure
        })?;
        defmt::trace!("Writing bytes: {:?}", &buf);
        idf.write(buf.as_bytes())?;

        Ok(())
    }

    /// Takes IMU queue and stores items.
    pub fn drain_queue(&mut self) -> Result<(), ()> {
        todo!()
    }

    /// Returns the current (next free ID).
    pub fn current_id(&self) -> Option<u32> {
        self.current_id
    }

    /// Set the current ID, but do not write to card.
    pub fn set_id(&mut self, id: u32) {
        self.current_id = Some(id);
    }

    /// Deserialize and return AxlPacket (without modifying sent status).
    pub fn get(&self, id: u32) -> Result<AxlPacket, StorageErr> {
        unimplemented!()
    }

    /// Mark package as sent over notecard.
    pub fn mark_sent(&mut self, id: u32) -> Result<(), StorageErr> {
        unimplemented!()
    }

    /// Check if package has been sent over notecard.
    pub fn is_sent(&mut self, id: u32) -> Result<bool, StorageErr> {
        unimplemented!()
    }

    /// Store a new package and mark it as unsent.
    pub fn store(&mut self, pck: &mut AxlPacket) -> Result<u32, StorageErr> {
        let id = self.current_id.unwrap();
        let (dir, file) = id_to_parts(id).map_err(|_| StorageErr::WriteIDFailure)?;

        // Package now has a storage ID.
        pck.storage_id = Some(id);
        self.current_id = Some(id + 1);
        {
            self.write_id()?;
        }

        // Serialize
        let buf: Vec<u8, { AXL_OUTN }> =
            postcard::to_vec(pck).map_err(|_| StorageErr::WriteError)?;

        defmt::debug!(
            "Writing package to card, id: {}, size: {}, timestamp: {}",
            id,
            buf.len(),
            pck.timestamp
        );
        {
            let block = self.sd.acquire()?;
            let mut c = Controller::new(block, CountClock);
            let mut v = c.get_volume(VolumeIdx(0))?;
            let mut root = DirHandle::open_root(&mut c, &mut v)?;
            let mut d = root.open_dir(&dir)?;
            let mut f = d.open_file(&file, Mode::ReadWriteCreate)?;
            f.write(&buf)?;
        }

        // Store unsent-status

        Ok(id)
    }
}

pub fn id_to_parts(id: u32) -> Result<(heapless::String<10>, heapless::String<8>), ()> {
    let dir = id / 10000;
    let file = id % 10000;

    let dir = heapless::String::from(dir);
    let mut file = heapless::String::from(file);
    file.push_str(".axl")?;

    Ok((dir, file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_to_parts() {
        let (dir, file) = id_to_parts(0).unwrap();
        assert_eq!(dir, "0");
        assert_eq!(file, "0.axl");

        let (dir, file) = id_to_parts(1234567).unwrap();
        assert_eq!(dir, "123");
        assert_eq!(file, "4567.axl");
    }
}
