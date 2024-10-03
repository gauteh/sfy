//! GPS interface
//!
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use heapless::Vec;

#[derive(serde::Deserialize, PartialEq)]
struct Sample {
    time: u64,
    lon: f32,
    lat: f32,
    z: f32,
}

struct Gps {
    buf: Vec<Sample, 1024>,
}

impl Gps {
    pub fn new(buf: &'static mut [u8]) -> Gps {
        Gps { buf: Vec::new() }
    }
}

pub fn sample(serial: &mut impl embedded_hal::serial::Read<u8>) {}
