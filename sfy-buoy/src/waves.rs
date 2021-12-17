//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use ahrs_fusion::NxpFusion;
use core::fmt::Debug;
use embedded_hal::blocking::{
    delay::DelayMs,
    i2c::{Write, WriteRead},
};
use ism330dhcx::{ctrl1xl, ctrl2g, fifo, fifoctrl, Ism330Dhcx};
use micromath::{Quaternion, vector::{Vector, Vector3d}};
use half::f16;

/// The installed IMU.
pub type IMU = Ism330Dhcx;

pub const SAMPLE_SZ: usize = 3;
pub const AXL_SZ: usize = SAMPLE_SZ * 1024;
pub type VecAxl = heapless::Vec<f16, AXL_SZ>;

pub struct Waves<I2C: WriteRead + Write> {
    pub i2c: I2C,
    pub imu: IMU,
    filter: NxpFusion,
    pub axl: VecAxl,
}

impl<E: Debug, I2C: WriteRead<Error = E> + Write<Error = E>> Waves<I2C> {
    pub fn new(mut i2c: I2C) -> Result<Waves<I2C>, E> {
        defmt::debug!("setting up imu driver..");
        let imu = Ism330Dhcx::new_with_address(&mut i2c, 0x6a)?;

        let mut w = Waves {
            i2c,
            imu,
            filter: NxpFusion::new(208.),
            axl: heapless::Vec::new(),
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
            .set_accelerometer_data_rate(i2c, ctrl1xl::Odr_Xl::Hz208)?;

        sensor
            .ctrl1xl
            .set_chain_full_scale(i2c, ctrl1xl::Fs_Xl::G4)?;
        sensor.ctrl1xl.set_lpf2_xl_en(i2c, true)?;

        // CTRL2_G
        sensor
            .ctrl2g
            .set_gyroscope_data_rate(i2c, ctrl2g::Odr::Hz208)?;

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
            .set_accelerometer_batch_data_rate(i2c, fifoctrl::BdrXl::Hz208)?;
        self.imu
            .fifoctrl
            .set_gyroscope_batch_data_rate(i2c, fifoctrl::BdrGy::Hz208)?;

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
    pub fn consume_fifo(&mut self) -> Result<impl ExactSizeIterator<Item = Result<fifo::Value, E>> + '_, E> {
        let n = self.imu.fifostatus.diff_fifo(&mut self.i2c)?;
        defmt::debug!("consuming {} samples from FIFO..", n);
        Ok((0..n).map(|_| self.imu.fifo_pop(&mut self.i2c)))
    }

    pub fn read_and_filter(&mut self) -> Result<(), E> {
        use fifo::Value;

        let n = self.imu.fifostatus.diff_fifo(&mut self.i2c)?;

        let i2c = &mut self.i2c;
        let imu = &mut self.imu;
        let filter = &mut self.filter;

        let n = n / 2;
        let n = n.max((AXL_SZ / SAMPLE_SZ) as u16);

        for _ in 0..n {
            let m1 = imu.fifo_pop(i2c)?;
            let m2 = imu.fifo_pop(i2c)?;

            let ga = match (m1, m2) {
                (Value::Gyro(g), Value::Accel(a)) => Some((g, a)),
                (Value::Accel(a), Value::Gyro(g)) => Some((g, a)),
                _ => None
            };

            if let Some((g, a)) = ga {
                filter.update(g[0] as f32, g[1] as f32, g[2] as f32, a[0] as f32, a[1] as f32, a[2] as f32, 0., 0., 0.);

                let q = filter.quaternion();
                let q = Quaternion::new(q[0], q[1], q[2], q[3]);
                let axl = Vector3d { x: a[0] as f32, y: a[1] as f32, z: a[2] as f32 };
                let axl = q.rotate(axl);

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
                break // we got something else than gyro or accel
            }
        }

        // TODO: In case FIFO ran full it needs resetting.
        // TODO: Set time of start for batch + current offset in FIFO where the timestamp points to.

        Ok(())
    }
}

/// Compresses the stream of f32's, the `buf` must be at least 4 times as long in `u8` as `values` is
/// in `f32`.
pub fn compress(values: &[f16], buf: &mut [u8]) -> Result<usize, lzss::LzssError<void::Void, lzss::SliceWriteError>> {
    use lzss::{Lzss, SliceReader, SliceWriter};
    type MyLzss = Lzss<10, 4, 0x20, { 1 << 10 }, { 2 << 10 }>;

    let values = bytemuck::cast_slice(values);

    debug_assert!(buf.len() >= values.len());

    MyLzss::compress(
        SliceReader::new(values),
        SliceWriter::new(buf)
    )
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

        println!("compressed from: {} to {} (ratio: {})", v.len() * 4, n, ratio);
    }
}
