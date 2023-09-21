use super::buf::{SENSORS_DPS_TO_RADS, SENSORS_GRAVITY_STANDARD};

/// Scaling of acceleration values before they are sent or stored.
///
/// > Do not change without updating the storage version.
pub const ACCEL_MAX: f32 = 2. * super::ACCEL_RANGE * SENSORS_GRAVITY_STANDARD as f32; // in m/s^2

/// Scaling of gyro values before they are sent or stored.
///
/// > Do not change without updating the storage version.
pub const GYRO_MAX: f32 = 2. * super::GYRO_RANGE * SENSORS_DPS_TO_RADS as f32; // in rad/s

/// An acceleration value packed into an u16 between pre-determined limits.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct A16(u16);

/// A gyro value packed into an u16 between pre-determined limits.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct G16(u16);

unsafe impl bytemuck::Zeroable for A16 {}
unsafe impl bytemuck::Zeroable for G16 {}
unsafe impl bytemuck::Pod for A16 {}
unsafe impl bytemuck::Pod for G16 {}

pub trait ScaledF32: Sized {
    const MAX: f32;

    fn from_u16(u: u16) -> Self;
    fn to_u16(&self) -> u16;

    fn from_f32(v: f32) -> Self {
        Self::from_u16(scale_f32_to_u16(Self::MAX, v))
    }

    fn to_f32(&self) -> f32 {
        scale_u16_to_f32(Self::MAX, self.to_u16())
    }
}

impl ScaledF32 for A16 {
    const MAX: f32 = ACCEL_MAX;

    fn from_u16(u: u16) -> Self {
        A16(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

impl ScaledF32 for G16 {
    const MAX: f32 = GYRO_MAX;

    fn from_u16(u: u16) -> Self {
        G16(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

/// Move an f32 on the range -max to max to 0 to u16::MAX
fn scale_f32_to_u16(max: f32, v: f32) -> u16 {
    debug_assert!(max > 0.);
    let max = max as f64;
    let v = v as f64;

    // clip to bounds.
    let v = v.min(max);
    let v = v.max(-max);

    // v should be in the range from [-max to max]
    let v = v + max; // v -> [0, 2*max]
    let u = v * u16::MAX as f64 / (2. * max); // v -> [0, u16::MAX]
    return libm::round(u) as u16; // will maybe panic if u is out-of-bounds?
}

/// Move an u16 on given -max to max range to its real value in f32.
#[allow(unused)]
fn scale_u16_to_f32(max: f32, u: u16) -> f32 {
    debug_assert!(max > 0.);
    let max = max as f64;
    let v = u as f64;
    let v = v * (2. * max) / u16::MAX as f64;
    let v = v - max;
    return v as f32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_limits() {
        assert_eq!(scale_f32_to_u16(10., 10.), u16::MAX);
        assert_eq!(scale_f32_to_u16(10., 0.), u16::MAX / 2 + 1);
        assert_eq!(scale_f32_to_u16(10., -10.), 0);

        assert_eq!(scale_u16_to_f32(10., u16::MAX), 10.);
        assert!((scale_u16_to_f32(10., u16::MAX / 2) - 0.).abs() < 0.001);
        assert_eq!(scale_u16_to_f32(10., 0), -10.);
    }

    #[test]
    fn round_trip_integers() {
        const MAX: i32 = 1000;
        for i in -MAX..MAX {
            let f = i as f32;
            let u = scale_f32_to_u16(MAX as f32, f);
            let fu = scale_u16_to_f32(MAX as f32, u);
            assert!((f - fu).abs() < 0.1);
        }
    }

    #[test]
    fn round_trip_accel() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * ACCEL_MAX / N as f32 - ACCEL_MAX;
            let u = scale_f32_to_u16(ACCEL_MAX, v);
            let fu = scale_u16_to_f32(ACCEL_MAX, u);

            let uu = A16::from_f32(v);
            let fuu = uu.to_f32();

            assert_eq!(u, uu.to_u16());
            assert_eq!(fu, fuu);

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("accel u16 avg diff: {}", avg);
        println!("accel u16 max diff: {}", max);

        assert!(max < 0.01);
    }

    #[test]
    fn round_trip_accel_half16() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * ACCEL_MAX / N as f32 - ACCEL_MAX;
            let hv = half::f16::from_f32(v);
            let fu: f32 = hv.to_f32();

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("accel half avg diff: {}", avg);
        println!("accel half max diff: {}", max);

        assert!(max < 0.01);
    }

    #[test]
    fn round_trip_gyro() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * GYRO_MAX / N as f32 - GYRO_MAX;
            let u = scale_f32_to_u16(GYRO_MAX, v);
            let fu = scale_u16_to_f32(GYRO_MAX, u);

            let uu = G16::from_f32(v);
            let fuu = uu.to_f32();

            assert_eq!(u, uu.to_u16());
            assert_eq!(fu, fuu);

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("gyro u16 avg diff: {}", avg);
        println!("gyro u16 max diff: {}", max);

        assert!(max < 0.01);
    }

    #[test]
    fn round_trip_gyro_half16() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * GYRO_MAX / N as f32 - GYRO_MAX;
            let hv = half::f16::from_f32(v);
            let fu: f32 = hv.to_f32();

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("gyro half avg diff: {}", avg);
        println!("gyro half max diff: {}", max);

        assert!(max < 0.01);
    }

    #[test]
    fn slice_as_u16s() {
        let k: [A16; 1024] = core::array::from_fn(|i| A16(i as u16));
        println!("{k:?}");
        let kp = &k as *const _ as *const u16;
        let u = unsafe { core::slice::from_raw_parts(kp, k.len()) };
        println!("{u:?}");

        let uk: [u16; 1024] = core::array::from_fn(|i| i as u16);
        assert_eq!(uk, u);
    }

    #[test]
    fn slice_as_u8() {
        let k: [A16; 1024] = core::array::from_fn(|i| A16(i as u16));
        let ub: &[u8] = bytemuck::cast_slice(&k);
        println!("{ub:?}");

        let kub: &[u16] = bytemuck::cast_slice(&ub);
        let uk: [u16; 1024] = core::array::from_fn(|i| i as u16);
        assert_eq!(&uk, kub);

        let kkub: &[A16] = bytemuck::cast_slice(&ub);
        assert_eq!(&k, kkub);
    }
}
