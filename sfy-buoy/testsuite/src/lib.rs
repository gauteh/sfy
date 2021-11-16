#![no_std]
#![cfg_attr(test, no_main)]

use panic_probe as _; // memory layout + panic handler
use defmt_rtt as _;
use ambiq_hal as _;

#[defmt_test::tests]
mod tests {}
