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
    use sfy::note::{self, Notecarrier};
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
        let note = Notecarrier::new(i2c).unwrap();

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
    fn send_axl_batch(s: &mut State) {
        let pck = note::AxlPacket {
            timestamp: 1000,
            data: (0..3072).map(|v| half::f16::from_f32(v as f32)).collect::<heapless::Vec<_, { 3 * 1024 }>>()
        };

        assert!(pck.data.len() == sfy::waves::AXL_SZ);

        let r = s.note.send(pck).unwrap();
        defmt::debug!("package queued for sending: {:?}", r);

        defmt::debug!("triggering sync..");
        s.note.sync_and_wait(&mut s.delay, 60000).unwrap();
    }

    #[cfg(disabled)]
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

    #[cfg(disabled)]
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

