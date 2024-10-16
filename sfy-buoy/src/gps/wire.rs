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

// The distance movable within one packet length of maximum 20 seconds. For 100 km/h this is
// (100 * 1000 / (60 * 60) * 20 ~= 550 m
const MAX_KM_PER_DEGREE: f32 = 111.3;
pub const LON_RANGE: f32 = 2.0 * 1.0e7 * 550.0 / (MAX_KM_PER_DEGREE * 1.0e3); // 550 m in both directions
                                                                        // [deg * 1e-7]
// Maximum distance within 20 seconds: 60 m ?
pub const MSL_RANGE: f32 = 2.0 * 60.0 * 1.0e3; // 60 m in both directions [mm]

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
