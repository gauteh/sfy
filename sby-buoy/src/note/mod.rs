//! Protocol for transmitting: https://dev.blues.io/notecard/notecard-guides/serial-over-i2c-protocol/
//! API: https://dev.blues.io/reference/notecard-api/introduction/
//!

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};
use embedded_hal::blocking::i2c::{Read, SevenBitAddress, Write};

mod card;

#[derive(defmt::Format)]
pub enum NoteState {
    Handshake,

    /// Ready to make request.
    Request,

    /// Waiting for response to become ready, value is tries made.
    Poll(usize),

    /// Reading response, value is remaining bytes.
    Response(usize),
}

#[derive(defmt::Format)]
pub enum NoteError {
    I2cWriteError,
    I2cReadError,

    RemainingData,

    /// Method called when notecarrier is in invalid state.
    WrongState,
}


pub struct Note<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    i2c: IOM,
    addr: u8,
    state: NoteState,
    buf: heapless::Vec<u8, 1024>,
}

impl<IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> Note<IOM> {
    pub fn new(i2c: IOM) -> Note<IOM> {
        Note {
            i2c,
            addr: 0x17,
            state: NoteState::Handshake,
            buf: heapless::Vec::new(),
        }
    }

    /// Check if notecarrier is connected and responding.
    ///
    /// `ping` should be allowed no matter the state.
    pub fn ping(&mut self) -> bool {
        self.i2c.write(self.addr, &[]).is_ok()
    }

    /// Query the notecard for available bytes.
    fn data_query(&mut self) -> Result<usize, NoteError> {
        // Ask for reading, but with zero bytes allocated.
        self.i2c
            .write(self.addr, &[0, 0])
            .map_err(|_| NoteError::I2cWriteError)?;

        let mut buf = [0u8; 1];

        // Read available bytes to read
        self.i2c.read(self.addr, &mut buf).map_err(|_| NoteError::I2cReadError)?;

        let available = buf[0] as usize;

        // Read bytes sent, this should be zero.
        self.i2c.read(self.addr, &mut buf).map_err(|_| NoteError::I2cReadError)?;

        if buf[0] > 0 {
            Err(NoteError::RemainingData)
        } else {
            Ok(available)
        }
    }

    /// Read any remaining data from the Notecarrier.
    fn consume_response(&mut self) -> Result<(), NoteError> {
        while self.data_query()? > 0 {
            // Consume any left-over response.
        }
        Ok(())
    }

    fn handshake(&mut self) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Handshake) {
            self.consume_response()?;

            self.state = NoteState::Request;
        }
        Ok(())
    }

    fn request(&mut self, cmd: &[u8]) -> Result<(), NoteError> {
        if matches!(self.state, NoteState::Request) {
            for c in cmd.chunks(255) {
                // Send length
                self.i2c
                    .write(self.addr, &[c.len() as u8])
                    .map_err(|_| NoteError::I2cWriteError)?;

                // Send chunks
                self.i2c
                    .write(self.addr, c)
                    .map_err(|_| NoteError::I2cWriteError)?;
            }

            // Send length
            self.i2c
                .write(self.addr, &[1])
                .map_err(|_| NoteError::I2cWriteError)?;

            // Terminate command
            self.i2c
                .write(self.addr, b"\n")
                .map_err(|_| NoteError::I2cWriteError)?;

            self.state = NoteState::Poll(0);

            Ok(())
        } else {
            Err(NoteError::WrongState)
        }
    }

    // pub fn init(&mut self) -> Result<(), NoteError> {
    //     mat
    // }

    /// [card Requests](https://dev.blues.io/reference/notecard-api/card-requests/#card-location)
    pub fn card(&mut self) -> card::Card<IOM> {
        card::Card::from(self)
    }
}

/// A future response. This probably requires some pin-projecting..
#[must_use]
pub struct FutureResponse<'a, T, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> {
    note: &'a mut Note<IOM>,
    _r: core::marker::PhantomData<T>,
}

impl<'a, T, IOM: Write<SevenBitAddress> + Read<SevenBitAddress>> FutureResponse<'a, T, IOM> {
    // pub fn poll(&mut self) -> bool {
    // }

    pub fn wait_raw(self) -> &'a [u8] {
        self.note.buf.as_slice()
    }

    pub fn wait(self) -> Result<T, NoteError> {
        // TODO: deserialize
        unimplemented!()
    }
}
