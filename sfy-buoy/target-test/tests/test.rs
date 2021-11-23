#![no_std]
#![no_main]

use panic_probe as _; // memory layout + panic handler
use defmt_rtt as _;
use ambiq_hal as _;

// See https://crates.io/crates/defmt-test/0.1.0 for more documentation (e.g. about the 'state'
// feature)
#[defmt_test::tests]
mod tests {
    use defmt::{info, assert, assert_eq};

    #[test]
    fn assert_true() {
        assert!(true)
    }

    #[ignore]
    #[test]
    fn assert_eq() {
        assert_eq!(24, 42, "TODO: write actual tests")
    }

    #[test]
    fn fail() {
        info!("some info");
        assert!(false);
    }
}