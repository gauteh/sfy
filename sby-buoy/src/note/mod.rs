//! Protocol for transmitting: https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/
//! API: https://dev.blues.io/reference/notecard-api/introduction/
//!
//! Each
use crate::hal;

pub enum NoteState {
    PreHandshake,
    Ready,
}

pub struct Note {
    i2c: hal::i2c::I2c,
    addr: u8,
    state: NoteState,
}

impl Note {
    pub fn new(i2c: hal::i2c::I2c) -> Note {
        Note {
            i2c,
            addr: 0x17,
            state: NoteState::PreHandshake,
        }
    }

    pub fn reset(&mut self) {
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
