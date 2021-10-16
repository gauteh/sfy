#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

// pick a panicking behavior
#[cfg(not(test))]
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use ambiq_hal as hal;
use cortex_m_rt::entry;
use hal::prelude::*;

mod defmt_uart;
use defmt_uart::{UartLogger, LOGGER};

#[allow(unused_imports)]
use defmt::{debug, error, info, trace, warn};

use notecard::Note;

#[entry]
fn main() -> ! {
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

    let mut dp = hal::pac::Peripherals::take().unwrap();
    let core = hal::pac::CorePeripherals::take().unwrap();

    let pins = hal::gpio::Pins::new(dp.GPIO);

    let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
    let serial = hal::uart::Uart0::new(dp.UART0, pins.tx0, pins.rx0);
    unsafe {
        LOGGER = Some(UartLogger { uart: serial });
    }

    // Initialize notecard
    let i2c = hal::i2c::I2c::new(dp.IOM4, pins.d15, pins.d14);
    let mut note = Note::new(i2c);
    note.initialize().ok();

    let mut led = pins.d13.into_push_pull_output();

    defmt::info!("Waiting to start..!");
    delay.delay_ms(2000u32);

    defmt::info!("hello world!");

    // Delay
    delay.delay_ms(300u32);

    if note.ping() {
        warn!("notecard found!");
    } else {
        error!("notecard not found!");
    }

    delay.delay_ms(1000u32);
    info!("done: looping.");

    loop {
        delay.delay_ms(2000u32);
        info!("note: card.time");
        info!("note: time: {:?}", note.card().time().unwrap().wait());

        info!("querying status..");
        info!("status: {:?}", note.card().status().unwrap().wait());

        // Toggle LEDs
        led.toggle().unwrap();
    }
}

#[cfg(test)]
mod tests {
}
