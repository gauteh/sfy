#![no_std]
#![no_main]

extern crate cmsis_dsp; // sinf, cosf, etc
use ambiq_hal::{self as hal, prelude::*};
use core::sync::atomic::AtomicI32;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

use embedded_hal::{
    blocking::delay::DelayUs,
    blocking::spi::{write::Default as DefaultWrite, Transfer},
    digital::v2::{OutputPin, InputPin},
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

    use hal::delay::Delay;
    use hal::gpio::Pins;

    #[init]
    fn setup() {
        defmt::info!("Setting up!");
        unsafe {
            // Set the clock frequency.
            halc::am_hal_clkgen_control(
                halc::am_hal_clkgen_control_e_AM_HAL_CLKGEN_CONTROL_SYSCLK_MAX,
                0 as *mut c_void,
            );

            // Set the default cache configuration
            halc::am_hal_cachectrl_config(&halc::am_hal_cachectrl_defaults);
            halc::am_hal_cachectrl_enable();

            // Configure the board for low power operation.
            halc::am_bsp_low_power_init();
        }

        defmt::info!("setup done..");
    }

    #[cfg(disabled)]
    #[test]
    fn test_io() {
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        let pins = hal::gpio::Pins::new(dp.GPIO);

        let mut led = pins.d19.into_push_pull_output();

        // pin D8 (on artemis nano)
        // let mut tp = pins.d8.into_input_output();
        let mut tp = pins.d10.into_input_output();
        tp.open_drain();
        // tp.internal_pull_up(true);

        led.set_low();
        // delay.delay_ms(5000_u32);
        // assert_eq!(tp.is_high().unwrap(), false);

        // test write + read
        // tp.set_low().unwrap();
        // tp.set_high().unwrap();
        // delay.delay_ms(1000_u32);
        // assert_eq!(tp.is_high().unwrap(), true);

        defmt::flush();
        // delay.delay_ms(1000_u32);

        defmt::info!("setting up dsb driver");
        // let temp = sfy::temp::Temps::new(tp, &mut delay).expect("failed to open temp");

        led.set_high();
        // delay.delay_ms(5000_u32);
        led.set_low();
        // delay.delay_ms(1000_u32);
        tp.set_high();
        for _ in 0..1000 {
            defmt::info!("test");
            let state = tp.is_high().unwrap();
            defmt::info!("state: {}", state);
            // tp.toggle().ok();
            led.toggle().ok();
            delay.delay_ms(1000_u32);
            defmt::flush();
        }
    }

    #[test]
    fn find_probes() {
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        let mut delay = hal::delay::FlashDelay::new();

        let pins = hal::gpio::Pins::new(dp.GPIO);

        let mut led = pins.d19.into_push_pull_output();

        // pin D8 (on artemis nano)
        let mut tp = pins.d8.into_input();
        // let mut tp = pins.d10.into_input_output();
        // tp.internal_pull_up(true);
        // tp.open_drain();
        // tp.set_high().ok();
        // assert_eq!(tp.is_high().unwrap(), true);

        // tp.set_low().ok();
        // delay.delay_ms(1000_u32);
        // assert_eq!(tp.is_low().unwrap(), true);

        // defmt::info!("blinking+toggling voltage N times");
        // tp.set_high();

        // for i in 0..10 {
        //     defmt::info!("test");
        //     let state = tp.is_high().unwrap();
        //     defmt::info!("state: {}", state);
        //     if i % 2 == 0 {
        //         assert_eq!(state, true);
        //     } else {
        //         assert_eq!(state, false);
        //     }
        //     tp.toggle().ok();
        //     led.toggle().ok();
        //     delay.delay_ms(1000_u32);
        //     defmt::flush();
        // }

        // tp.set_high().ok();

        defmt::info!("setting up dsb driver");
        defmt::flush();
        delay.delay_ms(1000_u32);
        let temp = sfy::temp::Temps::new(tp, &mut delay).unwrap();

        for _ in 0..1000 {
            defmt::info!("loop");
            // let state = tp.is_high().unwrap();
            // defmt::info!("state: {}", state);
            led.toggle().ok();
            delay.delay_ms(1000_u32);
            defmt::flush();
        }
    }
}

