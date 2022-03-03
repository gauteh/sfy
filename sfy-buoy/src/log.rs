use ambiq_hal as hal;
use hal::{i2c, delay::FlashDelay};
use heapless::{mpmc::Q16, String};
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write},
};
use cortex_m::interrupt::free;
use notecard::NoteError;

use crate::note::Notecarrier;

/// Log message queue for messages to be sent back over notecard.
static LOGQ: Q16<String<256>> = Q16::new();


/// A reference to the Notecarrier once it is initialized. The idea is that
/// it can be used from reset routines to transfer log messages. In that case the main thread will
/// not be running anyway.
pub static mut NOTE: Option<*mut Notecarrier<i2c::Iom2>> = None;

pub fn log(msg: &str) {
    defmt::debug!("logq: {}", msg);
    LOGQ.enqueue(String::from(msg)).inspect_err(|e| defmt::error!("failed to queue message: {:?}", e)).ok();
}

pub fn drain_log<I: Read + Write>(note: &mut Notecarrier<I>, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
    note.drain_log(&LOGQ, delay)
}

/// Tries to send the remaining queue to notecard in case of panic or HardFault.
pub unsafe fn panic_drain_log() {
    defmt::warn!("entering panic_drain_log.");
    free(|_| {
        if let Some(note) = NOTE {
            defmt::info!("NOTE is set, consuming response and sending log..");
            let note: &mut Notecarrier<i2c::Iom2> = &mut *note;
            let mut delay = FlashDelay::new();

            note.reset(&mut delay).ok();
            delay.delay_ms(50u16);

            drain_log(note, &mut delay).inspect_err(|e| defmt::error!("failed to drain log to notecard: {:?}", e)).ok();

            delay.delay_ms(4000u16);
        } else {
            defmt::error!("NOTE is not set.");
        }
    })
}

