use half::f16;
use ahrs_fusion::NxpFusion;
use micromath::{
    vector::Vector3d,
    Quaternion,
};

use crate::{
    axl::{AXL_SZ, SAMPLE_SZ},
    fir,
};

pub type VecAxl = heapless::Vec<f16, AXL_SZ>;

#[derive(Debug, Clone)]
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
        }
    }

    pub fn take_buf(&mut self) -> VecAxl {
        let b = self.axl.clone();
        self.axl.clear();

        b
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

        self.filter.update(
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

        let q = self.filter.quaternion();
        let q = Quaternion::new(q[0], q[1], q[2], q[3]);
        let axl = Vector3d {
            x: a[0] as f32,
            y: a[1] as f32,
            z: a[2] as f32,
        };
        let axl = q.rotate(axl);

        match (
            self.fir[0].decimate(axl.x),
            self.fir[1].decimate(axl.y),
            self.fir[2].decimate(axl.z),
        ) {
            (Some(x), Some(y), Some(z)) => {
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
    use super::*;

    #[test]
    fn filter_decimater() {
        let mut buf = ImuBuf::new(200.);

        for _ in 0..1024 {
            buf.sample([0., 1., 2.], [0., 1., 2.]).unwrap();
        }

        assert_eq!(buf.axl.len(), SAMPLE_SZ * 1024 / fir::DECIMATE as usize);
        assert_eq!(buf.free(), (AXL_SZ / SAMPLE_SZ) - (1024 / fir::DECIMATE as usize));
    }
}

