use blues_notecard::NoteError;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Read, Write},
};
use heapless::{mpmc::Q4 as Queue, String};

use crate::note::Notecarrier;

/// Log message queue for messages to be sent back over notecard.
static LOGQ: Queue<String<256>> = Queue::new();

pub fn log(msg: &str) {
    defmt::debug!("logq: {}", msg);
    let mut s = String::new();
    s.push_str(msg).ok();
    LOGQ.enqueue(s)
        .inspect_err(|e| defmt::error!("failed to queue message: {:?}", e))
        .ok();
}

pub fn drain_log<I: Read + Write>(
    note: &mut Notecarrier<I>,
    delay: &mut impl DelayMs<u16>,
) -> Result<(), NoteError> {
    note.drain_log(&LOGQ, delay)
}

/// Tries to send the remaining queue to notecard in case of panic or HardFault. Must be wrapped
/// in free() to avoid multiple access.
pub unsafe fn panic_drain_log<IOM: Read + Write>(
    note: Option<*mut Notecarrier<IOM>>,
    delay: &mut impl DelayMs<u16>,
) {
    defmt::warn!("entering panic_drain_log.");
    if let Some(note) = note {
        defmt::info!("NOTE is set, consuming response and sending log..");

        let note: &mut Notecarrier<_> = &mut *note;
        note.reset(delay).ok();
        delay.delay_ms(50u16);

        drain_log(note, delay)
            .inspect_err(|e| defmt::error!("failed to drain log to notecard: {:?}", e))
            .ok();

        delay.delay_ms(4000u16);
    } else {
        defmt::error!("NOTE is not set.");
    }
}
