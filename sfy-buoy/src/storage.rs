//! Storage works by inserting itself between the IMU queue and the notecarrier. The IMU keeps
//! pushing and either the notecarrier or the sd-storage needs to keep up. The notecarrier keeps
//! pushing to the notecard as long as there is new data in the queue.
//!
//! With the SD-storage we can also support a longer delay between transmissions, since the
//! notecard is no longer required to keep up with the limited RAM.
//!
//! It would be great if we could control somewhat what we want from the buoy using the notecarrier.
//! That requires that we keep up-to-date some statistics/status on the notecarrier, and that the
//! notecarrier can communicate to the storage what has already been sent. This can probably go
//! through `main` to avoid too much interdependency.

use embedded_sdmmc::{SdMmcSpi, TimeSource, Timestamp};

use crate::axl::AxlPacket;

pub enum StorageErr {
    SdMmcErr,
}

pub struct Storage {
    // sd: SdMmcSpi,
    /// Last written ID.
    current_id: u32,
}

impl Storage {
    // pub fn open() -> Storage {
    //     // Get last id (or create file with 0, or scan)
    // }

    /// Takes IMU queue and stores items.
    pub fn drain_queue(&mut self) -> Result<(), ()> {
        todo!()
    }

    pub fn current_id(&self) -> u32 {
        self.current_id
    }

    // Deserialize and return AxlPacket.
    pub fn get(&self, id: u32) -> Result<AxlPacket, StorageErr> {
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
