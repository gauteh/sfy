#![no_std]
#![no_main]

extern crate cmsis_dsp; // sinf, cosf, etc
use ambiq_hal as hal;
use core::sync::atomic::AtomicI32;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

use sfy::storage::{self, Storage};

pub static COUNT: AtomicI32 = AtomicI32::new(0);

type Spi0 = hal::spi::Spi0;
type CS = hal::gpio::pin::P35<{ hal::gpio::Mode::Output }>;
type DL = hal::delay::FlashDelay;

fn clean_up_collection(s: &mut Storage<Spi0, CS, DL>) {
    defmt::info!("cleaning up test collection");
    s.acquire().unwrap().remove_collection(0).ok();
    s.acquire().unwrap().remove_collection(1).ok();
    s.deinit();
}

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};
    use embedded_hal::{prelude::*, spi};
    use hal::spi::{Freq, Spi};
    use heapless::Vec;

    use sfy::axl::{AxlPacket, AXL_POSTCARD_SZ, AXL_SZ, VERSION};
    use sfy::storage::{SdSpiSpeed, Storage};

    struct State {
        // note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        #[allow(unused)]
        rtc: hal::rtc::Rtc,

        storage: Storage<Spi0, CS, DL>,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
        let mut delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        let cs = pins.a14;
        let mut cs = cs.into_push_pull_output();
        cs.internal_pull_up(true);
        // cs.set_high();

        delay.delay_ms(300_u32);

        defmt::info!("Setting up SPI");
        let spi = Spi::new(
            dp.IOM0,
            pins.d12,
            pins.d13,
            pins.d11,
            Freq::F100kHz,
            spi::MODE_0,
        );

        delay.delay_ms(300_u32);

        let mut storage = Storage::open(
            spi,
            cs,
            sfy::storage::clock::CountClock(&COUNT),
            |spi, speed| match speed {
                SdSpiSpeed::Low => spi.set_freq(Freq::F100kHz),
                SdSpiSpeed::High => spi.set_freq(Freq::F48mHz),
            },
            hal::delay::FlashDelay,
        );

        // clean up previous tests
        assert_eq!(sfy::storage::STORAGE_VERSION_STR, "t");
        clean_up_collection(&mut storage);

        State {
            delay,
            rtc,
            storage,
        }
    }

    #[test]
    fn initialize_storage(s: &mut State) {
        s.storage.acquire().unwrap();
        defmt::info!("next id: {:?}", s.storage.next_id());
        assert_eq!(s.storage.next_id(), Some(0), "tests run on card with data");
    }

    #[test]
    fn write_package(s: &mut State) {
        let p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: VERSION,
            temperature: 0.0,
            accel_range: 4.0,
            gyro_range: 500.0,
            data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
        };

        let mut p = (p,);

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.0.storage_id, Some(0));

        let p = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: VERSION,
            temperature: 0.0,
            accel_range: 4.0,
            gyro_range: 500.0,
            data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
        };

        let mut p = (p,);
        s.storage.store(&mut p).unwrap();
        assert_eq!(p.0.storage_id, Some(1));

        clean_up_collection(&mut s.storage);
    }

    #[test]
    fn write_read_package(s: &mut State) {
        let p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: VERSION,
            temperature: 0.0,
            accel_range: 4.0,
            gyro_range: 500.0,
            data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
        };

        let mut p = (p,);
        s.storage.store(&mut p).unwrap();
        assert_eq!(p.0.storage_id, Some(0));
        assert_eq!(p.0.storage_version, VERSION);

        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p.0, p_read);

        let p1 = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: VERSION,
            temperature: 0.0,
            accel_range: 4.0,
            gyro_range: 500.0,
            data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
        };

        let mut p1 = (p1,);
        s.storage.store(&mut p1).unwrap();
        assert_eq!(p1.0.storage_id, Some(1));

        let p2 = AxlPacket {
            timestamp: 1002500,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: VERSION,
            temperature: 0.0,
            accel_range: 4.0,
            gyro_range: 500.0,
            data: (9..3081).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
        };

        let mut p2 = (p2,);
        s.storage.store(&mut p2).unwrap();
        assert_eq!(p2.0.storage_id, Some(2));

        // Do some random reads
        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p.0, p_read);

        let p_read = s.storage.get(2).unwrap();
        assert_eq!(p2.0, p_read);

        let p_read = s.storage.get(1).unwrap();
        assert_eq!(p1.0, p_read);

        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p.0, p_read);

        clean_up_collection(&mut s.storage);
    }

    #[test]
    fn write_many_packages(s: &mut State) {
        for i in 0..1050u32 {
            let p = AxlPacket {
                timestamp: 100 + i as i64,
                position_time: 123123,
                lat: 34.52341,
                lon: 54.012,
                freq: 53.0,
                offset: 15,
                storage_id: None,
                storage_version: VERSION,
                temperature: 0.0,
                accel_range: 4.0,
                gyro_range: 500.0,
                data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
            };

            let mut p = (p,);
            s.storage.store(&mut p).unwrap();
            assert_eq!(p.0.storage_id, Some(i));

            let (c, fid, offset) = storage::id_to_parts(p.0.storage_id.unwrap());

            if i < 1000 {
                assert_eq!(c, "0.t");
                assert_eq!(fid, i);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * i);
            } else {
                assert_eq!(c, "1.t");
                assert_eq!(fid, i - 1000);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * (i - 1000));
            }
        }

        for i in 0..150u32 {
            let p = AxlPacket {
                timestamp: 100 + i as i64,
                position_time: 123123,
                lat: 34.52341,
                lon: 54.012,
                freq: 53.0,
                offset: 15,
                storage_id: Some(i),
                storage_version: VERSION,
                temperature: 0.0,
                accel_range: 4.0,
                gyro_range: 500.0,
                data: (6..3078).map(|v| v as u16).collect::<Vec<_, { AXL_SZ }>>(),
            };

            let p_read = s.storage.get(i).unwrap();
            assert_eq!(p, p_read);

            let (c, fid, offset) = storage::id_to_parts(p.storage_id.unwrap());

            if i < 1000 {
                assert_eq!(c, "0.t");
                assert_eq!(fid, i);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * i);
            } else {
                assert_eq!(c, "1.t");
                assert_eq!(fid, i - 1000);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * (i - 1000));
            }
        }
        clean_up_collection(&mut s.storage);
    }
}
