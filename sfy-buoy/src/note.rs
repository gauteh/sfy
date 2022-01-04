use crate::waves::AXL_SZ;
use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, Write};
use half::f16;
use notecard::{NoteError, Notecard};

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C, delay: &mut impl DelayMs<u16>) -> Result<Notecarrier<I2C>, NoteError> {
        let mut note = Notecard::new(i2c);
        note.initialize()?;

        note.hub()
            .set(
                Some("no.met.gauteh:sfy"),
                None,
                Some(notecard::hub::req::HubMode::Periodic),
                Some("cain"),
                Some(10), // max time between out-going sync in minutes.
                None,
                None,
                None,
                None,
                Some(true),
                None,
            )?
            .wait(delay)?;

        note.card()
            .location_mode(Some("continuous"), None, None, None, None, None, None, None)?
            .wait(delay)?;
        note.card()
            .location_track(true, false, true, None, None)?
            .wait(delay)?;


        let mut n = Notecarrier { note };

        n.setup_templates(delay)?;
        defmt::info!("initializing initial sync..");
        n.note.hub().sync()?.wait(delay)?;

        Ok(n)
    }

    /// Initiate sync and wait for it to complete (or time out).
    pub fn sync_and_wait(
        &mut self,
        delay: &mut impl DelayMs<u16>,
        timeout_ms: u16,
    ) -> Result<bool, NoteError> {
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
            freq: f32,
            packet: u32,
            lon: f32,
            lat: f32,
        }

        let meta_template = AxlPacketMetaTemplate {
            timestamp: 14,
            offset: 14,
            length: 14,
            freq: 14.1,
            packet: 12,
            lon: 14.1,
            lat: 14.1,
        };

        defmt::debug!("setting up template for AxlPacketMeta");
        self.note()
            .template(Some("axl.qo"), Some(meta_template), Some(AXL_OUTN as u32))?
            .wait(delay)?;

        Ok(())
    }

    pub fn send(
        &mut self,
        pck: AxlPacket,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<usize, NoteError> {
        defmt::debug!("sending acceleration package");

        let b64 = pck.base64();

        for (pi, p) in b64.chunks(8 * 1024).enumerate() {
            let meta = AxlPacketMeta {
                timestamp: pck.timestamp,
                offset: pck.offset as u32,
                packet: pi as u32,
                length: p.len() as u32,
                freq: pck.freq,
                lon: pck.lon,
                lat: pck.lat,
            };

            let r = self
                .note
                .note()
                .add(
                    Some("axl.qo"),
                    None,
                    Some(meta),
                    Some(core::str::from_utf8(p).unwrap()),
                    false,
                )?
                .wait(delay)?;

            defmt::debug!(
                "sent data package: {}, bytes: {} (note: {:?})",
                pi,
                b64.len(),
                r
            );
        }

        Ok(b64.len())
    }

    /// Send all available packages to the notecard.
    pub fn drain_queue(
        &mut self,
        queue: &mut heapless::spsc::Consumer<'static, AxlPacket, 16>,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<usize, NoteError> {
        let status = self.note.card().status()?.wait(delay)?;

        if status.storage > 75 {
            // wait untill notecard has synced.
            defmt::warn!("notecard is more than 75% full, not adding more notes untill sync is done: queue sz: {}", queue.len());
            return Ok(0usize);
        }

        delay.delay_ms(10);

        let mut sz = 0;

        while let Some(pck) = queue.dequeue() {
            sz += self.send(pck, delay)?;
        }

        Ok(sz)
    }

    /// Check if notecard is filling up, and initiate sync in that case.
    pub fn check_and_sync(
        &mut self,
        delay: &mut impl DelayMs<u16>
    ) -> Result<(), NoteError> {
        let status = self.note.card().status()?.wait(delay)?;

        if status.storage > 30 {
            delay.delay_ms(10);
            let sync_status = self.note.hub().sync_status()?.wait(delay)?;

            if sync_status.requested.is_none() {
                defmt::warn!("notecard is more than 30% full, initiating sync.");
                self.note.hub().sync()?.wait(delay)?;
                delay.delay_ms(10);
            }
            defmt::info!("notecard is filling up: sync status: {:?}", sync_status);
        }

        Ok(())
    }
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacketMeta {
    pub timestamp: u32,
    pub offset: u32,
    pub packet: u32,
    pub length: u32,
    pub freq: f32,
    pub lon: f32,
    pub lat: f32,
}

#[derive(Debug, serde::Serialize, Default)]
pub struct AxlPacket {
    /// Timstamp of sample at `offset`.
    pub timestamp: u32,
    pub offset: u16,
    pub lon: f32,
    pub lat: f32,
    pub freq: f32,

    /// This is moved to the payload of the note.
    #[serde(skip)]
    pub data: heapless::Vec<f16, { AXL_SZ }>,
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
            lat: 0.0,
            lon: 0.0,
            freq: 100.0,
            offset: 0,
            data: (0..3072)
                .map(|v| f16::from_f32(v as f32))
                .collect::<heapless::Vec<_, { AXL_SZ }>>(),
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }

    #[test]
    fn read_transmitted_data_package() {
        use std::fs;

        let sent_data = (0..3072)
            .map(|v| f16::from_f32(v as f32))
            .collect::<heapless::Vec<_, { AXL_SZ }>>();

        let length: usize = 8192;
        let b64 = fs::read("tests/data/transmitted_payload.txt").unwrap();

        let b64 = &b64[..length];

        // this test assumes host platform is little endian

        let mut buf = Vec::with_capacity(3072 * 2);
        buf.resize(3072 * 2, 0);
        let _data_bytes = base64::decode_config_slice(b64, base64::STANDARD, &mut buf).unwrap();
        let data_values = bytemuck::cast_slice::<_, half::f16>(&buf);

        assert_eq!(sent_data, data_values);
    }
}
