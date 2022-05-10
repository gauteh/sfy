#![no_std]
#![no_main]

use ambiq_hal as hal;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};
    use hal::spi::{Freq, Spi};
    use embedded_hal::spi;

    struct State {
        // note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        #[allow(unused)]
        rtc: hal::rtc::Rtc,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);

        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let spi = Spi::new(dp.IOM0, pins.d11, pins.d12, pins.d13, Freq::F100kHz, spi::MODE_0);

        // defmt::info!("Setting up notecarrier");
        // let note = Notecarrier::new(i2c, &mut delay).unwrap();

        State { delay, rtc }
    }

    #[test]
    fn ping_sd_card(s: &mut State) {
    }
}

