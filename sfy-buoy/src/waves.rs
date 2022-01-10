//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use crate::note::AxlPacket;
use ahrs_fusion::NxpFusion;
use core::fmt::Debug;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Write, WriteRead},
};
use half::f16;
use ism330dhcx::{ctrl1xl, ctrl2g, fifo, fifoctrl, Ism330Dhcx};
use micromath::{
    vector::{Vector, Vector3d},
    Quaternion,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Freq {
    Hz26,
    Hz104,
    Hz208,
}

impl Freq {
    pub fn value(&self) -> f32 {
        use Freq::*;

        match self {
            Hz26 => 26.,
            Hz104 => 104.,
            Hz208 => 208.,
        }
    }

    pub fn gyro_odr(&self) -> ctrl2g::Odr {
        use ctrl2g::Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
        }
    }

    pub fn accel_odr(&self) -> ctrl1xl::Odr_Xl {
        use ctrl1xl::Odr_Xl as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
        }
    }

    pub fn accel_bdr(&self) -> fifoctrl::BdrXl {
        use fifoctrl::BdrXl as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
        }
    }

    pub fn gyro_bdr(&self) -> fifoctrl::BdrGy {
        use fifoctrl::BdrGy as Odr;
        use Freq::*;

        match self {
            Hz26 => Odr::Hz26,
            Hz104 => Odr::Hz104,
            Hz208 => Odr::Hz208,
        }
    }
}

/// The installed IMU.
pub type IMU = Ism330Dhcx;

pub const SAMPLE_SZ: usize = 3;
pub const AXL_SZ: usize = SAMPLE_SZ * 1024;
pub type VecAxl = heapless::Vec<f16, AXL_SZ>;

pub struct Waves<I2C: WriteRead + Write> {
    pub i2c: I2C,
    pub imu: IMU,
    pub freq: Freq,
    filter: NxpFusion,

    /// Buffer with values ready to be sent.
    pub axl: VecAxl,

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
        let freq = Freq::Hz26;
        defmt::debug!("imu frequency: {}", freq.value());

        let mut w = Waves {
            i2c,
            imu,
            freq,
            filter: NxpFusion::new(freq.value()),
            axl: heapless::Vec::new(),
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
            data: self.axl.clone(),
            lon: self.lon,
            lat: self.lat,
            freq: self.freq.value(),
        };

        self.axl.clear();

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

    pub fn read_and_filter(&mut self) -> Result<(), E> {
        use fifo::Value;

        let n = self.imu.fifostatus.diff_fifo(&mut self.i2c)?;

        let i2c = &mut self.i2c;
        let imu = &mut self.imu;
        let filter = &mut self.filter;

        let fifo_full = imu.fifostatus.full(i2c)?;
        let fifo_overrun = imu.fifostatus.overrun(i2c)?;
        let fifo_overrun_latched = imu.fifostatus.overrun_latched(i2c)?;
        defmt::trace!("reading {} (fifo_full: {}, overrun: {}, overrun_latched: {}) sample pairs (buffer: {}/{})", n, fifo_full, fifo_overrun, fifo_overrun_latched, self.axl.len(), AXL_SZ);

        // XXX: If any of these flags are true we need to reset the FIFO (and return an error from
        // this function), otherwise it will have stopped accumulating samples.
        if fifo_full || fifo_overrun || fifo_overrun_latched {
            defmt::error!("IMU fifo overrun: fifo sz: {}, (fifo_full: {}, overrun: {}, overrun_latched: {}) sample pairs (buffer: {}/{})", n, fifo_full, fifo_overrun, fifo_overrun_latched, self.axl.len(), AXL_SZ);

            panic!("FIFO overrun.");
            // return Err(E::default());
        }

        let n = n / 2;
        let n = n.min(((self.axl.capacity() - self.axl.len()) / 3) as u16);

        for _ in 0..n {
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
                filter.update(
                    g[0] as f32,
                    g[1] as f32,
                    g[2] as f32,
                    a[0] as f32,
                    a[1] as f32,
                    a[2] as f32,
                    0., // Ignore (uncalibrated) magnetometer. This does more harm than good, ref. Jeans buoy.
                    0.,
                    0.,
                );

                let q = filter.quaternion();
                let q = Quaternion::new(q[0], q[1], q[2], q[3]);
                let axl = Vector3d {
                    x: a[0] as f32,
                    y: a[1] as f32,
                    z: a[2] as f32,
                };
                let axl = q.rotate(axl);

                // XXX: use try_extend
                self.axl.extend(axl.iter().map(f16::from_f32));

                // XXX:
                //
                // Depending on experiment and duration there might be other values that should be
                // stored. E.g. aggregated / statistical values. In case of detected breaking
                // events we could send the event.
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

/// Compresses the stream of f32's, the `buf` must be at least 4 times as long in `u8` as `values` is
/// in `f32`.
pub fn compress(
    values: &[f16],
    buf: &mut [u8],
) -> Result<usize, lzss::LzssError<void::Void, lzss::SliceWriteError>> {
    use lzss::{Lzss, SliceReader, SliceWriter};
    type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;

    let values = bytemuck::cast_slice(values);

    debug_assert!(buf.len() >= values.len());

    MyLzss::compress(SliceReader::new(values), SliceWriter::new(buf))
}

pub enum Event {
    FifoAlmostFull,
    TimeSeriesReady,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_and_compress() {
        let mut v = VecAxl::new();

        for i in 0..3072 {
            v.push(f16::from_f32(i as f32)).unwrap();
        }

        let mut buf = [0u8; 1024 * 4 * 3];

        let n = compress(&v, &mut buf).unwrap();

        let ratio = (v.len() * 4) as f32 / (n as f32);

        println!(
            "compressed from: {} to {} (ratio: {})",
            v.len() * 4,
            n,
            ratio
        );
    }
}
