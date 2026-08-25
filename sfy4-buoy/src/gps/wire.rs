use crate::waves::wire::ScaledF32;

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Lon16(u16);

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Lat16(u16);

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Msl16(u16);

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vel16(u16);

// The distance movable within one packet length of maximum 20 seconds. For 100 km/h this is
// (100 * 1000 / (60 * 60) * 20 ~= 550 m
const MAX_KM_PER_DEGREE: f32 = 111.3;
const DEG_PER_M: f32 = 1.0 / (111.3 * 1e3);
pub const LON_RANGE: f32 = 2.0 * DEG_PER_M * 550.0 * 1e7;
// pub const LON_RANGE: f32 = 2.0 * 1.0e8 * 550.0 / (MAX_KM_PER_DEGREE * 1.0e3); // 550 m in both directions
// [deg * 1e-7]
// Maximum distance within 20 seconds: 2 * 60 m ?
pub const MSL_RANGE: f32 = 2.0 * 120.0 * 1.0e3; // 60 m in both directions [mm]

// Maximum speed
// 115 km/h range; u16 resolution ≈ 0.97 mm/s, below the chip's integer mm/s precision.
pub const VEL_RANGE: f32 = 115.0 * 1.0e6 / 60.0 / 60.0;

impl ScaledF32 for Lon16 {
    const MAX: f32 = LON_RANGE;

    fn from_u16(u: u16) -> Self {
        Lon16(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

impl ScaledF32 for Lat16 {
    const MAX: f32 = LON_RANGE;

    fn from_u16(u: u16) -> Self {
        Lat16(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

impl ScaledF32 for Msl16 {
    const MAX: f32 = MSL_RANGE;

    fn from_u16(u: u16) -> Self {
        Self(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

impl ScaledF32 for Vel16 {
    const MAX: f32 = VEL_RANGE;

    fn from_u16(u: u16) -> Self {
        Self(u)
    }

    fn to_u16(&self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waves::wire::{scale_f32_to_u16, scale_u16_to_f32};

    #[test]
    fn round_trip_lat() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * LON_RANGE / N as f32;
            let u = scale_f32_to_u16(LON_RANGE, v);
            let fu = scale_u16_to_f32(LON_RANGE, u);

            let uu = Lat16::from_f32(v);
            let fuu = uu.to_f32();

            assert_eq!(u, uu.to_u16());
            assert_eq!(fu, fuu);

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("lon range: {}", LON_RANGE);
        println!("lat u16 avg diff: {}", avg);
        println!("lat u16 max diff: {}", max);

        // 1 mm
        let deg_to_mm = (111.3 * 1.0e6) / 10.0e7;
        println!("1 mm: {}", deg_to_mm);
        println!("max mm: {}", max * deg_to_mm);
        println!("avg mm: {}", avg * deg_to_mm);

        assert!(max < 20.0);
        // panic!();
    }

    #[test]
    fn round_trip_msl() {
        let mut max: f32 = 0.0;
        let mut avg: f32 = 0.0;

        const N: i32 = 1000000i32;

        for i in 0..N {
            let v = (i as f32) * MSL_RANGE / N as f32;
            let u = scale_f32_to_u16(MSL_RANGE, v);
            let fu = scale_u16_to_f32(MSL_RANGE, u);

            let uu = Msl16::from_f32(v);
            let fuu = uu.to_f32();

            assert_eq!(u, uu.to_u16());
            assert_eq!(fu, fuu);

            let d = (v - fu).abs();
            max = max.max(d);
            avg = avg + d;
        }

        avg = avg / N as f32;
        println!("msl range: {}", MSL_RANGE);
        println!("msl u16 avg diff: {}", avg);
        println!("msl u16 max diff: {}", max);

        assert!(max < 4.0);
        // panic!();
    }

    #[test]
    fn vel_resolution_below_1mm_s() {
        // VEL_RANGE must be small enough that the u16 step size is < 1 mm/s,
        // matching the chip's native integer mm/s precision.
        let resolution = 2.0 * VEL_RANGE / u16::MAX as f32;
        println!("vel resolution: {} mm/s", resolution);
        assert!(resolution < 1.0, "vel resolution {:.3} mm/s >= 1 mm/s", resolution);
    }
}
