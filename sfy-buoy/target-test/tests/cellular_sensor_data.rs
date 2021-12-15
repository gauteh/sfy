#![no_std]
#![no_main]

use ambiq_hal as hal;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};
    use serde::{Deserialize, Serialize};
    use sfy::note::Notecarrier;
    use hal::i2c::{I2c, Freq};

    struct State {
        note: Notecarrier<hal::i2c::Iom2>,
        delay: hal::delay::Delay,
    }

    #[derive(Deserialize, Serialize, defmt::Format, Default)]
    struct Measurements {
        t0: u32,
        v: heapless::Vec<f32, 100>
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let i2c = I2c::new(dp.IOM2, pins.d17, pins.d18, Freq::F100kHz);

        defmt::info!("Setting up notecarrier");
        let note = Notecarrier::new(i2c);

        State {
            note,
            delay
        }
    }

    #[test]
    fn ping_notecarrier(s: &mut State) {
        assert_eq!(
            s.note.ping(),
            true,
            "notecarrier / notecard _not_ attached and responding!"
        );
    }

    #[test]
    fn send_single_measurement(s: &mut State) {
        let m = Measurements {
            t0: 100,
            v: heapless::Vec::from_slice(&[1.0, 3.0, 4.0]).unwrap()
        };

        defmt::info!("adding measurements to sensor.db");
        s.note.note().add(Some("sensor.db"), Some("?"), Some(m), None, true).unwrap().wait().unwrap();

        assert_eq!(s.note.sync_and_wait(&mut s.delay, 60000).unwrap(), true);
    }

    #[test]
    fn send_multiple_measurements(s: &mut State) {
        let m1 = Measurements {
            t0: 200,
            v: heapless::Vec::from_slice(&[2.0, 6.0, 4.0]).unwrap()
        };

        let m2 = Measurements {
            t0: 300,
            v: heapless::Vec::from_slice(&[7.0, 6.0, 4.0]).unwrap()
        };

        defmt::info!("adding measurements to sensor.db");
        s.note.note().add(Some("sensor.db"), Some("?"), Some(m1), None, true).unwrap().wait().unwrap();
        s.note.note().add(Some("sensor.db"), Some("?"), Some(m2), None, true).unwrap().wait().unwrap();

        assert_eq!(s.note.sync_and_wait(&mut s.delay, 60000).unwrap(), true);
    }
}

