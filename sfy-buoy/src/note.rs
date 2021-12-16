use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::i2c::{Read, Write};
use embedded_hal::blocking::delay::DelayMs;
use notecard::{Notecard, NoteError};
use half::f16;

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C) -> Result<Notecarrier<I2C>, NoteError> {
        let mut note = Notecard::new(i2c);
        note.initialize().expect("could not initialize notecard.");

        note.hub()
            .set(Some("no.met.gauteh:sfy"), None, None, Some("cain"))?
            .wait()?;

        note.card().location_mode(Some("periodic"), Some(60), None, None, None, None, None, None).unwrap().wait().ok();
        note.card().location_track(true, false, true, None, None).unwrap().wait().ok();

        let mut n = Notecarrier {
            note
        };

        n.setup_templates()?;

        Ok(n)
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
        defmt::debug!("setting up templates..");


        Ok(())
    }

    pub fn send(&mut self, pck: AxlPacket) -> Result<notecard::note::res::Add, NoteError> {
        defmt::debug!("sending acceleration package");

        let (sz, b64) = pck.base64();
        let b64 = &b64[..sz];

        let mut payload = b64.chunks(250);

        // Send first payload with timestamp
        let r = self.note.note().add(Some("axl.qo"), None, Some(pck), payload.next().map(|b| core::str::from_utf8(b).unwrap()), false)?.wait()?;

        for p in payload {
            self.note.note().add::<AxlPacket>(Some("axl.qo"), None, None, Some(core::str::from_utf8(p).unwrap()), false)?.wait()?;
        }

        Ok(r)
    }
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacket {
    pub timestamp: u32,

    /// This is added to the payload of the note.
    #[serde(skip)]
    pub data: heapless::Vec<f16, { 3 * 1024 }>
}

pub const AXL_OUTN: usize = {3 * 1024} * 4 * 4 / 3 + 4;

#[derive(serde::Serialize)]
pub struct AxlPacketJson {
    pub timestamp: u32,

    /// This is added to the payload of the note.
    #[serde(skip)]
    pub data: heapless::String<AXL_OUTN>,
}

impl AxlPacket {
    pub fn base64(&self) -> (usize, [u8; AXL_OUTN]) {
        let mut buf = [0u8; AXL_OUTN];

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut buf);

        (written, buf)
    }

    pub fn as_json(self) -> AxlPacketJson {
        let (sz, b64) = self.base64();
        let b64 = &b64[..sz];
        let b64 = core::str::from_utf8(&b64).unwrap();

        AxlPacketJson {
            timestamp: self.timestamp,
            data: heapless::String::from(b64)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_data_package() {
        let p = AxlPacket {
            timestamp: 0,
            data: (0..3072).map(|v| f16::from_f32(v as f32)).collect::<heapless::Vec<_, { 3 * 1024 }>>()
        };

        let (sz, data) = p.base64();
        println!("base64 sz: {}", sz);
    }

    #[test]
    fn serialize_for_notecard() {
        let p = AxlPacket {
            timestamp: 0,
            data: (0..3072).map(|v| f16::from_f32(v as f32)).collect::<heapless::Vec<_, { 3 * 1024 }>>()
        };
        let (sz, b64) = p.base64();
        let b64 = &b64[..sz];
        let b64 = core::str::from_utf8(&b64).unwrap();
    }

    #[test]
    fn data_package_json() {
        let p = AxlPacket {
            timestamp: 0,
            data: (0..3072).map(|v| f16::from_f32(v as f32)).collect::<heapless::Vec<_, { 3 * 1024 }>>()
        };

        let p = p.as_json();
        let s = serde_json_core::to_string::<_, {AXL_OUTN + 256}>(&p).unwrap();
        println!("serialized: {}", s);
    }
}
