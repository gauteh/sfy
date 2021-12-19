#![no_std]
#![no_main]

use ambiq_hal as _;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};

    #[test]
    fn assert_true() {
        assert!(true)
    }
}
