use ahrs_fusion::NxpFusion;
use ism330dhcx::{AccelValue, GyroValue};
use micromath::{vector::Vector3d, Quaternion};

use crate::axl::{AXL_SZ, SAMPLE_SZ};
#[cfg(feature = "fir")]
use crate::fir;

#[cfg(feature = "spectrum")]
use super::welch::Welch;

use super::wire::{ScaledF32, A16};

#[cfg(feature = "raw")]
use super::wire::G16;

// From Adafruit Sensors library.
// pub const SENSORS_RADS_TO_DPS: f64 = 57.29577793;
pub const SENSORS_DPS_TO_RADS: f64 = 0.017453293;
pub const SENSORS_GRAVITY_STANDARD: f64 = 9.80665;

#[cfg(feature = "fir")]
pub const RAW_AXL_SZ: usize = 2 * AXL_SZ * fir::DECIMATE as usize;

#[cfg(feature = "fir")]
pub const RAW_AXL_BYTE_SZ: usize = 2 * AXL_SZ * fir::DECIMATE as usize * 2;

#[cfg(not(feature = "fir"))]
pub const RAW_AXL_SZ: usize = 2 * AXL_SZ;

#[cfg(not(feature = "fir"))]
pub const RAW_AXL_BYTE_SZ: usize = 2 * AXL_SZ * 2;

pub type VecAxl = heapless::Vec<u16, AXL_SZ>;
pub type VecRawAxl = heapless::Vec<u16, RAW_AXL_SZ>;

#[cfg(feature = "raw")]
pub type AxlBufT = (VecAxl, VecRawAxl);

#[cfg(not(feature = "raw"))]
pub type AxlBufT = (VecAxl,);

#[derive(Debug, Clone, defmt::Format)]
pub enum Error {
    BufFull,
}

pub struct ImuBuf {
    #[cfg(feature = "fir")]
    fir: [fir::Decimator; SAMPLE_SZ],

    filter: NxpFusion,

    /// Buffer with values ready to be sent. Only `sample()` is allowed to grow the buf, and
    /// it must always grow with `SAMPLE_SZ` samples. The buf must also be a multiple of
    /// `SAMPLE_SZ`.
    pub axl: VecAxl,

    /// Buffer with raw values, is emptied whenever axl is emptied.
    #[cfg(feature = "raw")]
    pub raw_axl: VecRawAxl,

    #[cfg(feature = "spectrum")]
    pub welch: Welch,
}

impl ImuBuf {
    pub fn new(freq: f32) -> ImuBuf {
        #[cfg(feature = "fir")]
        let fir = [
            fir::FIR::new().into_decimator(),
            fir::FIR::new().into_decimator(),
            fir::FIR::new().into_decimator(),
        ];

        let filter = NxpFusion::new(freq);

        ImuBuf {
            #[cfg(feature = "fir")]
            fir,

            filter,
            axl: VecAxl::new(),

            #[cfg(feature = "raw")]
            raw_axl: VecRawAxl::new(),

            #[cfg(feature = "spectrum")]
            welch: Welch::new(),
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

        #[cfg(feature = "spectrum")]
        self.welch.reset();

        self.filter.reset();

        #[cfg(feature = "fir")]
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
    pub fn sample(&mut self, g: GyroValue, a: AccelValue) -> Result<(), Error> {
        if self.is_full() {
            return Err(Error::BufFull);
        }

        let g_rad = g.as_rad();
        let g_dps = g.as_dps();

        let a_m_ss = a.as_m_ss();
        let a_g = a.as_g();

        // Store raw values
        #[cfg(feature = "raw")]
        {
            self.raw_axl
                .extend(g_rad.iter().map(|g| G16::from_f32(*g as f32).to_u16()));
            self.raw_axl
                .extend(a_m_ss.iter().map(|a| A16::from_f32(*a as f32).to_u16()));
        }

        defmt::trace!("gyro: [{:?}]", g_rad);
        // Feed AHRS filter
        //
        // The filter takes gyro readings in degrees per second (dps) and accelerometer in (g) for
        // linear acceleration (can also take it in m/s^2 if not linear acceleration).
        self.filter.update(
            (g_dps[0]) as f32,
            (g_dps[1]) as f32,
            (g_dps[2]) as f32,
            (a_g[0]) as f32,
            (a_g[1]) as f32,
            (a_g[2]) as f32,
            0., // Ignore (uncalibrated) magnetometer. This does more harm than good, ref. Jeans buoy.
            0.,
            0.,
        );

        // Rotate the instantanuous acceleration into the NED reference frame using the
        // rotation from the Kalman filter.
        let q = self.filter.quaternion();
        let q = Quaternion::new(q[0], q[1], q[2], q[3]);
        let axl = Vector3d {
            x: a_m_ss[0] as f32,
            y: a_m_ss[1] as f32,
            z: a_m_ss[2] as f32,
        };
        let axl = q.rotate(axl);

        defmt::trace!("unrotated acl: [{:?}]", a);
        defmt::trace!("pushing acl: [{}, {}, {}]", axl.x, axl.y, axl.z);

        #[cfg(not(feature = "fir"))]
        {
            // x, y, z from axl is in m/s^2, the quaternion is only used to
            // rotate the instantanuous acceleration.
            self.axl.push(A16::from_f32(axl.x).to_u16()).unwrap();
            self.axl.push(A16::from_f32(axl.y).to_u16()).unwrap();
            self.axl
                .push(A16::from_f32(axl.z - SENSORS_GRAVITY_STANDARD as f32).to_u16())
                .unwrap();
        }

        // Filter and decimate the rotated acceleration.
        //
        // Removing the mean from the z-component should give better resolution.
        #[cfg(feature = "fir")]
        match (
            self.fir[0].decimate(axl.x),
            self.fir[1].decimate(axl.y),
            self.fir[2].decimate(axl.z - SENSORS_GRAVITY_STANDARD as f32),
        ) {
            (Some(x), Some(y), Some(z)) => {
                // x, y, z from axl is in m/s^2, the quaternion is only used to
                // rotate the instantanuous acceleration.
                self.axl.push(A16::from_f32(x).to_u16()).unwrap();
                self.axl.push(A16::from_f32(y).to_u16()).unwrap();
                self.axl.push(A16::from_f32(z).to_u16()).unwrap();

                #[cfg(feature = "spectrum")]
                self.welch.sample(z);
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
    #[cfg(feature = "fir")]
    #[test]
    fn filter_decimater() {
        use super::*;
        use crate::axl::SAMPLE_NO;
        use ism330dhcx::{ctrl1xl, ctrl2g};

        let mut buf = ImuBuf::new(200.);

        for _ in 0..SAMPLE_NO {
            buf.sample(
                GyroValue::new(ctrl2g::Fs::Dps500, [0, 1, 2]),
                AccelValue::new(ctrl1xl::Fs_Xl::G2, [0, 1, 2]),
            )
            .unwrap();
        }

        assert_eq!(
            buf.axl.len(),
            SAMPLE_SZ * SAMPLE_NO / fir::DECIMATE as usize
        );
        assert_eq!(
            buf.free(),
            (AXL_SZ / SAMPLE_SZ) - (SAMPLE_NO / fir::DECIMATE as usize)
        );
    }
}
