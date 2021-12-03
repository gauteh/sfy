//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use embedded_hal::blocking::i2c::WriteRead;

/// The installed IMU.
pub type IMU = ();

pub struct Waves {
    _imu: IMU,
}

impl Waves {
    pub fn new(_i2c: impl WriteRead) -> Waves {
        Waves {
            _imu: ()
        }
    }
}

