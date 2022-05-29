//! Stores data-packages to the SD-card.
//!
//! Every data-package is stored to the SD-card and queued for the Notecard. It should also be
//! possible to request a range of old packages.
//!
//! The maximum number of files in a FAT32 directory is 65536. If a data package has ID
//! `1234567` it is put in the file: `123.X` where `X` is the version of the storage format
//! starting with 1. The packages are serialized using the `postcard` format and separated with
//! `COBS`es. The collection file is the full ID stripped of the last 4 digits. Each collection
//! file holds 10.000 packages.
//!
//! At 52 Hz and 1024 length data-package, there is 4389 packages per day. Each collection will last about 2 days. See tests for more details.

use core::fmt::Write as _;
use embedded_sdmmc::{
    Controller, Error as GenericSdMmcError, Mode, SdMmcError, SdMmcSpi, VolumeIdx,
};
use heapless::{String, Vec};

use ambiq_hal::gpio::pin::{Mode as SpiMode, P35 as CS};
use ambiq_hal::spi::Spi0 as Spi;

use crate::axl::{AxlPacket, AXL_POSTCARD_SZ};

mod clock;
mod handles;

use clock::CountClock;
use handles::*;

pub const COLLECTION_SIZE: u32 = 10_000;
pub const STORAGE_VERSION_STR: &'static str = "1";

#[derive(Debug, defmt::Format)]
pub enum StorageErr {
    SdMmcErr(SdMmcError),
    GenericSdMmmcErr(GenericSdMmcError<SdMmcError>),
    ParseIDFailure,
    WriteIDFailure,
    WriteError,
    ReadPackageError,
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
    /// Next free ID.
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

    /// Returns the current (next free ID).
    pub fn current_id(&self) -> Option<u32> {
        self.current_id
    }

    /// Set the current ID, but do not write to card.
    pub fn set_id(&mut self, id: u32) {
        self.current_id = Some(id);
    }

    /// Deserialize and return AxlPacket.
    pub fn get(&mut self, id: u32) -> Result<AxlPacket, StorageErr> {
        defmt::debug!("Reading file: {}", id);
        let (collection, file, offset) = id_to_parts(id);

        let mut buf: Vec<u8, { AXL_POSTCARD_SZ }> = Vec::new();
        buf.resize_default(AXL_POSTCARD_SZ).unwrap();

        defmt::debug!(
            "Reading package id: {} from collection: {}, fileid: {}, offset: {}",
            id,
            collection,
            file,
            offset
        );

        {
            let block = self.sd.acquire()?;
            let mut c = Controller::new(block, CountClock);
            let mut v = c.get_volume(VolumeIdx(0))?;
            let mut root = DirHandle::open_root(&mut c, &mut v)?;
            let mut f = root.open_file(&collection, Mode::ReadOnly)?;
            f.seek_from_start(offset as u32)
                .map_err(|_| StorageErr::ReadPackageError)?;
            let sz = f.read(&mut buf)?;
            defmt::trace!("Read {} bytes.", sz);
        }

        // De-serialize
        let pck: AxlPacket =
            postcard::from_bytes_cobs(&mut buf).map_err(|_| StorageErr::ReadPackageError)?;

        Ok(pck)
    }

    /// Store a new package.
    pub fn store(&mut self, pck: &mut AxlPacket) -> Result<u32, StorageErr> {
        let id = self.current_id.unwrap();
        let (collection, fid, offset) = id_to_parts(id);

        // Package now has a storage ID.
        pck.storage_id = Some(id);
        self.current_id = Some(id + 1);
        {
            self.write_id()?;
        }

        // Serialize
        let buf: Vec<u8, { AXL_POSTCARD_SZ }> =
            postcard::to_vec_cobs(pck).map_err(|_| StorageErr::WriteError)?;

        // And write..
        defmt::debug!(
            "Writing package to card id: {}, size: {}, timestamp: {}, collection: {}, fileid: {}, offset: {}",
            id,
            buf.len(),
            pck.timestamp,
            collection,
            fid,
            offset
        );
        {
            let block = self.sd.acquire()?;
            let mut c = Controller::new(block, CountClock);
            let mut v = c.get_volume(VolumeIdx(0))?;
            let mut root = DirHandle::open_root(&mut c, &mut v)?;
            let mut f = root.open_file(&collection, Mode::ReadWriteCreateOrAppend)?;
            f.seek_from_end(0).map_err(|_| StorageErr::WriteError)?; // We should already be at the
                                                                     // end.
            f.write(&buf)?;
        }

        Ok(id)
    }
}

/// Calculate collection file, file number in collection and byte offset of start of pacakge in
/// collection file for a given ID.
pub fn id_to_parts(id: u32) -> (String<32>, u32, usize) {
    let collection = id / COLLECTION_SIZE;
    let fileid = id % COLLECTION_SIZE;
    let offset = fileid as usize * AXL_POSTCARD_SZ;

    let mut collection = String::from(collection);
    collection.push_str(".").unwrap();
    collection.push_str(STORAGE_VERSION_STR).unwrap();

    (collection, fileid, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use half::f16;
    use crate::axl::AXL_SZ;

    #[test]
    fn test_id_to_parts() {
        let (c, file, o) = id_to_parts(0);
        assert_eq!(c, "0.1");
        assert_eq!(file, 0);
        assert_eq!(o, 0);

        let (c, file, o) = id_to_parts(1234567);
        assert_eq!(c, "123.1");
        assert_eq!(file, 4567);
        assert_eq!(o, 4567 * AXL_POSTCARD_SZ);
    }

    #[test]
    fn test_fat32_limits() {
        let pcks_per_day = 52 * 60 * 60 * 24 / 1024;
        let collection_file_size = COLLECTION_SIZE * AXL_POSTCARD_SZ as u32;

        // max file size.
        assert!((collection_file_size as u64) < { 4 * 1024 * 1024 * 1024 });

        println!("collection file size: {} b", collection_file_size);
        println!("pcks per day: {}", pcks_per_day);

        let collections_per_day = pcks_per_day as f32 / COLLECTION_SIZE as f32;
        let collections_per_year = (pcks_per_day * 365) as f32 / COLLECTION_SIZE as f32;
        println!("Collections per day: {}", collections_per_day);
        println!("Collections per year: {}", collections_per_year);

        // max files in directory (should last at least a year)
        assert!(collections_per_year < 65536 as f32);
    }

    #[test]
    fn read_collection() {
        let mut c = std::fs::read("tests/data/0.1").unwrap();
        assert_eq!(c.len(), AXL_POSTCARD_SZ * 2);

        let buf = c.as_mut_slice();

        let p0: AxlPacket = postcard::from_bytes_cobs(&mut buf[..AXL_POSTCARD_SZ]).unwrap();
        let p1: AxlPacket = postcard::from_bytes_cobs(&mut buf[AXL_POSTCARD_SZ..]).unwrap();

        assert_eq!(p0.storage_id, Some(0));
        assert_eq!(p1.storage_id, Some(1));

        let p0_truth = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: Some(0),
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };
        let p1_truth = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: Some(1),
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        assert_eq!(p0_truth, p0);
        assert_eq!(p1_truth, p1);
    }
}
