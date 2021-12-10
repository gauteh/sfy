use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::i2c::{Read, Write};
use embedded_hal::blocking::delay::DelayMs;
use notecard::{Notecard, NoteError};

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C) -> Notecarrier<I2C> {
        let mut note = Notecard::new(i2c);
        note.initialize().expect("could not initialize notecard.");

        note.hub()
            .set(Some("no.met.gauteh:sfy"), None, None, Some("cain"))
            .unwrap()
            .wait()
            .ok();

        note.card().location_mode(Some("periodic"), Some(60), None, None, None, None, None, None).unwrap().wait().ok();
        note.card().location_track(true, false, true, None, None).unwrap().wait().ok();

        Notecarrier {
            note
        }
    }

    /// Initiate sync and wait for it to complete (or time out).
    pub fn sync_and_wait(&mut self, delay: &mut impl DelayMs<u32>, timeout_ms: u32) -> Result<bool, NoteError> {
        defmt::info!("sync..");
        self.note.hub().sync()?.wait()?;

        for _ in 0..(timeout_ms / 1000) {
            delay.delay_ms(1000u32);
            defmt::debug!("querying sync status..");
            let status = self.note.hub().sync_status()?.wait();
            defmt::debug!("status: {:?}", status);

            if let Ok(status) = status {
                if status.completed.is_some() {
                    defmt::info!("successful sync.");
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Set up note templates for sensor data and other messages, this will save space and
    /// bandwidth.
    fn setup_templates(&mut self) -> Result<(), NoteError> {


        Ok(())
    }
}

impl<I2C: Read + Write> Deref for Notecarrier<I2C> {
    type Target = Notecard<I2C>;

    fn deref(&self) -> &Self::Target {
        &self.note
    }
}

impl<I2C: Read + Write> DerefMut for Notecarrier<I2C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.note
    }
}
