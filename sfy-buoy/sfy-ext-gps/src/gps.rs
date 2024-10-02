//! GPS interface
//!
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};

struct Gps {}

impl Gps {
    pub fn new(buf: &'static mut [u8]) -> Gps {
        Gps {}
    }
}
