#![no_std]
#![no_main]

use ambiq_hal as hal;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler
use sfy_buoy as sfy;

#[allow(unused)]
use sfy::{note::Notecarrier, *};

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};

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
        assert_eq!(
            note.ping(),
            true,
            "notecarrier / notecard _not_ attached and responding!"
        );
    }
}
