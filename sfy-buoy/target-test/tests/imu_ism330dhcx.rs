#![no_std]
#![no_main]

use ambiq_hal as hal;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};

    use hal::i2c::{I2c, Freq};

    use sfy::waves::Waves;

    struct State {
        waves: Waves<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let i2c = I2c::new(dp.IOM2, pins.d17, pins.d18, Freq::F100kHz);

        defmt::info!("Setting up wave sensor");
        let waves = Waves::new(i2c);

        State {
            waves,
            delay
        }
    }

    #[test]
    fn get_temperature(s: &mut State) {
        let temp = s.waves.get_temperature();
        defmt::info!("temperature: {:?}", temp);
    }
}

