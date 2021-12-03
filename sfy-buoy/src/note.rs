use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::i2c::{Read, Write};
use notecard::Notecard;

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C) -> Notecarrier<I2C> {
        let mut note = Notecard::new(i2c);
        note.initialize().expect("could not initialize notecard.");

        note.hub()
            .set(Some("com.vetsj.gaute.eg:sby"), None, None, Some("cain"))
            .unwrap()
            .wait()
            .ok();

        note.card().location_mode(Some("periodic"), Some(60), None, None, None, None, None, None).unwrap().wait().ok();
        note.card().location_track(true, false, true, None, None).unwrap().wait().ok();

        Notecarrier {
            note
        }
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
