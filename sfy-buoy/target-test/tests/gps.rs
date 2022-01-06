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
    use sfy::note::Notecarrier;
    use chrono::{NaiveDateTime, NaiveDate};

    struct State {
        note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        rtc: hal::rtc::Rtc,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);

        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        let i2c = I2c::new(dp.IOM2, pins.d17, pins.d18, Freq::F100kHz);

        defmt::info!("Setting up notecarrier");
        let note = Notecarrier::new(i2c, &mut delay).unwrap();

        State { note, delay, rtc }
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
    fn gps_position(s: &mut State) {
        let location = s
            .note
            .card()
            .location()
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();
        defmt::info!("location: {:?}", location);

        assert!(location.lon.is_some());
    }

    #[test]
    fn set_rtc(s: &mut State) {
        s.rtc.enable();
        let before = s.rtc.now().timestamp_millis();
        defmt::info!("now: {}", before);

        let d = NaiveDate::from_ymd(2020, 1, 1).and_hms(0, 0, 0);
        s.rtc.set(d);
        let now = s.rtc.now().timestamp_millis();
        defmt::info!("after change: {}", now);
        assert_ne!(before, now);
        assert_eq!(d.timestamp(), s.rtc.now().timestamp());
    }

    #[test]
    fn set_rtc_from_gps(s: &mut State) {
        s.rtc.enable();
        let before = s.rtc.now().timestamp_millis();
        defmt::info!("now: {}", before);

        let tm = s
            .note
            .card()
            .time()
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();
        defmt::info!("time: {:?}", tm);

        if let Some(time) = tm.time {
            let d = NaiveDateTime::from_timestamp(time as i64, 0);
            assert_eq!(d.timestamp(), time as i64);

            s.rtc.set(d);
            let now = s.rtc.now().timestamp_millis();

            defmt::info!("after change: {}", now);
            assert_eq!(time as i64, s.rtc.now().timestamp());
        } else {
            defmt::error!("no time from gps, test skipped.");
        }
    }

    #[test]
    fn periodic_gps_position(s: &mut State) {
        let mode = s
            .note
            .card()
            .location_mode(
                Some("periodic"),
                Some(10),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();

        defmt::info!("mode: {:?}", mode);
        assert_eq!(mode.mode, "periodic");

        defmt::debug!(
            "track: {:?}",
            s.note
                .card()
                .location_track(true, false, true, Some(1), None)
                .unwrap()
                .wait(&mut s.delay)
        );

        defmt::info!("retrieve current mode..");
        let mode = s
            .note
            .card()
            .location_mode(Some(""), None, None, None, None, None, None, None)
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();
        defmt::info!("mode: {:?}", mode);
        assert_eq!(mode.mode, "periodic");

        let location = s
            .note
            .card()
            .location()
            .unwrap()
            .wait(&mut s.delay)
            .unwrap();
        defmt::info!("location: {:?}", location);

        // we might not have a positon, but the gps should be active.
        assert!(location.lat.is_some() || location.status.contains("{gps-active}"));
    }
}
