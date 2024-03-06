use core::fmt::Debug;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::digital::v2::{InputPin, OutputPin};

use heapless::Vec;
use one_wire_bus::{Address, OneWire, OneWireError};

const MAX_PROBES: usize = 8;

pub struct Temps<P: OutputPin<Error = E> + InputPin<Error = E>, E: Debug> {
    wire: OneWire<P>,
    probes: Vec<Probe, MAX_PROBES>,
    resolution: ds18b20::Resolution,
}

pub struct Probe {
    pub address: Address,
    pub sensor: ds18b20::Ds18b20,
}

impl<P: OutputPin<Error = E> + InputPin<Error = E>, E: Debug> Temps<P, E> {
    /// Scan for devices, init and set up logging.
    ///
    /// Can be re-run to reset.
    pub fn new(w: P, delay: &mut impl DelayUs<u16>) -> Result<Temps<P, E>, OneWireError<E>> {
        defmt::info!("setting up temperature sensors..");
        let mut wire = OneWire::new(w)?;

        defmt::info!("scanning for temperature probes..");

        let resolution = ds18b20::Resolution::Bits12;

        let mut addresses = heapless::Vec::<_, MAX_PROBES>::new();

        for device in wire.devices(false, delay) {
            let device = device?;
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

        let mut probes = heapless::Vec::new();

        for addr in addresses {
            probes
                .push(Probe::new(addr, &mut wire, resolution, delay)?)
                .map_err(|_| ()) // Probe doesn't impl Debug
                .unwrap(); // already checked size
        }

        Ok(Temps {
            wire,
            probes,
            resolution,
        })
    }

    pub fn read_all_temps(
        &mut self,
        delay: &mut (impl DelayUs<u16> + DelayMs<u16>),
    ) -> Result<Vec<f32, MAX_PROBES>, OneWireError<E>> {
        let mut temps = Vec::new();

        if !self.probes.is_empty() {
            ds18b20::start_simultaneous_temp_measurement(&mut self.wire, delay)?;

            self.resolution.delay_for_measurement_time(delay);

            for p in &self.probes {
                let data = p.sensor.read_data(&mut self.wire, delay)?;
                temps.push(data.temperature).unwrap(); // cannot be more probes than MAX_PROBES
            }
        }

        Ok(temps)
    }
}

impl Probe {
    pub fn new<P: OutputPin<Error = E> + InputPin<Error = E>, E: Debug>(
        address: Address,
        wire: &mut OneWire<P>,
        resolution: ds18b20::Resolution,
        delay: &mut impl DelayUs<u16>,
    ) -> Result<Probe, OneWireError<E>> {
        let sensor = ds18b20::Ds18b20::new(address)?;

        // configure
        sensor.set_config(i8::MIN, i8::MAX, resolution, wire, delay)?;

        Ok(Probe { address, sensor })
    }
}
