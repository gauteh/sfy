#![no_std]
#![no_main]

extern crate cmsis_dsp; // sinf, cosf, etc
use ambiq_hal as hal;
use defmt_rtt as _;
use panic_probe as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    use super::*;
    #[allow(unused)]
    use defmt::{assert, assert_eq, info};

    use hal::{
        i2c::{Freq, I2c},
        prelude::*,
    };

    use sfy::waves::Waves;

    struct State {
        // waves: Waves<hal::i2c::Iom4>,
        waves: Waves<hal::i2c::Iom3>,
        // waves: Waves<hal::i2c::Iom2>,
        delay: hal::delay::Delay,
    }

    #[init]
    fn setup() -> State {
        defmt::debug!("Setting up peripherals");
        let core = hal::pac::CorePeripherals::take().unwrap();
        let mut dp = hal::pac::Peripherals::take().unwrap();
        let pins = hal::gpio::Pins::new(dp.GPIO);

        let delay = hal::delay::Delay::new(core.SYST, &mut dp.CLKGEN);
        // let i2c = I2c::new(dp.IOM2, pins.d17, pins.d18, Freq::F100kHz);
        let i2c = I2c::new(dp.IOM3, pins.d6, pins.d7, Freq::F1mHz);
        // let i2c = I2c::new(dp.IOM4, pins.d10, pins.d9, Freq::F1mHz);

        defmt::info!("Setting up wave sensor");
        let waves = Waves::new(i2c).unwrap();

        State { waves, delay }
    }

    #[test]
    fn get_temperature(s: &mut State) {
        let temp = s.waves.get_temperature();
        defmt::info!("temperature: {:?}", temp);
    }

    #[test]
    fn fifo_accel_gyro(s: &mut State) {
        s.waves.disable_fifo().unwrap();
        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        defmt::debug!("samples: {}", samples);
        assert_eq!(samples, 0);

        s.waves.enable_fifo(&mut s.delay).unwrap();

        defmt::debug!("wait for some samples to accumulate..");
        s.delay.delay_ms(1500u16);

        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        defmt::debug!("the FIFO should now be full: samples: {}", samples);
        assert_eq!(samples, 512);

        assert_eq!(s.waves.imu.fifostatus.full(&mut s.waves.i2c).unwrap(), true);

        s.waves.disable_fifo().unwrap();
        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        defmt::debug!("the FIFO should now be empty: samples: {}", samples);
        assert_eq!(samples, 0);
        assert_eq!(
            s.waves.imu.fifostatus.full(&mut s.waves.i2c).unwrap(),
            false
        );
    }

    #[test]
    fn empty_fifo(s: &mut State) {
        s.waves.disable_fifo().unwrap();
        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        assert_eq!(samples, 0);

        s.waves.enable_fifo(&mut s.delay).unwrap();

        defmt::debug!("wait for some samples..");
        s.delay.delay_ms(800u16);

        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        assert!(samples > 100);

        defmt::debug!("attempting to empty FIFO.. {}", samples);
        s.waves.consume_fifo().unwrap().for_each(drop);

        let samples2 = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        defmt::debug!("FIFO: {}", samples2);
        assert!(samples2 < samples);
    }

    #[test]
    fn fifo_pull_batches(s: &mut State) {
        s.waves.disable_fifo().unwrap();
        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        assert_eq!(samples, 0);

        s.waves.enable_fifo(&mut s.delay).unwrap();

        defmt::debug!("wait for some samples..");
        s.delay.delay_ms(800u16);

        let n = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();

        let samples = s
            .waves
            .consume_fifo()
            .unwrap()
            .collect::<Result<heapless::Vec<_, 512>, _>>()
            .unwrap();
        defmt::debug!("collected {} values", samples.len());
        assert!(samples.len() > 100);

        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        defmt::debug!("values in FIFO after collection: {}", samples);
        assert!(samples < n);
    }

    #[test]
    fn fifo_sample_sequence(s: &mut State) {
        use ism330dhcx::fifo;

        s.waves.disable_fifo().unwrap();
        let samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        assert_eq!(samples, 0);

        s.waves.enable_fifo(&mut s.delay).unwrap();

        defmt::debug!("wait for some samples..");
        s.delay.delay_ms(800u16);

        let samples = s
            .waves
            .consume_fifo()
            .unwrap()
            .collect::<Result<heapless::Vec<_, 512>, _>>()
            .unwrap();
        defmt::debug!("collected {} values", samples.len());
        assert!(samples.len() > 100);

        let mut last = samples[0];

        for i in samples.iter().skip(1) {
            match i {
                fifo::Value::Accel(_) => assert!(matches!(last, fifo::Value::Gyro(_))),
                fifo::Value::Gyro(_) => assert!(matches!(last, fifo::Value::Accel(_))),
                _ => panic!(),
            };

            last = *i;
        }
    }

    #[test]
    fn read_and_filter(s: &mut State) {
        s.waves.disable_fifo().unwrap();
        let mut samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
        assert_eq!(samples, 0);

        let _p = s.waves.take_buf(100031231, 1231231, 34.0, 23.2).unwrap();

        s.waves.enable_fifo(&mut s.delay).unwrap();

        while !s.waves.is_full()  {
            defmt::debug!("wait for some samples..");
            s.delay.delay_ms(200u16);

            samples = s.waves.imu.fifostatus.diff_fifo(&mut s.waves.i2c).unwrap();
            defmt::debug!("values in FIFO before collecting: {}", samples);

            defmt::debug!("read and filter..");
            s.waves.read_and_filter().unwrap();

            defmt::debug!("buffer: {}", s.waves.len());
        }

        defmt::debug!("time series len: {}", s.waves.len());

        defmt::debug!("taking buf..");
        let p = s.waves.take_buf(100031231, 1231231, 34.0, 23.2).unwrap();
        defmt::debug!("pck: {:?}", p);

    }
}
