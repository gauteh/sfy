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
    use sfy::note::Notecarrier;
    use hal::prelude::*;

    struct State {
        note: Notecarrier<hal::i2c::Iom2>,
        delay: hal::delay::Delay,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let i2c = hal::i2c::I2c::new(dp.IOM2, pins.d17, pins.d18);

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
    fn log_and_sync(s: &mut State) {
        defmt::debug!("sending test log message to notehub..");
        s.note.hub().log("cain test starting up!", true, true).unwrap().wait().unwrap();

        defmt::debug!("initiate sync..");
        s.note.hub().sync().unwrap().wait().unwrap();

        for _ in 0..30 {
            s.delay.delay_ms(1000u32);
            defmt::debug!("querying sync status..");
            let status = s.note.hub().sync_status().unwrap().wait();
            defmt::debug!("status: {:?}", status);

            if let Ok(status) = status {
                if status.completed.is_some() {
                    defmt::info!("successful sync.");
                    return;
                }
            }
        }

        panic!("sync didn't complete within timeout");
    }
}
