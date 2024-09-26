//! UBLOX RTK GPS interface
//!
//! Probably need to communicate over UART to avoid knowing I2C telegram length in advance.
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use heapless::Vec;

use static_cell::StaticCell;
use ublox::{FixedLinearBuffer, Parser};

static BUF: StaticCell<Vec<u8, 256>> = StaticCell::new();

struct Gps {
    parser: Parser<FixedLinearBuffer<'static>>,
}

impl Gps {
    pub fn new(buf: &'static mut [u8]) -> Gps {
        let buf = FixedLinearBuffer::new(buf);
        let parser = Parser::new(buf);
        Gps { parser }
    }
}
