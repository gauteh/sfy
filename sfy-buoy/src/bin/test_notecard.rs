#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use ambiq_hal::{self as hal, prelude::*};
use notecard::Notecard;
use defmt_rtt as _;
#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

#[entry]
fn main() -> ! {

    loop {}
}
