use crate::axl::{AxlPacket, AXL_OUTN};
use core::ops::{Deref, DerefMut};
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, Write};
use notecard::{NoteError, Notecard};

pub const BUOYSN: &'static str = const { option_env!("BUOYSN").unwrap_or("cain") };

/// Initialize sync when storage use is above this percentage.
pub const NOTECARD_STORAGE_INIT_SYNC: u32 = 50;

pub struct Notecarrier<I2C: Read + Write> {
    note: Notecard<I2C>,
}

impl<I2C: Read + Write> Notecarrier<I2C> {
    pub fn new(i2c: I2C, delay: &mut impl DelayMs<u16>) -> Result<Notecarrier<I2C>, NoteError> {
        let mut note = Notecard::new(i2c);
        note.initialize(delay)?;

        #[cfg(feature = "continuous")]
        note.card()
            .location_mode(
                delay,
                Some("continuous"),
                Some(360), // https://discuss.blues.io/t/gps-in-continuous-hub-mode/788/4
                None,
                None,
                None,
                None,
                None,
                None,
            )?
            .wait(delay)?;

        note.hub()
            .set(
                delay,
                Some(env!("BUOYPR", "Specify notehub project")),
                None,
                if cfg!(feature = "continuous") {
                    Some(notecard::hub::req::HubMode::Continuous)
                } else {
                    Some(notecard::hub::req::HubMode::Periodic)
                },
                Some(&BUOYSN),
                Some(20), // max time between out-going sync in minutes.
                None,
                None,
                None,
                None,
                Some(false),
                None,
            )?
            .wait(delay)?;

        #[cfg(not(feature = "continuous"))]
        note.card()
            .location_mode(
                delay,
                Some("periodic"),
                Some(60),
                None,
                None,
                None,
                None,
                None,
                None,
            )?
            .wait(delay)?;

        note.card()
            .location_track(delay, true, true, false, Some(1), None)?
            .wait(delay)?;

        let mut n = Notecarrier { note };
        n.setup_templates(delay)?;

        defmt::info!("initializing initial sync (removing any penalties)..");
        n.note.hub().sync(delay, true)?.wait(delay)?;

        Ok(n)
    }

    /// Initiate sync and wait for it to complete (or time out).
    pub fn sync_and_wait(
        &mut self,
        delay: &mut impl DelayMs<u16>,
        timeout_ms: u16,
    ) -> Result<bool, NoteError> {
        defmt::info!("sync..");
        self.note.hub().sync(delay, true)?.wait(delay)?;

        for _ in 0..(timeout_ms / 1000) {
            delay.delay_ms(1000u16);
            defmt::debug!("querying sync status..");
            let status = self.note.hub().sync_status(delay)?.wait(delay);
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
            position_time: u32,
            offset: u32,
            length: u32,
            freq: f32,
            packet: u32,
            lon: f32,
            lat: f32,
        }

        let meta_template = AxlPacketMetaTemplate {
            timestamp: 18,
            position_time: 14,
            offset: 14,
            length: 14,
            freq: 14.1,
            packet: 12,
            lon: 18.1,
            lat: 18.1,
        };

        defmt::debug!("setting up template for AxlPacketMeta");
        self.note()
            .template(
                delay,
                Some("axl.qo"),
                Some(meta_template),
                Some(AXL_OUTN as u32),
            )?
            .wait(delay)?;

        Ok(())
    }

    pub fn send(
        &mut self,
        pck: &AxlPacket,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<usize, NoteError> {
        let b64 = pck.base64();

        for (pi, p) in b64.chunks(8 * 1024).enumerate() {
            let meta = AxlPacketMeta {
                timestamp: pck.timestamp,
                offset: pck.offset as u32,
                packet: pi as u32,
                length: p.len() as u32,
                freq: pck.freq,
                position_time: pck.position_time,
                lon: pck.lon,
                lat: pck.lat,
            };

            let r = self
                .note
                .note()
                .add(
                    delay,
                    Some("axl.qo"),
                    None,
                    Some(meta),
                    Some(core::str::from_utf8(p).unwrap()),
                    if cfg!(feature = "continuous") {
                        true
                    } else {
                        false
                    },
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

    /// Send log messages
    pub fn drain_log(
        &mut self,
        queue: &heapless::mpmc::Q16<heapless::String<256>>,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<(), NoteError> {
        while let Some(msg) = queue.dequeue() {
            defmt::info!("logging message: {}", msg);
            self.note
                .hub()
                .log(delay, msg.as_str(), false, false)?
                .wait(delay)?;
        }

        Ok(())
    }

    /// Send all available packages to the notecard.
    pub fn drain_queue(
        &mut self,
        queue: &mut heapless::spsc::Consumer<'static, AxlPacket, 32>,
        delay: &mut impl DelayMs<u16>,
    ) -> Result<usize, NoteError> {
        let mut sz = 0;

        while let Some(pck) = queue.peek() {
            #[cfg(not(feature = "continuous"))]
            {
                // let sync_status = self.note.hub().sync_status(delay)?.wait(delay)?;

                // if sync_status.requested.is_some() {
                //     defmt::warn!(
                //         "notecard is syncing, not sending any data-packages until done: queue sz: {}",
                //         queue.len()
                //     );
                //     return Ok(sz);
                // }
            }

            let status = self.note.card().status(delay)?.wait(delay)?;

            if status.storage > 75 {
                // wait until notecard has synced.
                defmt::warn!("notecard is more than 75% full, not adding more notes until sync is done: queue sz: {}", queue.len());
                return Ok(sz);
            }

            defmt::debug!("sending package: queue sz: {}", queue.len());

            sz += self.send(pck, delay).inspect_err(|e| {
                defmt::error!("Error while sending package to notecard: {:?}", e)
            })?;

            queue.dequeue();
        }

        Ok(sz)
    }

    /// Check if notecard is filling up, and initiate sync in that case.
    pub fn check_and_sync(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), NoteError> {
        let status = self.note.card().status(delay)?.wait(delay)?;
        defmt::trace!("card.status: {}", status);

        let sync_status = self.note.hub().sync_status(delay)?.wait(delay)?;
        defmt::trace!("hub.sync_status: {}", sync_status);

        #[cfg(debug_assertions)]
        {
            let wireless = self.note.card().wireless(delay).and_then(|r| r.wait(delay));
            defmt::trace!("card.wireless: {}", wireless);
        }

        if status.storage > NOTECARD_STORAGE_INIT_SYNC as usize {
            if sync_status.requested.is_none() {
                defmt::warn!(
                    "notecard is more than {}% full, initiating sync.",
                    NOTECARD_STORAGE_INIT_SYNC
                );
                self.note.hub().sync(delay, false)?.wait(delay)?;
            }
            defmt::info!(
                "notecard is filling up ({}%): sync status: {:?}",
                status.storage,
                sync_status
            );
        }

        Ok(())
    }
}

#[derive(serde::Serialize, Default)]
pub struct AxlPacketMeta {
    pub timestamp: i64,
    pub offset: u32,
    pub packet: u32,
    pub length: u32,
    pub freq: f32,
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
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
    use crate::axl::AXL_SZ;
    use half::f16;

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
