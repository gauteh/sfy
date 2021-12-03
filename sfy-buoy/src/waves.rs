//! Measure waves using an IMU, feed it through a Kalman filter and collect
//! time-series or statistics.

use embedded_hal::blocking::i2c::WriteRead;
use ahrs_fusion::NxpFusion;

/// The installed IMU.
pub type IMU = ();

pub struct Waves {
    _imu: IMU,
    filter: NxpFusion,
}

impl Waves {
    pub fn new(_i2c: impl WriteRead) -> Waves {
        Waves {
            _imu: (),
            filter: NxpFusion::new(5.)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::i2c::{Mock, Transaction};

    #[test]
    fn update_filter() {
        let expectations = [
            Transaction::write(0xaa, vec![1, 2]),
            Transaction::read(0xbb, vec![3, 4]),
        ];
        let mut i2c = Mock::new(&expectations);

        let mut w = Waves::new(i2c);
        w.filter.update(0.1, 0.2, 0.3, 0.3, 4., 0.5, 0., 0., 0.);
    }
}

