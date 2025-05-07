//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use core::fmt::Debug;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Write, WriteRead},
};
use ism330dhcx::{ctrl1xl, ctrl2g, fifo, fifoctrl, Ism330Dhcx};

#[cfg(feature = "fir")]
use static_assertions as sa;

use crate::{axl::AxlPacket, axl::VERSION};

#[cfg(feature = "fir")]
use crate::fir;

mod buf;
pub mod wire;

#[cfg(feature = "spectrum")]
pub mod welch;

use buf::ImuBuf;
pub use buf::{VecAxl, VecRawAxl, RAW_AXL_BYTE_SZ, RAW_AXL_SZ};

#[cfg(feature = "raw")]
pub type AxlPacketT = (AxlPacket, VecRawAxl);

#[cfg(not(feature = "raw"))]
pub type AxlPacketT = (AxlPacket,);

pub const FREQ: Freq = Freq::Hz208;

#[cfg(all(feature = "surf", feature = "ice"))]
compile_error!("only one of the surf and ice features should be enabled at the same time.");

// See discussion below in `boot`.
#[cfg(feature = "surf")]
pub const ACCEL_RANGE: f32 = 16.; // [g]
#[cfg(feature = "surf")]
pub const GYRO_RANGE: f32 = 1000.; // [dps]
                                   //
#[cfg(feature = "ice")]
pub const ACCEL_RANGE: f32 = 2.; // [g]
#[cfg(feature = "ice")]
pub const GYRO_RANGE: f32 = 125.; // [dps]
                                  //
#[cfg(all(not(feature = "surf"), not(feature = "ice")))]
pub const ACCEL_RANGE: f32 = 4.; // [g]
#[cfg(all(not(feature = "surf"), not(feature = "ice")))]
pub const GYRO_RANGE: f32 = 500.; // [dps]

#[cfg(all(feature = "20Hz", not(feature = "fir")))]
compile_error!("Feature 20Hz requires feature fir");

#[cfg(feature = "fir")]
pub const OUTPUT_FREQ: f32 = fir::OUT_FREQ;

#[cfg(not(feature = "fir"))]
pub const OUTPUT_FREQ: f32 = FREQ.value();

#[cfg(feature = "fir")]
sa::const_assert_eq!(FREQ.value(), fir::FREQ);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Freq {
    Hz26,
    Hz52,
    Hz104,
    Hz208,
    Hz833,
}

impl Freq {
    pub const fn value(&self) -> f32 {
        use Freq::*;

        match self {
            Hz26 => 26.,
            Hz52 => 52.,
            Hz104 => 104.,
            Hz208 => 208.,
            Hz833 => 833.,
        }
    }

    pub fn gyro_odr(&self) -> ctrl2g::Odr {
        use ctrl2g::Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz52 => Odr::Hz52,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
            Hz833 => Odr::Hz833,
        }
    }

    pub fn accel_odr(&self) -> ctrl1xl::Odr_Xl {
        use ctrl1xl::Odr_Xl as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz52 => Odr::Hz52,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
            Hz833 => Odr::Hz833,
        }
    }

    pub fn accel_bdr(&self) -> fifoctrl::BdrXl {
        use fifoctrl::BdrXl as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz52 => Odr::Hz52,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
            Hz833 => Odr::Hz833,
        }
    }

    pub fn gyro_bdr(&self) -> fifoctrl::BdrGy {
        use fifoctrl::BdrGy as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz52 => Odr::Hz52,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
            Hz833 => Odr::Hz833,
        }
    }
}

/// The installed IMU.
pub type IMU = Ism330Dhcx;

pub struct Waves<I2C: WriteRead + Write> {
    pub i2c: I2C,
    pub imu: IMU,
    pub freq: Freq,
    pub output_freq: f32,

    /// Buffer with values ready to be sent.
    buf: ImuBuf,

    /// Timestamp at `fifo_offset` sample in buffer.
    pub timestamp: i64, // [ms]
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
    pub temperature: f32,

    /// Offset in FIFO _in samples_ (that is one gyro and one accel sample) when timestamp
    /// was set.
    pub fifo_offset: u16,

    #[cfg(feature = "spectrum")]
    pub spectrum_timestamp: i64, // [ms]
    #[cfg(feature = "spectrum")]
    pub spectrum_fifo_offset: u16,
}

#[derive(Debug, defmt::Format)]
pub enum ImuError<E: Debug> {
    I2C(E),
    FifoOverrun {
        fifo_full: bool,
        overrun: bool,
        latched: bool,
        samples: u16,
        buffer: usize,
    },
    FifoBadSequence(fifo::Value, fifo::Value),
    TooFewSamples(i64),
}

impl<E: Debug> From<E> for ImuError<E> {
    fn from(e: E) -> ImuError<E> {
        ImuError::I2C(e)
    }
}

impl<E: Debug, I2C: WriteRead<Error = E> + Write<Error = E>> Waves<I2C> {
    pub fn new(mut i2c: I2C) -> Result<Waves<I2C>, E> {
        defmt::debug!("setting up imu driver..");
        let imu = Ism330Dhcx::new_with_address(&mut i2c, 0x6a)?;

        defmt::debug!("imu frequency: {}", FREQ.value());
        defmt::debug!("output frequency: {}", OUTPUT_FREQ);

        let mut w = Waves {
            i2c,
            imu,
            freq: FREQ,
            output_freq: OUTPUT_FREQ,
            buf: ImuBuf::new(FREQ.value(), OUTPUT_FREQ),
            timestamp: 0,
            position_time: 0,
            temperature: 0.0,
            lon: 0.0,
            lat: 0.0,
            fifo_offset: 0,

            #[cfg(feature = "spectrum")]
            spectrum_timestamp: 0,
            #[cfg(feature = "spectrum")]
            spectrum_fifo_offset: 0,
        };

        defmt::debug!("booting imu..");
        w.boot_imu()?;
        w.disable_fifo()?;

        // TODO: Turn off magnetometer.

        Ok(w)
    }

    pub fn ping(&mut self) -> bool {
        defmt::debug!("pinging imu..");
        self.i2c.write(0x6a, &[]).is_ok()
    }

    /// Attempt to reset and re-boot IMU.
    pub fn reset(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), E> {
        defmt::warn!("Attempting to reset IMU and filters.");
        self.disable_fifo()?;
        delay.delay_ms(1000u16);

        // Reboot IMU
        self.imu.ctrl3c.sw_reset(&mut self.i2c)?;
        delay.delay_ms(1000u16);

        self.imu = Ism330Dhcx::new_with_address(&mut self.i2c, 0x6a)?;

        self.buf.reset();

        // first batch is going to be off in timing.
        self.timestamp = 0;
        self.fifo_offset = 0;

        #[cfg(feature = "spectrum")]
        {
            self.spectrum_timestamp = 0;
            self.spectrum_fifo_offset = 0;
        }

        defmt::debug!("booting imu..");
        self.boot_imu()?;
        defmt::debug!("imu ready.");

        Ok(())
    }

    /// Temperature in Celsius.
    pub fn get_temperature(&mut self) -> Result<f32, E> {
        self.imu.get_temperature(&mut self.i2c)
    }

    /// Booting the sensor according to Adafruit's driver
    fn boot_imu(&mut self) -> Result<(), E> {
        let sensor = &mut self.imu;
        let i2c = &mut self.i2c;

        // CTRL3_C
        sensor.ctrl3c.set_boot(i2c, true)?;
        sensor.ctrl3c.set_bdu(i2c, true)?;
        sensor.ctrl3c.set_if_inc(i2c, true)?;

        // CTRL9_XL
        // sensor.ctrl9xl.set_den_x(i2c, true)?;
        // sensor.ctrl9xl.set_den_y(i2c, true)?;
        // sensor.ctrl9xl.set_den_z(i2c, true)?;
        // sensor.ctrl9xl.set_device_conf(i2c, true)?;

        // CTRL1_XL
        sensor
            .ctrl1xl
            .set_accelerometer_data_rate(i2c, self.freq.accel_odr())?;

        // # Acceleration range
        //
        // Acceleration range is measured to up to 8 and 14 g in breaking waves. But much less in
        // normal waves.
        //
        // Important: Make sure that the range here matches the range specified in the serialization of
        // acceleration values.
        //
        // Feddersen, F., Andre Amador, Kanoa Pick, A. Vizuet, Kaden Quinn, Eric Wolfinger, J. H. MacMahan, and Adam Fincham. “The Wavedrifter: A Low-Cost IMU-Based Lagrangian Drifter to Observe Steepening and Overturning of Surface Gravity Waves and the Transition to Turbulence.” Coastal Engineering Journal, July 26, 2023, 1–14. https://doi.org/10.1080/21664250.2023.2238949.
        //
        // Sinclair, Alexandra. “FlowRider: A Lagrangian Float to Measure 3-D Dynamics of Plunging Breakers in the Surf Zone.” Journal of Coastal Research 293 (January 2014): 205–9. https://doi.org/10.2112/JCOASTRES-D-13-00014.1.

        #[cfg(all(not(feature = "surf"), not(feature = "ice")))]
        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G4)?;

        #[cfg(feature = "surf")]
        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G16)?;

        #[cfg(feature = "ice")]
        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G2)?;

        defmt::info!(
            "accelerometer range: {} g",
            sensor.ctrl1xl.chain_full_scale()
        );
        assert_eq!(sensor.ctrl1xl.chain_full_scale().g(), ACCEL_RANGE);

        sensor.ctrl1xl.set_lpf2_xl_en(i2c, true)?; // Use LPF2 filtering (cannot be used at the
                                                   // same time as HP filter)
                                                   // XL_HM_MODE is enabled by default (CTRL6C)

        // Accelerometer High-Pass filter: At least 30 seconds, preferably the same
        // as the gyro-scope (16 mHz).
        //
        // 0.016 = 208 Hz / X => X = 208 / 0.016 = 13000. The lowest is ODR / 800 which is 3.86
        //   seconds. This is too high, so we cannot use the built-in HP-filter.
        // sensor.ctrl8xl.set_hpcf(i2c, ctrl8xl::HPCF_XL::)

        // CTRL2_G
        sensor
            .ctrl2g
            .set_gyroscope_data_rate(i2c, self.freq.gyro_odr())?;

        // # Angular velocity range
        //
        // The angular velocity was measured to maximum 1000 dps with buoys travelling through a
        // breaking wave, with very seldomly values measured above 600 dps. We set it to 1000 in
        // surf-experiments, and lower in open-water experiments. For deployments in very quiet
        // areas, it would probably be best to use the lowest possible range (i.e. 125 for our
        // sensor).
        //
        // Important: Make sure that the range here matches the range specified in the serialization of
        // gyro values.
        //
        // Feddersen, F., Andre Amador, Kanoa Pick, A. Vizuet, Kaden Quinn, Eric Wolfinger, J. H. MacMahan, and Adam Fincham. “The Wavedrifter: A Low-Cost IMU-Based Lagrangian Drifter to Observe Steepening and Overturning of Surface Gravity Waves and the Transition to Turbulence.” Coastal Engineering Journal, July 26, 2023, 1–14. https://doi.org/10.1080/21664250.2023.2238949.
        #[cfg(all(not(feature = "surf"), not(feature = "ice")))]
        sensor
            .ctrl2g
            .set_chain_full_scale(i2c, ctrl2g::Fs::Dps500)?;

        #[cfg(feature = "surf")]
        sensor
            .ctrl2g
            .set_chain_full_scale(i2c, ctrl2g::Fs::Dps1000)?;

        #[cfg(feature = "ice")]
        sensor
            .ctrl2g
            .set_chain_full_scale(i2c, ctrl2g::Fs::Dps125)?;

        defmt::info!("gyroscope range: {} dps", sensor.ctrl2g.chain_full_scale());
        assert_eq!(sensor.ctrl2g.chain_full_scale().dps(), GYRO_RANGE);

        // CTRL7_G
        sensor.ctrl7g.set_g_hm_mode(i2c, true)?; // high-res mode on gyro (default is already on)

        // High-pass filter for gyro
        // sensor.ctrl7g.set_hpm_g(i2c, ctrl7g::Hpm_g::Hpmg16)?; // HPF at 16mHz (62.5
        //                                                       // seconds)
        // sensor.ctrl7g.set_hp_en_g(i2c, true)?;

        // Both the gyro and accelerometer is low-pass filtered on-board:
        //
        // Gyro: LPF2 at 66.8 Hz when ODR = 208 Hz (not configurable)
        // Accel: default is ODR/2 => 104 Hz.

        Ok(())
    }

    pub fn enable_fifo(&mut self, delay: &mut impl DelayMs<u16>) -> Result<(), E> {
        defmt::debug!("enabling FIFO mode");

        let i2c = &mut self.i2c;

        // Reset FIFO
        self.imu.fifoctrl.mode(i2c, fifoctrl::FifoMode::Bypass)?;
        self.imu
            .fifoctrl
            .set_accelerometer_batch_data_rate(i2c, self.freq.accel_bdr())?;
        self.imu
            .fifoctrl
            .set_gyroscope_batch_data_rate(i2c, self.freq.gyro_bdr())?;

        // Wait for FIFO to be cleared.
        delay.delay_ms(10);

        // clear status bits.
        self.imu.fifostatus.full(i2c)?;
        self.imu.fifostatus.overrun(i2c)?;
        self.imu.fifostatus.overrun_latched(i2c)?; // XXX: only necessary on this one.

        // Start FIFO. The FIFO will fill up and stop if it is not emptied fast enough.
        self.imu.fifoctrl.mode(i2c, fifoctrl::FifoMode::FifoMode)?;

        Ok(())
    }

    /// Disable FIFO mode (this also resets the FIFO).
    pub fn disable_fifo(&mut self) -> Result<(), E> {
        self.imu
            .fifoctrl
            .mode(&mut self.i2c, fifoctrl::FifoMode::Bypass)?;

        // Read FIFO status register to clear.
        let _fifo_full = self.imu.fifostatus.full(&mut self.i2c)?;
        let _fifo_overrun = self.imu.fifostatus.overrun(&mut self.i2c)?;
        let _fifo_overrun_latched = self.imu.fifostatus.overrun_latched(&mut self.i2c)?;

        Ok(())
    }

    /// Returns iterator with all the currently available samples in the FIFO.
    pub fn consume_fifo(
        &mut self,
    ) -> Result<impl ExactSizeIterator<Item = Result<fifo::Value, E>> + '_, E> {
        let n = self.imu.fifostatus.diff_fifo(&mut self.i2c)?;
        defmt::debug!("consuming {} samples from FIFO..", n);
        Ok((0..n).map(|_| self.imu.fifo_pop(&mut self.i2c)))
    }

    /// Take buf and reset timestamp.
    pub fn take_buf(
        &mut self,
        now: i64,
        position_time: u32,
        lon: f64,
        lat: f64,
    ) -> Result<AxlPacketT, E> {
        defmt::trace!("axl: taking buffer");
        #[cfg(feature = "raw")]
        let (data, raw) = self.buf.take_buf();

        #[cfg(not(feature = "raw"))]
        let (data,) = self.buf.take_buf();

        let pck = AxlPacket {
            timestamp: self.timestamp,
            offset: self.fifo_offset,
            data,
            storage_id: None,
            storage_version: VERSION,
            position_time: self.position_time,
            temperature: self.temperature,
            lon: self.lon,
            lat: self.lat,
            freq: self.output_freq,
            accel_range: ACCEL_RANGE,
            gyro_range: GYRO_RANGE,
        };
        defmt::trace!("axl: buffer taken: {:?}", pck);

        self.lon = lon;
        self.lat = lat;
        self.timestamp = now;
        self.position_time = position_time;
        self.fifo_offset = self.imu.fifostatus.diff_fifo(&mut self.i2c)? / 2;
        self.temperature = self.get_temperature()?;

        defmt::debug!(
            "cleared buffer: {}, new timestamp: {}, new offset: {}",
            pck.data.len(),
            self.timestamp,
            self.fifo_offset
        );

        #[cfg(feature = "raw")]
        return Ok((pck, raw));

        #[cfg(not(feature = "raw"))]
        return Ok((pck,));
    }

    #[cfg(feature = "spectrum")]
    pub fn take_spectrum(&mut self, now: i64) -> Result<welch::WelchPacket, E> {
        defmt::debug!("axl: taking spectrum..");

        let time = self.spectrum_timestamp
            - (self.spectrum_fifo_offset as i64 * 1000 / self.freq.value() as i64) as i64;

        let spec = self.buf.welch.take_spectrum();
        self.spectrum_timestamp = now;
        self.spectrum_fifo_offset = self.imu.fifostatus.diff_fifo(&mut self.i2c)? / 2;

        let pck = welch::WelchPacket {
            timestamp: time,
            spec,
        };

        defmt::debug!(
            "axl: spectrum ready, timestamp: {}, new timestamp: {}, new offset: {}",
            time,
            self.spectrum_timestamp,
            self.spectrum_fifo_offset
        );

        Ok(pck)
    }

    pub fn is_full(&self) -> bool {
        self.buf.is_full()
    }

    #[cfg(feature = "spectrum")]
    pub fn is_spec_full(&self) -> bool {
        self.buf.is_spec_full()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Read and filter samples from IMU. Returns number of sample pairs consumed (at IMU
    /// frequency).
    pub fn read_and_filter(&mut self) -> Result<u32, ImuError<E>> {
        use fifo::Value;

        let n = self.imu.fifostatus.diff_fifo(&mut self.i2c)?;

        let i2c = &mut self.i2c;
        let imu = &mut self.imu;

        let fifo_full = imu.fifostatus.full(i2c)?;
        let fifo_overrun = imu.fifostatus.overrun(i2c)?;
        let fifo_overrun_latched = imu.fifostatus.overrun_latched(i2c)?;
        defmt::trace!("reading {} (fifo_full: {}, overrun: {}, overrun_latched: {}) sample pairs (buffer: {}/{})", n, fifo_full, fifo_overrun, fifo_overrun_latched, self.buf.len(), self.buf.capacity());

        // XXX: If any of these flags are true we need to reset the FIFO (and return an error from
        // this function), otherwise it will have stopped accumulating samples.
        if fifo_full || fifo_overrun || fifo_overrun_latched {
            defmt::error!("IMU fifo overrun: fifo sz: {}, (fifo_full: {}, overrun: {}, overrun_latched: {}) (buffer: {}/{})", n, fifo_full, fifo_overrun, fifo_overrun_latched, self.buf.len(), self.buf.capacity());

            return Err(ImuError::FifoOverrun {
                fifo_full,
                overrun: fifo_overrun,
                latched: fifo_overrun_latched,
                samples: n,
                buffer: self.buf.len(),
            });
        }

        let n = n / 2;

        let mut samples = 0;

        for _ in 0..n {
            #[cfg(feature = "spectrum")]
            if self.buf.is_spec_full() || self.buf.is_full() {
                defmt::debug!("axl or spec buf is full, waiting to be cleared..");
                break;
            }

            #[cfg(not(feature = "spectrum"))]
            if self.buf.is_full() {
                defmt::debug!("axl buf is full, waiting to be cleared..");
                break;
            }

            let m1 = imu.fifo_pop(i2c)?;
            let m2 = imu.fifo_pop(i2c)?;

            let ga = match (m1, m2) {
                (Value::Gyro(g), Value::Accel(a)) => Some((g, a)),
                (Value::Accel(a), Value::Gyro(g)) => Some((g, a)),
                _ => None,
            };

            if let Some((g, a)) = ga {
                self.buf.sample(g, a).unwrap();
            } else {
                defmt::error!("Bad sequence of samples in FIFO: {:?}, {:?}", m1, m2);
                return Err(ImuError::FifoBadSequence(m1, m2));
            }

            samples += 1;
        }

        let nn = imu.fifostatus.diff_fifo(i2c)?;
        defmt::trace!("fifo length after read: {}", nn);

        Ok(samples)
    }
}
