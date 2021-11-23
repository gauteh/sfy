use core::ops::{Deref, DerefMut};
use notecard::Notecard;
use crate::*;

pub struct Notecarrier {
    note: Notecard<hal::i2c::Iom2>,
}

impl Notecarrier {
    pub fn new(i2c: hal::i2c::Iom2) -> Notecarrier {
        let mut note = Notecard::new(i2c);
        note.initialize().expect("could not initialize notecard.");

        Notecarrier {
            note
        }
    }
}

impl Deref for Notecarrier {
    type Target = Notecard<hal::i2c::Iom2>;

    fn deref(&self) -> &Self::Target {
        &self.note
    }
}

impl DerefMut for Notecarrier {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.note
    }
}
