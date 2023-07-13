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
mod wire;

use buf::ImuBuf;
pub use buf::{VecAxl, VecRawAxl, RAW_AXL_BYTE_SZ, RAW_AXL_SZ};

#[cfg(feature = "raw")]
pub type AxlPacketT = (AxlPacket, VecRawAxl);

#[cfg(not(feature = "raw"))]
pub type AxlPacketT = (AxlPacket,);

pub const FREQ: Freq = Freq::Hz833; /////////////////////////////////

#[cfg(all(feature = "20Hz", not(feature = "fir")))]
compile_error!("Feature 20Hz requires feature fir");

#[cfg(feature = "fir")]
pub const OUTPUT_FREQ: f32 = fir::OUT_FREQ;

#[cfg(not(feature = "fir"))]
pub const OUTPUT_FREQ: f32 = FREQ.value();

#[cfg(feature = "fir")]
sa::const_assert_eq!(FREQ.value(), fir::FREQ); ////// make sure the two values are equal at compile time

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Freq {
    Hz26,
    Hz52,
    Hz104,
    Hz208,
    Hz417,
    Hz833,
    Hz1667,
    Hz3333,
    Hz6667,
}

impl Freq {
    pub const fn value(&self) -> f32 {
        use Freq::*;

        match self {
            Hz26 => 26.,
            Hz52 => 52.,
            Hz104 => 104.,
            Hz208 => 208.,
            Hz417 => 417.,
            Hz833 => 833.,
            Hz1667 => 1667.,
            Hz3333 => 3333.,
            Hz6667 => 6667.,
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
            Hz417 => Odr::Hz417,
            Hz833 => Odr::Hz833,
            Hz1667 => Odr::Hz1667,
            Hz3333 => Odr::Hz3333,
            Hz6667 => Odr::Hz6667,
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
            Hz417 => Odr::Hz417,
            Hz833 => Odr::Hz833,
            Hz1667 => Odr::Hz1667,
            Hz3333 => Odr::Hz3333,
            Hz6667 => Odr::Hz6667,
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
            Hz417 => Odr::Hz417,
            Hz833 => Odr::Hz833,
            Hz1667 => Odr::Hz1667,
            Hz3333 => Odr::Hz3333,
            Hz6667 => Odr::Hz6667,
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
            Hz417 => Odr::Hz417,
            Hz833 => Odr::Hz833,
            Hz1667 => Odr::Hz1667,
            Hz3333 => Odr::Hz3333,
            Hz6667 => Odr::Hz6667,
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
    pub timestamp: i64,
    pub position_time: u32,
    pub lon: f64,
    pub lat: f64,
    pub temperature: f32,

    /// Offset in FIFO _in samples_ (that is one gyro and one accel sample) when timestamp
    /// was set.
    pub fifo_offset: u16,
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

        defmt::debug!("imu frequency: {}", FREQ.value()); ///////////////////
        defmt::debug!("output frequency: {}", OUTPUT_FREQ); ////////////////////

        let mut w = Waves {
            i2c,
            imu,
            freq: FREQ,
            output_freq: OUTPUT_FREQ,
            buf: ImuBuf::new(FREQ.value()),
            timestamp: 0,
            position_time: 0,
            temperature: 0.0,
            lon: 0.0,
            lat: 0.0,
            fifo_offset: 0,
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
            .set_accelerometer_data_rate(i2c, self.freq.accel_odr())?; /////////////////

        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G2)?;
        sensor.ctrl1xl.set_lpf2_xl_en(i2c, true)?; // high-res mode on accelerometer.

        // CTRL2_G
        sensor
            .ctrl2g
            .set_gyroscope_data_rate(i2c, self.freq.gyro_odr())?; //////////////

        sensor
            .ctrl2g
            .set_chain_full_scale(i2c, ctrl2g::Fs::Dps125)?;

        // CTRL7_G
        sensor.ctrl7g.set_g_hm_mode(i2c, true)?; // high-res mode on gyro

        sensor.fifoctrl.compression(i2c, true)?; // Enable compression


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

    pub fn is_full(&self) -> bool {
        self.buf.is_full()
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
