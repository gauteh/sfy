use chrono::{Datelike, NaiveDateTime, Timelike};
use core::sync::atomic::{AtomicI32, Ordering};
use embedded_sdmmc::{TimeSource, Timestamp};

pub struct NullClock;

impl TimeSource for NullClock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// Accesses `core::COUNT` to get globally updated timestamp from RTC interrupt, which is set by
/// the GPS.
#[derive(Clone, Copy)]
pub struct CountClock(pub &'static AtomicI32);

impl TimeSource for CountClock {
    fn get_timestamp(&self) -> Timestamp {
        let dt = NaiveDateTime::from_timestamp(self.0.load(Ordering::Relaxed) as i64, 0);
        Timestamp {
            year_since_1970: (dt.year() - 1970) as u8,
            zero_indexed_month: dt.month0() as u8,
            zero_indexed_day: dt.day0() as u8,
            hours: dt.hour() as u8,
            minutes: dt.minute() as u8,
            seconds: dt.second() as u8,
        }
    }
}
