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

mod note;

#[entry]
fn main() -> ! {
    let mut peripherals = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();

    let mut delay = hal::delay::Delay::new(core.SYST, &mut peripherals.CLKGEN);

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
    let led = 0u32;

    // uint32_t ux, ui32GPIONumber;
    // for (ux = 0; ux < leds; ux++) {
    //     ui32GPIONumber = am_bsp_psLEDs[ux].ui32GPIONumber;
    //     am_hal_gpio_pinconfig(ui32GPIONumber, g_AM_HAL_GPIO_OUTPUT);
    //     am_devices_led_off(am_bsp_psLEDs, ux);
    // }

    unsafe {
        let gpion = halc::am_bsp_psLEDs[0].ui32GPIONumber;
        halc::am_hal_gpio_pinconfig(gpion, halc::g_AM_HAL_GPIO_OUTPUT);
        halc::am_devices_led_off(halc::am_bsp_psLEDs.as_mut_ptr(), led);
    }
    let mut led_state = false;

    // Blink forever
    loop {
        // Toggle LEDs
        led_state = !led_state;
        if led_state {
            unsafe {
                halc::am_devices_led_off(halc::am_bsp_psLEDs.as_mut_ptr(), led);
            }
        } else {
            unsafe {
                halc::am_devices_led_on(halc::am_bsp_psLEDs.as_mut_ptr(), led);
            }
        }
        // uint32_t ux;
        // for (ux = 0; ux < leds; ux++) {
        //     ui32GPIONumber = am_bsp_psLEDs[ux].ui32GPIONumber;
        //     (led_state) ? am_devices_led_on(am_bsp_psLEDs, ux) : am_devices_led_off(am_bsp_psLEDs, ux);
        // }

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
