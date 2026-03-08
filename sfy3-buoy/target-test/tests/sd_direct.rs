#![no_std]
#![no_main]

extern crate cmsis_dsp; // sinf, cosf, etc
use ambiq_hal as hal;
use core::sync::atomic::AtomicI32;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

use embedded_hal::{
    blocking::delay::DelayUs,
    blocking::spi::{write::Default as DefaultWrite, Transfer},
    digital::v2::OutputPin,
    spi::FullDuplex,
};
use embedded_sdmmc::{
    Error as GenericSdMmcError, Mode, SdCard, SdCardError, VolumeIdx, VolumeManager,
};
use sfy::storage::{self, Storage};

pub static COUNT: AtomicI32 = AtomicI32::new(0);

type Spi0 = hal::spi::Spi0;
type CS = hal::gpio::pin::P35<{ hal::gpio::Mode::Output }>;
type DL = hal::delay::FlashDelay;

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};
    use embedded_hal::{prelude::*, spi};
    use embedded_sdmmc::{TimeSource, Timestamp};
    use hal::spi::{Freq, Spi};
    use heapless::Vec;

    use sfy::axl::{AxlPacket, AXL_POSTCARD_SZ, AXL_SZ, VERSION};
    use sfy::storage::{SdSpiSpeed, Storage};

    struct Clock;
    impl TimeSource for Clock {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp::from_calendar(0, 0, 0, 0, 0, 0).unwrap()
        }
    }

    //     struct State {
    //         // note: Notecarrier<hal::i2c::Iom2>,
    //         #[allow(unused)]
    //         delay: hal::delay::Delay,
    //         #[allow(unused)]
    //         rtc: hal::rtc::Rtc,

    //         sd: VolumeManager<Spi0, CS, DL>,
    //     }

    //     #[init]
    //     fn setup() -> State {
    //         defmt::debug!("Setting up peripherals");
    //         let core = hal::pac::CorePeripherals::take().unwrap();
    //         let mut dp = hal::pac::Peripherals::take().unwrap();
    //         let pins = hal::gpio::Pins::new(dp.GPIO);

    //         let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
    //         let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

    //         let cs = pins.a14;
    //         let mut cs = cs.into_push_pull_output();
    //         cs.internal_pull_up(true);
    //         // cs.set_high();

    //         delay.delay_ms(300_u32);

    //         defmt::info!("Setting up SPI");
    //         let spi = Spi::new(
    //             dp.IOM0,
    //             pins.d12,
    //             pins.d13,
    //             pins.d11,
    //             Freq::F100kHz,
    //             spi::MODE_0,
    //         );

    //         delay.delay_ms(300_u32);

    //         let mut storage = Storage::open(
    //             spi,
    //             cs,
    //             sfy::storage::clock::CountClock(&COUNT),
    //             |spi, speed| match speed {
    //                 SdSpiSpeed::Low => spi.set_freq(Freq::F100kHz),
    //                 SdSpiSpeed::High => spi.set_freq(Freq::F48mHz),
    //             },
    //             hal::delay::FlashDelay,
    //         );

    //         // clean up previous tests
    //         assert_eq!(sfy::storage::STORAGE_VERSION_STR, "t");
    //         clean_up_collection(&mut storage);

    //         State {
    //             delay,
    //             rtc,
    //             storage,
    //         }
    //     }

    #[test]
    fn initialize_storage() {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        let cs = pins.a14;
        let mut cs = cs.into_push_pull_output();
        cs.internal_pull_up(true);
        // cs.set_high();

        delay.delay_ms(300_u32);

        defmt::info!("Setting up SPI");
        let spi = Spi::new(
            dp.IOM0,
            pins.d12,
            pins.d13,
            pins.d11,
            Freq::F100kHz,
            spi::MODE_0,
        );

        delay.delay_ms(300_u32);

        defmt::info!("Construct SdCard..");
        let fdelay = hal::delay::FlashDelay;
        let sd = SdCard::new(spi, cs, fdelay);
        delay.delay_ms(300_u32);

        let bt = sd.num_bytes();
        defmt::info!("num bytes: {}", bt);
        delay.delay_ms(300_u32);

        defmt::info!("Construct VolManager..");
        let vol = VolumeManager::new(sd, Clock);

        delay.delay_ms(300_u32);
    }
}
