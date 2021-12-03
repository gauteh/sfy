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
    use sfy::{note::Notecarrier, *};

    #[init]
    fn setup() -> Notecarrier {
        defmt::debug!("Setting up peripherals");
        let dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let i2c = hal::i2c::I2c::new(dp.IOM2, pins.d17, pins.d18);

        defmt::info!("Setting up notecarrier");
        Notecarrier::new(i2c)
    }

    #[test]
    fn ping_notecarrier(note: &mut Notecarrier) {
        assert_eq!(
            note.ping(),
            true,
            "notecarrier / notecard _not_ attached and responding!"
        );
    }

    #[test]
    fn get_gps_position(note: &mut Notecarrier) {
        let location = note.card().location().unwrap().wait().unwrap();
        defmt::info!("location: {:?}", location);

        assert!(location.lon.is_some());
    }

    #[test]
    fn get_periodic_gps_position(note: &mut Notecarrier) {
        let mode = note.card()
            .location_mode(Some("periodic"), Some(10), None, None, None, None, None, None)
            .unwrap()
            .wait()
            .unwrap();

        defmt::info!("mode: {:?}", mode);
        assert_eq!(mode.mode, "periodic");

        defmt::debug!("track: {:?}", note.card().location_track(true, false, true, Some(1), None).unwrap().wait());

        defmt::info!("retrieve current mode..");
        let mode = note.card()
            .location_mode(Some(""), None, None, None, None, None, None, None)
            .unwrap()
            .wait()
            .unwrap();
        defmt::info!("mode: {:?}", mode);
        assert_eq!(mode.mode, "periodic");

        let location = note.card().location().unwrap().wait().unwrap();
        defmt::info!("location: {:?}", location);

        // we might not have a positon, but the gps should be active.
        assert!(location.lat.is_some() || location.status.contains("{gps-active}"));
    }
}
