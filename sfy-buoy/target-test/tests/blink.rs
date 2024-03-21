#![no_std]
#![no_main]

extern crate cmsis_dsp; // sinf, cosf, etc
use ambiq_hal::{self as hal, prelude::*};
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler


// pub static COUNT: AtomicI32 = AtomicI32::new(0);
// defmt::timestamp!("{=i32}", COUNT.load(Ordering::Relaxed));

#[defmt_test::tests]
mod tests {
    use super::*;

    #[allow(unused)]
    use defmt::{assert, assert_eq, info};

    #[test]
    fn blink() {
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

        let mut dp = hal::pac::Peripherals::take().unwrap();
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        let pins = hal::gpio::Pins::new(dp.GPIO);


        let mut led = pins.d19.into_push_pull_output();

        info!("Blinking to indicate start-up.");
        led.set_high().unwrap();

        info!("Giving subsystems a couple of seconds to boot..");
        delay.delay_ms(5_000u32);

        led.set_low().unwrap();

        loop {
            info!("Loop 1s!");
            led.toggle().unwrap();
            delay.delay_ms(1_000u32);
        }
    }
}
