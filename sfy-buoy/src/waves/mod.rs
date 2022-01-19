//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use core::fmt::Debug;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Write, WriteRead},
};
use ism330dhcx::{ctrl1xl, ctrl2g, fifo, fifoctrl, Ism330Dhcx};

use crate::{axl::AxlPacket, fir};

mod buf;

use buf::ImuBuf;
pub use buf::VecAxl;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Freq {
    Hz26,
    Hz104,
    Hz208,
    Hz833,
}

impl Freq {
    pub fn value(&self) -> f32 {
        use Freq::*;

        match self {
            Hz26 => 26.,
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
    pub timestamp: i64,
    pub lon: f32,
    pub lat: f32,

    /// Offset in FIFO _in samples_ (that is one gyro and one accel sample) when timestamp
    /// was set.
    pub fifo_offset: u16,
}

impl<E: Debug, I2C: WriteRead<Error = E> + Write<Error = E>> Waves<I2C> {
    pub fn new(mut i2c: I2C) -> Result<Waves<I2C>, E> {
        defmt::debug!("setting up imu driver..");
        let imu = Ism330Dhcx::new_with_address(&mut i2c, 0x6a)?;

        let freq = Freq::Hz208;
        let output_freq = fir::OUT_FREQ;

        defmt::debug!("imu frequency: {}", freq.value());
        defmt::debug!("output frequency: {}", output_freq);

        let mut w = Waves {
            i2c,
            imu,
            freq,
            output_freq,
            buf: ImuBuf::new(freq.value()),
            timestamp: 0,
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

    /// Temperature in Celsius.
    pub fn get_temperature(&mut self) -> Result<f32, E> {
        self.imu.get_temperature(&mut self.i2c)
    }

    /// Booting the sensor accoring to Adafruit's driver
    fn boot_imu(&mut self) -> Result<(), E> {
        let sensor = &mut self.imu;
        let i2c = &mut self.i2c;

        // CTRL3_C
        sensor.ctrl3c.set_boot(i2c, true)?;
        sensor.ctrl3c.set_bdu(i2c, true)?;
        sensor.ctrl3c.set_if_inc(i2c, true)?;

        // CTRL9_XL
        sensor.ctrl9xl.set_den_x(i2c, true)?;
        sensor.ctrl9xl.set_den_y(i2c, true)?;
        sensor.ctrl9xl.set_den_z(i2c, true)?;
        sensor.ctrl9xl.set_device_conf(i2c, true)?;

        // CTRL1_XL
        sensor
            .ctrl1xl
            .set_accelerometer_data_rate(i2c, self.freq.accel_odr())?;

        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G4)?;
        sensor.ctrl1xl.set_lpf2_xl_en(i2c, true)?;

        // CTRL2_G
        sensor
            .ctrl2g
            .set_gyroscope_data_rate(i2c, self.freq.gyro_odr())?;

        sensor
            .ctrl2g
            .set_chain_full_scale(i2c, ctrl2g::Fs::Dps500)?;

        // CTRL7_G
        sensor.ctrl7g.set_g_hm_mode(i2c, true)?;

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
            .mode(&mut self.i2c, fifoctrl::FifoMode::Bypass)
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
    pub fn take_buf(&mut self, now: i64, lon: f32, lat: f32) -> Result<AxlPacket, E> {
        let pck = AxlPacket {
            timestamp: self.timestamp,
            offset: self.fifo_offset,
            data: self.buf.take_buf(),
            lon: self.lon,
            lat: self.lat,
            freq: self.freq.value(),
        };

        self.lon = lon;
        self.lat = lat;
        self.timestamp = now;
        self.fifo_offset = self.imu.fifostatus.diff_fifo(&mut self.i2c)? / 2;

        defmt::debug!(
            "cleared buffer: {}, new timestamp: {}, new offset: {}",
            pck.data.len(),
            self.timestamp,
            self.fifo_offset
        );

        Ok(pck)
    }

    pub fn is_full(&self) -> bool {
        self.buf.is_full()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn read_and_filter(&mut self) -> Result<(), E> {
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

            panic!("FIFO overrun.");
            // return Err(E::default());
        }

        let n = n / 2;

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
                _ => {
                    defmt::error!("Got non accel or gyro sample: {:?}, {:?}", m1, m2);
                    None
                }
            };

            if let Some((g, a)) = ga {
                self.buf.sample(g, a).unwrap();
            } else {
                // XXX: Fix and recover!
                //
                // - Reset FIFO and timestamps
                // - Reset IMU?
                panic!("Un-handled IMU FIFO error");
                // break; // we got something else than gyro or accel
            }
        }

        // Clear FIFO
        let nn = imu.fifostatus.diff_fifo(i2c)?;
        defmt::trace!("fifo length after read: {}", nn);

        // TODO: In case FIFO ran full it needs resetting.

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
