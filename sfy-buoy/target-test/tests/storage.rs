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
    use hal::prelude::*;
    use embedded_hal::spi;
    use embedded_sdmmc::SdMmcSpi;

    struct State {
        // note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        #[allow(unused)]
        rtc: hal::rtc::Rtc,

        sd: SdMmcSpi<hal::spi::Spi0, hal::gpio::pin::P35<{ hal::gpio::pin::Mode::Output }>>,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        defmt::info!("Setting up SPI");
        let spi = Spi::new(dp.IOM0, pins.d12, pins.d13, pins.d11, Freq::F100kHz, spi::MODE_0);

        let cs = pins.a14.into_push_pull_output();

        defmt::info!("Setting up SD SPI card driver");
        let sd = SdMmcSpi::new(spi, cs);

        State { delay, rtc, sd }
    }

    #[test]
    fn init_sd_card(s: &mut State) {
        defmt::info!("init SD");
        let _block = s.sd.acquire().unwrap();
    }
}

