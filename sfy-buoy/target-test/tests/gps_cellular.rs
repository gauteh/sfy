#![no_std]
#![no_main]

use panic_probe as _; // memory layout + panic handler
use defmt_rtt as _;
use ambiq_hal as hal;
use sfy_buoy as sfy;

use sfy::{*, note::Notecarrier};

#[defmt_test::tests]
mod tests {
    use defmt::{info, assert, assert_eq};
    use super::*;

    #[init]
    fn setup() -> Notecarrier {
        defmt::debug!("Setting up peripherals");
        let dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let i2c = hal::i2c::I2c::new(dp.IOM2, pins.d17, pins.d18);

        defmt::info!("Setting up notecarrier");
        Notecarrier::new(i2c)
    }

    #[test]
    fn ping_notecarrier(note: &mut Notecarrier) {
        assert_eq!(note.ping(), true, "notecarrier / notecard _not_ attached and responding!");
    }
}

