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
    use hal::i2c::{Freq, I2c};
    use hal::prelude::*;
    use sfy::note::Notecarrier;

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

        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let i2c = I2c::new(dp.IOM2, pins.d17, pins.d18, Freq::F100kHz);

        defmt::info!("Setting up notecarrier");
        let note = Notecarrier::new(i2c, &mut delay).unwrap();

        State { note, delay }
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
        let status = s
            .note
            .hub()
            .sync_status(&mut s.delay)
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();
        if status.requested.is_some() {
            defmt::panic!("sync already in progress: {:?}", status);
        }

        defmt::debug!("sending test log message to notehub..");
        s.note
            .hub()
            .log(&mut s.delay, "cain test starting up!", true, true)
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();

        defmt::debug!("initiate sync..");
        s.note
            .hub()
            .sync(&mut s.delay, false)
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();

        for _ in 0..30 {
            s.delay.delay_ms(1000u32);
            defmt::debug!("querying sync status..");
            let status = s
                .note
                .hub()
                .sync_status(&mut s.delay)
                .unwrap()
                .wait(&mut s.delay);
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
