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
use ufmt::uwriteln;
use defmt::{Logger, Write, global_logger};
use defmt::{error, warn, info, debug, trace};
use core::ptr;

use core::cell::RefCell;

// mod note;

pub struct UartLogger {
    uart: hal::uart::Uart0
}

// unsafe impl Sync for UartLogger {}

// impl stlog::GlobalLog for UartLogger {
//     fn log(&self, address: u8) {
//         if let Some(uart) = self.uart.borrow_mut().as_mut() {
//             nb::block!(uart.write(address));
//         }
//     }
// }

#[global_logger]
pub struct UartGlobalLogger;

unsafe impl defmt::Logger for UartGlobalLogger {
    fn acquire() -> Option<ptr::NonNull<dyn Write>> {
        unsafe {
            LOGGER.as_mut().map(|l| ptr::NonNull::new_unchecked(l as &mut dyn defmt::Write))
        }
    }

    unsafe fn release(writer: ptr::NonNull<dyn Write>) {
    }
}

impl defmt::Write for UartLogger {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            nb::block!(self.uart.write(*b));
        }
    }
}

static mut LOGGER: Option<UartLogger> = None;


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
    unsafe { LOGGER = Some(UartLogger { uart: serial }); }

    let mut i2c = hal::i2c::I2c::new(dp.IOM4, pins.d15, pins.d14);
    // let note = note::Note::new(i2c);

    // Set up BSP leds
    let mut led = pins.d13.into_push_pull_output();

    let mut i = 0;

    let noteaddr = 0x17u8;

    // Blink forever
    loop {
        defmt::info!("hello world!");
        // info!("hello world!");
        // uwriteln!(&mut serial, "hello world: {}\r", i).unwrap();
        i += 1;

        // Toggle LEDs
        led.toggle().unwrap();

        // Delay
        delay.delay_ms(300u32);

        // uwriteln!(&mut serial, "scanning i2c bus:\r");
        for a in 0x14u8..0x19u8 {
            info!("trying: 0x{:02x}", a);
            if a == 0x17 {
                debug!("trying notecarrier");
                // uwriteln!(&mut serial, "Trying: notecarrier (0x17)\r");
            } else {
                // uwriteln!(&mut serial, "Trying {}\r", a);
            }
            let r = i2c.write(a, &[]);
            if r.is_ok() {
                warn!("0x{:02x}: {:?}", a, r);
            } else {
                info!("0x{:02x}: {:?}", a, r);
            }
            // uwriteln!(&mut serial, "{}: {:?}\r", a, r);
            delay.delay_ms(300u32);
        }


        // Write something to the noteboard

        // i2c.write(noteaddr, r#"{"req": "card.time"}\n"#.as_bytes());

        delay.delay_ms(300u32);

        // let mut buffer = [0u8; 10];
        // i2c.read(noteaddr, &mut buffer);
        // ufmt::uwriteln!(&mut serial, "note: {}", unsafe { core::str::from_utf8_unchecked(&buffer) });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2, 1 + 1)
    }
}
