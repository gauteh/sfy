use ahrs_fusion::NxpFusion;
use half::f16;
use micromath::{vector::Vector3d, Quaternion};

use crate::{
    axl::{AXL_SZ, SAMPLE_SZ},
    fir,
};


// From Adafruit Sensors library.
pub const SENSORS_RADS_TO_DPS: f64 = 57.29577793;
pub const SENSORS_GRAVITY_STANDARD: f64 = 9.80665;

pub const RAW_AXL_SZ: usize = 2 * AXL_SZ * fir::DECIMATE as usize;
pub const RAW_AXL_BYTE_SZ: usize = 2 * AXL_SZ * fir::DECIMATE as usize * 2;

pub type VecAxl = heapless::Vec<f16, AXL_SZ>;
pub type VecRawAxl = heapless::Vec<f16, RAW_AXL_SZ>;


#[cfg(feature = "raw")]
pub type AxlBufT = (VecAxl, VecRawAxl);

#[cfg(not(feature = "raw"))]
pub type AxlBufT = (VecAxl,);

#[derive(Debug, Clone, defmt::Format)]
pub enum Error {
    BufFull,
}

pub struct ImuBuf {
    fir: [fir::Decimator; SAMPLE_SZ],
    filter: NxpFusion,

    /// Buffer with values ready to be sent. Only `sample()` is allowed to grow the buf, and
    /// it must always grow with `SAMPLE_SZ` samples. The buf must also be a multiple of
    /// `SAMPLE_SZ`.
    pub axl: VecAxl,

    /// Buffer with raw values, is emptied whenever axl is emptied.
    #[cfg(feature = "raw")]
    pub raw_axl: VecRawAxl,
}

impl ImuBuf {
    pub fn new(freq: f32) -> ImuBuf {
        let fir = [
            fir::FIR::new().into_decimator(),
            fir::FIR::new().into_decimator(),
            fir::FIR::new().into_decimator(),
        ];

        let filter = NxpFusion::new(freq);

        ImuBuf {
            fir,
            filter,
            axl: VecAxl::new(),

            #[cfg(feature = "raw")]
            raw_axl: VecRawAxl::new(),
        }
    }

    pub fn take_buf(&mut self) -> AxlBufT {
        let b = self.axl.clone();

        #[cfg(feature = "raw")]
        let r = self.raw_axl.clone();

        self.axl.clear();

        #[cfg(feature = "raw")]
        self.raw_axl.clear();

        #[cfg(feature = "raw")]
        return (b, r);

        #[cfg(not(feature = "raw"))]
        return (b,);
    }

    pub fn reset(&mut self) {
        self.axl.clear();

        #[cfg(feature = "raw")]
        self.raw_axl.clear();

        self.filter.reset();

        for f in &mut self.fir {
            f.reset();
        }
    }

    /// Free capacity in buf of full sample (`SAMPLE_SZ`).
    #[allow(dead_code)]
    pub fn free(&self) -> usize {
        (self.axl.capacity() - self.axl.len()) / SAMPLE_SZ
    }

    pub fn is_full(&self) -> bool {
        self.axl.is_full()
    }

    pub fn len(&self) -> usize {
        self.axl.len() / SAMPLE_SZ
    }

    pub fn capacity(&self) -> usize {
        self.axl.capacity() / SAMPLE_SZ
    }

    /// Sample a new value and filter through Kalman-filter and FIR-filters. Will grow
    /// buffer with `SAMPLE_SZ` samples.
    pub fn sample(&mut self, g: [f64; 3], a: [f64; 3]) -> Result<(), Error> {
        if self.is_full() {
            return Err(Error::BufFull);
        }

        // Store raw values
        #[cfg(feature = "raw")]
        {
            self.raw_axl.extend(g.iter().map(|g| f16::from_f64(*g)));
            self.raw_axl.extend(a.iter().map(|a| f16::from_f64(*a)));
        }

        // Feed AHRS filter
        //
        // The filter takes gyro readings in degrees per second (dps) and accelerometer in (g) for
        // linear acceleration (can also take it in m/s^2 if not linear acceleration).
        self.filter.update(
            (g[0] * SENSORS_RADS_TO_DPS) as f32,
            (g[1] * SENSORS_RADS_TO_DPS) as f32,
            (g[2] * SENSORS_RADS_TO_DPS) as f32,
            (a[0] / SENSORS_GRAVITY_STANDARD) as f32,
            (a[1] / SENSORS_GRAVITY_STANDARD) as f32,
            (a[2] / SENSORS_GRAVITY_STANDARD) as f32,
            0., // Ignore (uncalibrated) magnetometer. This does more harm than good, ref. Jeans buoy.
            0.,
            0.,
        );

        // Rotate the instantanuous acceleration into the NED reference frame using the
        // rotation from the Kalman filter.
        let q = self.filter.quaternion();
        let q = Quaternion::new(q[0], q[1], q[2], q[3]);
        let axl = Vector3d {
            x: a[0] as f32,
            y: a[1] as f32,
            z: a[2] as f32,
        };
        let axl = q.rotate(axl);

        // Filter and decimate the rotated acceleration.
        match (
            self.fir[0].decimate(axl.x),
            self.fir[1].decimate(axl.y),
            self.fir[2].decimate(axl.z),
        ) {
            (Some(x), Some(y), Some(z)) => {
                // x, y, z from axl is in m/s^2, the quaternion is only used to
                // rotate the instantanuous acceleration.
                self.axl.push(f16::from_f32(x)).unwrap();
                self.axl.push(f16::from_f32(y)).unwrap();
                self.axl.push(f16::from_f32(z)).unwrap();
            }
            (None, None, None) => {} // No filter output.
            _ => {
                unreachable!()
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::axl::SAMPLE_NO;

    use super::*;

    #[test]
    fn filter_decimater() {
        let mut buf = ImuBuf::new(200.);

        for _ in 0..SAMPLE_NO {
            buf.sample([0., 1., 2.], [0., 1., 2.]).unwrap();
        }

        assert_eq!(buf.axl.len(), SAMPLE_SZ * SAMPLE_NO / fir::DECIMATE as usize);
        assert_eq!(
            buf.free(),
            (AXL_SZ / SAMPLE_SZ) - (SAMPLE_NO / fir::DECIMATE as usize)
        );
    }
}
