//! Protocol for transmitting: https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/
//! API: https://dev.blues.io/reference/notecard-api/introduction/
//!

#[allow(unused_imports)]
use defmt::{error, warn, info, debug, trace};
use embedded_hal::blocking::i2c::{Write, Read, SevenBitAddress};

mod card;

#[derive(defmt::Format)]
pub enum NoteState {
    Handshake,
    Request,
    Poll,
    Response,
}

#[derive(defmt::Format)]
pub enum NoteError {
    I2cWriteError,
    I2cReadError,
}

pub struct Note<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    i2c: IOM,
    addr: u8,
    state: NoteState,
}

impl<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Note<IOM> {
    pub fn new(i2c: IOM) -> Note<IOM> {
        Note {
            i2c,
            addr: 0x17,
            state: NoteState::Handshake,
        }
    }

    /// Check if notecarrier is connected and responding.
    pub fn ping(&mut self) -> bool {
        self.i2c.write(self.addr, &[]).is_ok()
    }

    /// Query the notecard for available bytes.
    fn data_query(&mut self) -> Result<usize, NoteError> {
        self.i2c.write(self.addr, &[]).map_err(|_| NoteError::I2cWriteError)?;

        Ok(0)
    }

    fn handshake(&mut self) -> Result<(), NoteError> {
        unimplemented!()
    }

    // pub fn reset(&mut self) {
    // }
    //
    // pub fn time(&mut self) -> Result<

    /// [card Requests](https://dev.blues.io/reference/notecard-api/card-requests/#card-location)
    pub fn card(&mut self) -> card::Card<IOM> {
        card::Card::from(self)
    }
}


// use serde::Deserialize;
// #[derive(Deserialize)]
// pub struct Status {
//     status: heapless::String<10>,
//     usb: bool,
//     storage: usize,
//     time: u64,
//     connected: bool,
// }

// pub fn status() -> Result<Status, ()> {
//     serde_json_core::from_str(
//         r#"{
//     "status":    "{normal}",
//     "usb":       true,
//     "storage":   8,
//     "time":      1599684765,
//     "connected": "true"
//     }"#,
//     )
//     .map_err(|_| ())
//     .map(|(a, _)| a)
// }

#[cfg(test)]
mod tests {}
