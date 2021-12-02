//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use core::ops::{Deref, DerefMut};
use crate::*;

/// The installed IMU.
pub type IMU = ();

pub struct Waves {
}
