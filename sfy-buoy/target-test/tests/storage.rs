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
    use embedded_hal::spi;
    use hal::spi::{Freq, Spi};
    use half::f16;
    use heapless::Vec;

    use sfy::storage::Storage;
    use sfy::axl::{AXL_SZ, AxlPacket};

    struct State {
        // note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        #[allow(unused)]
        rtc: hal::rtc::Rtc,

        storage: Storage,
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
        let spi = Spi::new(
            dp.IOM0,
            pins.d12,
            pins.d13,
            pins.d11,
            Freq::F100kHz,
            spi::MODE_0,
        );

        let cs = pins.a14.into_push_pull_output();
        let storage = Storage::open(spi, cs).unwrap();

        State { delay, rtc, storage }
    }

    #[test]
    fn initialize_storage(s: &mut State) {
        defmt::info!("current id: {:?}", s.storage.current_id());
        assert_eq!(s.storage.current_id(), Some(0), "tests run on card with data");
    }

    #[test]
    fn write_id(s: &mut State) {
        s.storage.write_id().unwrap();
        assert_eq!(s.storage.read_id().unwrap(), 0);

        s.storage.set_id(1);
        s.storage.write_id().unwrap();
        assert_eq!(s.storage.read_id().unwrap(), 1);

        s.storage.set_id(0);
        s.storage.write_id().unwrap();
        assert_eq!(s.storage.read_id().unwrap(), 0);
    }

    #[test]
    fn write_package(s: &mut State) {
        let mut p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.storage_id, Some(0));

        let mut p = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.storage_id, Some(1));
    }
}
