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

fn clean_up_collection(s: &mut Storage<Spi0, CS>) {
    defmt::info!("cleaning up test collection");
    s.remove_collection(0).ok();
    s.remove_collection(1).ok();
    s.set_id(0);
}

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};
    use embedded_hal::spi;
    use hal::spi::{Freq, Spi};
    use half::f16;
    use heapless::Vec;

    use sfy::axl::{AxlPacket, AXL_SZ, AXL_POSTCARD_SZ};
    use sfy::storage::Storage;

    struct State {
        // note: Notecarrier<hal::i2c::Iom2>,
        #[allow(unused)]
        delay: hal::delay::Delay,
        #[allow(unused)]
        rtc: hal::rtc::Rtc,

        storage: Storage<Spi0, CS>,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let rtc = hal::rtc::Rtc::new(dp.RTC, &mut dp.CLKGEN);
        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);

        defmt::info!("Setting up SPI");
        let spi = Spi::new(
            dp.IOM0,
            pins.d12,
            pins.d13,
            pins.d11,
            Freq::F100kHz,
            spi::MODE_0,
        );

        let cs = pins.a14.into_push_pull_output();
        let storage = Storage::open(spi, cs, sfy::storage::clock::CountClock(&COUNT), |spi| {
            spi.set_freq(Freq::F48mHz)
        })
        .unwrap();

        State {
            delay,
            rtc,
            storage,
        }
    }

    #[test]
    fn initialize_storage(s: &mut State) {
        defmt::info!("next id: {:?}", s.storage.next_id());
        assert_eq!(
            s.storage.next_id(),
            Some(0),
            "tests run on card with data"
        );
    }

    #[test]
    fn write_package(s: &mut State) {
        let mut p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.storage_id, Some(0));

        let mut p = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.storage_id, Some(1));

        clean_up_collection(&mut s.storage);
    }

    #[test]
    fn write_read_package(s: &mut State) {
        let mut p = AxlPacket {
            timestamp: 1002330,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p).unwrap();
        assert_eq!(p.storage_id, Some(0));
        assert_eq!(p.storage_version, Some(2));

        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p, p_read);

        let mut p1 = AxlPacket {
            timestamp: 1002400,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: None,
            data: (6..3078)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p1).unwrap();
        assert_eq!(p1.storage_id, Some(1));

        let mut p2 = AxlPacket {
            timestamp: 1002500,
            position_time: 123123,
            lat: 34.52341,
            lon: 54.012,
            freq: 53.0,
            offset: 15,
            storage_id: None,
            storage_version: None,
            data: (9..3081)
                .map(|v| f16::from_f32(v as f32))
                .collect::<Vec<_, { AXL_SZ }>>(),
        };

        s.storage.store(&mut p2).unwrap();
        assert_eq!(p2.storage_id, Some(2));

        // Do some random reads
        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p, p_read);

        let p_read = s.storage.get(2).unwrap();
        assert_eq!(p2, p_read);

        let p_read = s.storage.get(1).unwrap();
        assert_eq!(p1, p_read);

        let p_read = s.storage.get(0).unwrap();
        assert_eq!(p, p_read);

        clean_up_collection(&mut s.storage);
    }

    #[test]
    fn write_many_packages(s: &mut State) {
        for i in 0..150u32 {
            let mut p = AxlPacket {
                timestamp: 100 + i as i64,
                position_time: 123123,
                lat: 34.52341,
                lon: 54.012,
                freq: 53.0,
                offset: 15,
                storage_id: None,
                storage_version: None,
                data: (6..3078)
                    .map(|v| f16::from_f32(v as f32))
                    .collect::<Vec<_, { AXL_SZ }>>(),
            };

            s.storage.store(&mut p).unwrap();
            assert_eq!(p.storage_id, Some(i));

            let (c, fid, offset) = storage::id_to_parts(p.storage_id.unwrap());

            if i < 100 {
                assert_eq!(c, "0.2");
                assert_eq!(fid, i);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * i);
            } else {
                assert_eq!(c, "1.2");
                assert_eq!(fid, i - 100);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * (i - 100));
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
                storage_version: Some(2),
                data: (6..3078)
                    .map(|v| f16::from_f32(v as f32))
                    .collect::<Vec<_, { AXL_SZ }>>(),
            };

            let p_read = s.storage.get(i).unwrap();
            assert_eq!(p, p_read);

            let (c, fid, offset) = storage::id_to_parts(p.storage_id.unwrap());

            if i < 100 {
                assert_eq!(c, "0.2");
                assert_eq!(fid, i);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * i);
            } else {
                assert_eq!(c, "1.2");
                assert_eq!(fid, i - 100);
                assert_eq!(offset as u32, (AXL_POSTCARD_SZ as u32) * (i - 100));
            }
        }
        clean_up_collection(&mut s.storage);
    }
}
