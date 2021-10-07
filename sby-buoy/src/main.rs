#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

// pick a panicking behavior
#[cfg(not(test))]
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger


use ambiq_hal as hal;
use hal::prelude::{*, halc::c_types::*};
use cortex_m_rt::entry;

// mod note;

#[entry]
fn main() -> ! {
    let mut dp = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();

    let pins = hal::gpio::Pins::new(dp.GPIO);

    let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
    let mut serial = hal::uart::Uart0::new(dp.UART0, pins.tx0, pins.rx0);

    // Set the clock frequency.
    unsafe {
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

    // Set up BSP leds
    let mut led = pins.d13.into_push_pull_output();

    // Blink forever
    loop {
        ufmt::uwriteln!(&mut serial, "hello world\r").unwrap();

        // Toggle LEDs
        led.toggle().unwrap();

        // Delay
        delay.delay_ms(300u32);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2, 1 + 1)
    }
}
