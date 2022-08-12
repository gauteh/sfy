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

use core::fmt::Debug;
use core::ops::DerefMut;
use cortex_m::interrupt::free;
use embedded_hal::{blocking::spi::Transfer, digital::v2::OutputPin};
use embedded_sdmmc::{
    Controller, Error as GenericSdMmcError, Mode, SdMmcError, SdMmcSpi, VolumeIdx,
};
use heapless::{String, Vec};

use crate::axl::{AxlPacket, AXL_POSTCARD_SZ};

pub mod clock;
mod handles;

use clock::CountClock;
use handles::*;

/// Writing to a file seems to take longer time when it has more packages, this can cause timeouts
/// in the interrupt that drains the IMU FIFO. See <https://github.com/gauteh/sfy/issues/77>.
pub const COLLECTION_SIZE: u32 = 100;
pub const STORAGE_VERSION_STR: &'static str = "2";
pub const STORAGE_VERSION: u32 = 2;

#[derive(Debug, defmt::Format)]
pub enum StorageErr {
    SdMmcErr(SdMmcError),
    GenericSdMmmcErr(GenericSdMmcError<SdMmcError>),
    ParseIDFailure,
    WriteIDFailure,
    WriteError,
    ReadPackageError,
    SerializationError,
    DiskFull,
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

pub struct Storage<Spi: Transfer<u8>, CS: OutputPin>
where
    <Spi as Transfer<u8>>::Error: Debug,
{
    sd: SdMmcSpi<Spi, CS>,
    /// Next free ID.
    next_id: Option<u32>,
    clock: CountClock,
}

impl<Spi: Transfer<u8>, CS: OutputPin> Storage<Spi, CS>
where
    <Spi as Transfer<u8>>::Error: Debug,
{
    pub fn open(
        spi: Spi,
        cs: CS,
        clock: CountClock,
        reclock_cb: fn(&mut Spi) -> (),
    ) -> Result<Storage<Spi, CS>, StorageErr> {
        defmt::info!("Opening SD card..");

        let mut sd = SdMmcSpi::new(spi, cs);
        defmt::info!("Initialize SD-card (re-clock SPI)..");
        {
            let mut block = sd.acquire()?;
            reclock_cb(block.spi().deref_mut());
            let sz = block.card_size_bytes()? / 1024_u64.pow(2);
            defmt::info!("SD card size: {} mb", sz);
        }

        let mut storage = Storage {
            sd,
            next_id: None,
            clock,
        };

        let c = storage.find_first_free_collection(None)?;
        defmt::info!("Starting new collection: {}", c);
        storage.set_id(c * COLLECTION_SIZE);

        Ok(storage)
    }

    /// Find the first free collection. Every time the buoy starts up a new collection will be used
    /// to prevent offset mismatch between file id. The ID will be set to the first entry in that
    /// collection.
    pub fn find_first_free_collection(&mut self, start: Option<u32>) -> Result<u32, StorageErr> {
        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, &self.clock);
        let mut v = c.get_volume(VolumeIdx(0))?;

        let mut root = DirHandle::open_root(&mut c, &mut v)?;

        for c in start.unwrap_or(0)..65536u32 {
            let f = collection_fname(c);
            defmt::debug!("Searching for free collection, testing: {}", f);
            match root.find_directory_entry(&f) {
                Ok(_) => continue,
                Err(GenericSdMmcError::FileNotFound) => return Ok(c),
                Err(e) => return Err(e.into()),
            }
        }

        Err(StorageErr::DiskFull)
    }

    /// Returns the next free ID.
    pub fn next_id(&self) -> Option<u32> {
        self.next_id
    }

    /// Set the current ID, but do not write to card.
    pub fn set_id(&mut self, id: u32) {
        self.next_id = Some(id);
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

        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, &self.clock);
        let mut v = c.get_volume(VolumeIdx(0))?;
        let mut root = DirHandle::open_root(&mut c, &mut v)?;
        let mut f = root.open_file(&collection, Mode::ReadOnly)?;

        if f.length() < (offset + AXL_POSTCARD_SZ) as u32 {
            defmt::debug!("Collection is not long enough, no such file in it.");
            return Err(GenericSdMmcError::FileNotFound.into());
        }

        f.seek_from_start(offset as u32)
            .map_err(|_| StorageErr::ReadPackageError)?;

        let sz = free(|_| f.read(&mut buf))?;
        defmt::trace!("Read {} bytes.", sz);

        // De-serialize
        let pck: AxlPacket =
            postcard::from_bytes_cobs(&mut buf).map_err(|_| StorageErr::ReadPackageError)?;

        Ok(pck)
    }

    /// Get the next free ID (and advance to new collection if necessary).
    pub fn advance_id(&mut self) -> Result<u32, StorageErr> {
        let id = self.next_id.unwrap();

        let mut next_id = id + 1;

        // Check that the next collection is free, if rolling over.
        if next_id % COLLECTION_SIZE == 0 {
            let c = next_id / COLLECTION_SIZE;
            let nc = self.find_first_free_collection(Some(c))?;

            if nc > c {
                next_id = nc * COLLECTION_SIZE;
            }
        }

        self.set_id(next_id);

        Ok(id)
    }

    /// Store a new package.
    pub fn store(&mut self, pck: &mut AxlPacket) -> Result<u32, StorageErr> {
        // Next ID: If writing fails further down this function, and later succeeds in the same
        // collection, the file-id and offset will no longer be correctly calculated. The buoy will
        // fail to read (and skip) those messages from the SD-card if requested. However, they will
        // be able to be read from the SD-card from a computer.
        let id = self.advance_id()?;
        let (collection, fid, offset) = id_to_parts(id);

        // Package now has a storage ID.
        pck.storage_id = Some(id);
        pck.storage_version = Some(STORAGE_VERSION);

        // Serialize
        let mut buf: Vec<u8, { AXL_POSTCARD_SZ }> = postcard::to_vec_cobs(pck)
            .inspect_err(|e| defmt::error!("Serialization: {:?}", defmt::Debug2Format(e)))
            .map_err(|_| StorageErr::SerializationError)?;
        buf.resize_default(buf.capacity()).unwrap();

        // And write..
        defmt::info!(
            "Writing package to card id: {}, size: {}, timestamp: {}, collection: {}, fileid: {}, offset: {}",
            id,
            buf.len(),
            pck.timestamp,
            collection,
            fid,
            offset
        );

        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, &self.clock);
        let mut v = c.get_volume(VolumeIdx(0))?;
        let mut root = DirHandle::open_root(&mut c, &mut v)?;
        let mut f = root.open_file(&collection, Mode::ReadWriteCreateOrAppend)?;
        f.seek_from_end(0)
            .inspect_err(|e| defmt::error!("File seek error: {}", e))
            .map_err(|_| StorageErr::WriteError)?; // We should already be at the
                                                   // end.
        free(|_| f.write(&buf))?;

        Ok(id)
    }

    pub fn remove_collection(&mut self, collection: u32) -> Result<(), StorageErr> {
        defmt::info!("Removing collection: {}", collection);

        let f = collection_fname(collection);

        let block = self.sd.acquire()?;
        let mut c = Controller::new(block, &self.clock);
        let mut v = c.get_volume(VolumeIdx(0))?;
        let mut root = DirHandle::open_root(&mut c, &mut v)?;
        root.delete_file(&f)?;

        Ok(())
    }
}

pub fn collection_fname(c: u32) -> String<32> {
    let mut f: String<32> = String::from(c);
    f.push_str(".").unwrap();
    f.push_str(STORAGE_VERSION_STR).unwrap();
    f
}

/// Calculate collection file, file number in collection and byte offset of start of pacakge in
/// collection file for a given ID.
pub fn id_to_parts(id: u32) -> (String<32>, u32, usize) {
    let collection = id / COLLECTION_SIZE;
    let fileid = id % COLLECTION_SIZE;
    let offset = fileid as usize * AXL_POSTCARD_SZ;

    let collection = collection_fname(collection);

    (collection, fileid, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axl::AXL_SZ;
    use half::f16;

    #[test]
    fn test_id_to_parts() {
        let (c, file, o) = id_to_parts(0);
        assert_eq!(c, "0.2");
        assert_eq!(file, 0);
        assert_eq!(o, 0);

        let (c, file, o) = id_to_parts(1231255);
        assert_eq!(c, "12312.2");
        assert_eq!(file, 55);
        assert_eq!(o, 55 * AXL_POSTCARD_SZ);
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
        println!("Years of collections possible (FAT32 file limit): {}", 65536.0 / collections_per_year);

        // max files in directory (should last at least a year)
        assert!(collections_per_year < 65536 as f32);
    }

    #[test]
    fn read_synth_collection() {
        let mut c = std::fs::read("tests/data/0.2").unwrap();
        assert_eq!(c.len(), AXL_POSTCARD_SZ * 4);

        let buf = c.as_mut_slice();

        let p0: AxlPacket = postcard::from_bytes_cobs(&mut buf[..AXL_POSTCARD_SZ]).unwrap();
        let p1: AxlPacket =
            postcard::from_bytes_cobs(&mut buf[AXL_POSTCARD_SZ..(2 * AXL_POSTCARD_SZ)]).unwrap();
        let p2: AxlPacket =
            postcard::from_bytes_cobs(&mut buf[(AXL_POSTCARD_SZ * 2)..(AXL_POSTCARD_SZ * 3)])
                .unwrap();

        assert_eq!(p0.storage_id, Some(0));
        assert_eq!(p1.storage_id, Some(1));
        assert_eq!(p2.storage_id, Some(2));

        let p0_truth = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: Some(0),
            storage_version: Some(STORAGE_VERSION),
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
            storage_version: Some(STORAGE_VERSION),
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };
        let p2_truth = AxlPacket {
            timestamp: 1002500,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: Some(2),
            storage_version: Some(STORAGE_VERSION),
            data: (9..3081)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        assert_eq!(p0_truth, p0);
        assert_eq!(p1_truth, p1);
        assert_eq!(p2_truth, p2);
    }

    #[test]
    fn read_real_data() {
        let mut c = std::fs::read("tests/data/2.2").unwrap();
        assert_eq!(c.len(), AXL_POSTCARD_SZ * 12);

        let buf = c.as_mut_slice();

        for p in 0..12 {
            let slice = &mut buf[(AXL_POSTCARD_SZ * p)..(AXL_POSTCARD_SZ * (p + 1))];
            let pck: AxlPacket = postcard::from_bytes_cobs(slice).unwrap();
            println!("Deserialized data package: {:?}", pck);

            assert_eq!(pck.storage_id, Some(200 + p as u32));
        }
    }
}
