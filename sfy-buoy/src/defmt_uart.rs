use core::ptr;
use defmt::{global_logger, Write};

use crate::hal::{uart, prelude::*};

pub struct UartLogger {
    pub uart: uart::Uart0,
}

#[global_logger]
pub struct UartGlobalLogger;

unsafe impl defmt::Logger for UartGlobalLogger {
    fn acquire() -> Option<ptr::NonNull<dyn Write>> {
        unsafe {
            LOGGER
                .as_mut()
                .map(|l| ptr::NonNull::new_unchecked(l as &mut dyn defmt::Write))
        }
    }

    unsafe fn release(_writer: ptr::NonNull<dyn Write>) {}
}

impl defmt::Write for UartLogger {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            nb::block!(self.uart.write(*b)).ok();
        }
    }
}

pub static mut LOGGER: Option<UartLogger> = None;

