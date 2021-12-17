use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::i2c::{Read, Write};
use embedded_hal::blocking::delay::DelayMs;
use notecard::{Notecard, NoteError};
use half::f16;
use crate::waves::AXL_SZ;

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C, delay: &mut impl DelayMs<u16>) -> Result<Notecarrier<I2C>, NoteError> {
        let mut note = Notecard::new(i2c);
        note.initialize().expect("could not initialize notecard.");

        note.hub()
            .set(Some("no.met.gauteh:sfy"), None, None, Some("cain"))?
            .wait(delay)?;

        note.card().location_mode(Some("continuous"), None, None, None, None, None, None, None).unwrap().wait(delay)?;
        note.card().location_track(true, false, true, None, None).unwrap().wait(delay)?;

        let mut n = Notecarrier {
            note
        };

        n.setup_templates(delay)?;

        Ok(n)
    }

    /// Initiate sync and wait for it to complete (or time out).
    pub fn sync_and_wait(&mut self, delay: &mut impl DelayMs<u16>, timeout_ms: u16) -> Result<bool, NoteError> {
        defmt::info!("sync..");
        self.note.hub().sync()?.wait(delay)?;

        for _ in 0..(timeout_ms / 1000) {
            delay.delay_ms(1000u16);
            defmt::debug!("querying sync status..");
            let status = self.note.hub().sync_status()?.wait(delay);
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
    fn setup_templates(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        defmt::debug!("setting up templates..");

        #[derive(serde::Serialize, Default)]
        struct AxlPacketMetaTemplate {
            timestamp: u32,
            offset: u32,
            length: u32,
        }

        let meta_template = AxlPacketMetaTemplate {
            timestamp: 14,
            offset: 14,
            length: 14
        };

        defmt::debug!("setting up template for AxlPacketMeta");
        self.note().template(Some("axl.qo"), Some(meta_template), Some(AXL_OUTN as u32))?.wait(delay)?;

        Ok(())
    }

    pub fn send(&mut self, pck: AxlPacket, delay: &mut impl DelayMs<u16>) -> Result<usize, NoteError> {
        defmt::debug!("sending acceleration package");

        let b64 = pck.base64();

        // Send first payload with timestamp
        let mut offset: usize = 0;

        for p in b64.chunks(8 * 1024) {
            let meta = AxlPacketMeta {
                timestamp: pck.timestamp,
                offset: offset as u32,
                length: p.len() as u32,
            };

            let r = self.note.note().add(Some("axl.qo"), None, Some(meta), Some(core::str::from_utf8(p).unwrap()), false)?.wait(delay)?;

            offset += p.len();

            defmt::trace!("sent data package: {} of {} (note: {:?})", offset, b64.len(), r);
        }

        Ok(offset)
    }
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacketMeta {
    pub timestamp: u32,
    pub offset: u32,
    pub length: u32,
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacket {
    pub timestamp: u32,

    /// This is moved to the payload of the note.
    #[serde(skip)]
    pub data: heapless::Vec<f16, { AXL_SZ }>
}

/// Maximum length of base64 string from [f16; AXL_SZ]
pub const AXL_OUTN: usize = { AXL_SZ * 2 } * 4 / 3 + 4;


impl AxlPacket {
    pub fn base64(&self) -> heapless::Vec<u8, AXL_OUTN> {
        let mut b64: heapless::Vec<_, AXL_OUTN> = heapless::Vec::new();
        b64.resize_default(AXL_OUTN).unwrap();

        // Check endianness (TODO: use byteorder or impl in hidefix to swap order if compiled for
        // big endian machine).
        #[cfg(target_endian = "big")]
        compile_error!("serializied samples are assumed to be in little endian, target platform is big endian and no conversion is implemented.");

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut b64);
        b64.truncate(written);

        b64
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
            data: (0..3072).map(|v| f16::from_f32(v as f32)).collect::<heapless::Vec<_, { AXL_SZ }>>()
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }

    #[test]
    fn read_transmitted_data_package() {
        use std::fs;

        let sent_data = (0..3072).map(|v| f16::from_f32(v as f32)).collect::<heapless::Vec<_, { AXL_SZ }>>();

        let length: usize = 8192;
        let b64 = fs::read("tests/data/transmitted_payload.txt").unwrap();

        let b64 = &b64[..length];

        // this test assumes host platform is little endian

        let mut buf = Vec::with_capacity(3072 * 2);
        buf.resize(3072 * 2, 0);
        let data_bytes = base64::decode_config_slice(b64, base64::STANDARD, &mut buf).unwrap();
        let data_values = bytemuck::cast_slice::<_, half::f16>(&buf);

        assert_eq!(sent_data, data_values);
    }
}
