use super::buf::SENSORS_GRAVITY_STANDARD;

// Scaling of values before they are sent or stored.
pub const ACCEL_MAX: f32 = SENSORS_GRAVITY_STANDARD as f32 * 4.; // in g
pub const GYRO_MAX: f32 = 125. * 8.; // in DPS

/// An acceleration value packed into an u16 between pre-determined limits.
pub struct A16(u16);

impl A16 {
    pub fn from_f32(v: f32) -> A16 {
        A16(scale_f32_to_u16(ACCEL_MAX, v))
    }
}

pub struct G16(u16);

/// Move an f32 on the range -max to max to 0 to u16::MAX
fn scale_f32_to_u16(max: f32, v: f32) -> u16 {
    debug_assert!(max > 0.);
    let max = max as f64;
    let v = v as f64;
    // v should be in the range from [-max to max]
    let v = v + max; // v -> [0, 2*max]
    let u = v * u16::MAX as f64 / (2. * max); // v -> [0, u16::MAX]
    return u as u16;
}

/// Move an u16 on given -max to max range to its real value in f32.
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
        assert_eq!(scale_f32_to_u16(10., 0.), u16::MAX / 2);
        assert_eq!(scale_f32_to_u16(10., -10.), 0);

        assert_eq!(scale_u16_to_f32(10., u16::MAX), 10.);
        assert!((scale_u16_to_f32(10., u16::MAX / 2) - 0.).abs() < 0.001);
        assert_eq!(scale_u16_to_f32(10., 0), -10.);
    }

    #[test]
    fn round_trip_integers() {
        const max: i32 = 1000;
        for i in (-max..max) {
            let f = i as f32;
            let u = scale_f32_to_u16(max as f32, f);
            let fu = scale_u16_to_f32(max as f32, u);
            assert!((f - fu).abs() < 0.1);
        }
    }

    #[test]
    fn round_trip_accel() {
        for i in (0..1000) {
            let v = (i as f32) * ACCEL_MAX / 1000 as f32 - ACCEL_MAX;
            let u = scale_f32_to_u16(ACCEL_MAX, v);
            let fu = scale_u16_to_f32(ACCEL_MAX, u);
            assert!((v - fu).abs() < 0.01);
        }
    }
}

