use core::convert::Infallible;
use core::fmt::Debug;
use core::marker::PhantomData;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::{InputPin, OutputPin};

use embedded_hal::serial::{Read, Write};
use heapless::Vec;
use one_wire_bus::{Address, BaudRate, OneWire, OneWireError};

use crate::waves::wire::ScaledF32;

const MAX_PROBES: usize = 8;

#[derive(Copy, Clone)]
enum TempsState {
    Ready(i64),    // Ready to sample: time since start of last sample
    Sampling(i64), // currently sampling, waiting for sample to be ready
    Failed(i64),   // failure, with timestamp (millis) since failure
}

pub struct Temps<U> {
    wire: OneWire<U>,
    pub probes: Vec<Probe, MAX_PROBES>,
    resolution: ds18b20::Resolution,
    state: TempsState,
}

pub struct Probe {
    pub address: Address,
    pub sensor: ds18b20::Ds18b20,
}

impl<U, E> Temps<U>
where
    U: Read<u8, Error = E> + Write<u8, Error = E>,
{
    /// Scan for devices, init and set up logging.
    ///
    /// Can be re-run to reset.
    pub fn new(
        w: U,
        set_baudrate: fn(&mut U, BaudRate) -> (),
        delay: &mut impl DelayUs<u16>,
    ) -> Result<Temps<U>, OneWireError<E>> {
        defmt::info!("setting up temperature sensors..");
        let mut wire = OneWire::new(w, set_baudrate)?;
        let resolution = ds18b20::Resolution::Bits12;
        let mut addresses = heapless::Vec::<_, MAX_PROBES>::new();

        defmt::info!("scanning for temperature probes..");

        for device in wire.devices(false, delay) {
            let device = device?;
            defmt::trace!("device: {:?}", device.0);
            if device.family_code() == ds18b20::FAMILY_CODE {
                defmt::info!(
                    "found device at: {:#x} with family code: {:#x}",
                    device.0,
                    device.family_code()
                );

                match addresses.push(device) {
                    Err(_) => {
                        defmt::error!("too many probes.");
                        break;
                    }
                    _ => {}
                };
            } else {
                defmt::warn!(
                    "found unknown device at: {:#x} with family code: {:#x}",
                    device.0,
                    device.family_code()
                );
            }
        }

        defmt::info!("found {} devices, configuring..", addresses.len());

        let mut probes = heapless::Vec::new();

        for addr in addresses {
            probes
                .push(Probe::new(addr, &mut wire, resolution, delay)?)
                .ok();
            // .unwrap(); // already checked size
        }

        Ok(Temps {
            wire,
            probes,
            resolution,
            state: TempsState::Ready(0),
        })
    }

    /// Initiate temp reading on all sensors and wait for required time. May be up to 750 ms at max
    /// resolution.
    ///
    /// XXX: need to make non-blocking
    pub fn read_all_temps(
        &mut self,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    ) -> Result<Vec<f32, MAX_PROBES>, OneWireError<E>> {
        let mut temps = Vec::new();

        if !self.probes.is_empty() {
            ds18b20::start_simultaneous_temp_measurement(&mut self.wire, delay)?;

            self.resolution.delay_for_measurement_time(delay); // XXX: would be nice to await this.
                                                               // probably need to do in non-block.

            for p in &self.probes {
                let data = p.sensor.read_data(&mut self.wire, delay)?;
                temps.push(data.temperature).unwrap(); // cannot be more probes than MAX_PROBES
            }
        }

        Ok(temps)
    }

    /// Non-blocking sampling
    pub fn sample(
        &mut self,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
        now: i64,
    ) -> Result<(), OneWireError<E>> {
        match self.state {
            TempsState::Ready(last) => {
                if now - last >= 1000 {
                    defmt::trace!("triggering simultaneous temp measurement..");
                    ds18b20::start_simultaneous_temp_measurement(&mut self.wire, delay)?;
                    self.state = TempsState::Sampling(now);
                } else if last > now {
                    // negative time jump
                    self.state = TempsState::Failed(now);
                }
            }
            TempsState::Sampling(start) => {
                if now - start >= self.resolution.max_measurement_time_millis().into() {
                    defmt::trace!("reading all temp measurements..");

                    for p in &self.probes {
                        let data = p.sensor.read_data(&mut self.wire, delay)?;
                        // temps.push(data.temperature).unwrap(); // cannot be more probes than MAX_PROBES
                        todo!("read");
                    }

                    self.state = TempsState::Ready(now);
                } else if start > now {
                    // negative time jump
                    self.state = TempsState::Failed(now);
                }
            }
            _ => todo!(),
        }

        Ok(())
    }
}

impl Probe {
    pub fn new<U, E>(
        address: Address,
        wire: &mut OneWire<U>,
        resolution: ds18b20::Resolution,
        delay: &mut impl DelayUs<u16>,
    ) -> Result<Probe, OneWireError<E>>
    where
        U: Read<u8, Error = E> + Write<u8, Error = E>,
    {
        let sensor = ds18b20::Ds18b20::new(address)?;

        // configure
        sensor.set_config(i8::MIN, i8::MAX, resolution, wire, delay)?;

        Ok(Probe { address, sensor })
    }
}

/// An temperature value packed into an u16 between pre-determined limits.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct T16(u16);

unsafe impl bytemuck::Zeroable for T16 {}
unsafe impl bytemuck::Pod for T16 {}

pub const TEMP_MAX: f32 = 125.; // in C
                                //
impl ScaledF32 for T16 {
    const MAX: f32 = TEMP_MAX; // celsius

    fn from_u16(u: u16) -> Self {
        T16(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_u16() {
        // 12 bits has accuracy 0.0625°C according to datasheet
        assert!((T16::from_f32(-3.).to_f32() - -3.).abs() < 0.001);
        assert!((T16::from_f32(-10.).to_f32() - -10.).abs() < 0.001);
        assert!((T16::from_f32(10.).to_f32() - 10.).abs() < 0.001);
        assert!((T16::from_f32(30.).to_f32() - 30.).abs() < 0.01);
    }
}
