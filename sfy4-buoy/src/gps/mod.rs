//! GPS interface using the MAX-M10S GNSS module over I2C.
//!
//! The external max-m10s driver reads `NavPvt` packets from the module.
//! This module collects those packets into `GpsPacket` bundles for
//! transmission via the notecard, and provides `EgpsTime` for RTC time-sync.
use chrono::{NaiveDate, NaiveDateTime};
#[allow(unused_imports)]
use defmt::{debug, error, info, println, trace, warn};
use heapless::{
    spsc::{Producer, Queue},
    Vec,
};

pub use max_m10s::ubx::NavPvt;

pub const GPS_PACKET_V: u8 = 3;
pub const GPS_PACKET_SZ: usize = 512;
/// Maximum length of base64 string produced from one GpsPacket.
pub const GPS_OUTN: usize = { 6 * GPS_PACKET_SZ * 2 } * 4 / 3 + 4;

mod wire;
pub use wire::*;

use crate::waves::wire::ScaledF32;
use crate::EPGS_SZ;

/// Queue from GPS collector to Notecard sender.
pub static mut EGPSQ: Queue<GpsPacket, { crate::EPGS_SZ }> = Queue::new();

/// Time and position snapshot delivered by the GPS timepulse interrupt.
#[derive(PartialEq, Clone, defmt::Format)]
pub struct EgpsTime {
    /// UTC time in milliseconds since the Unix epoch (from NavPvt).
    pub time: i64,
    /// Value of the RTC counter at the moment the timepulse fired (ms).
    pub pps_time: i64,
    pub lon: f64,
    pub lat: f64,
}

impl EgpsTime {
    /// Build an `EgpsTime` from a freshly-read `NavPvt` and the RTC snapshot
    /// captured in the timepulse interrupt (`pps_time`).
    ///
    /// Returns `None` when the PVT does not carry a valid date+time.
    pub fn from_pvt(pvt: &NavPvt, pps_time: i64) -> Option<Self> {
        if (pvt.valid & 0x03) != 0x03 {
            return None;
        }
        let ts = pvt_timestamp(pvt)?;
        Some(EgpsTime {
            time: ts.and_utc().timestamp_millis(),
            pps_time,
            lon: pvt.lon as f64 / 1.0e7,
            lat: pvt.lat as f64 / 1.0e7,
        })
    }
}

/// Construct a `NaiveDateTime` from a `NavPvt`, handling negative `nano`.
pub fn pvt_timestamp(pvt: &NavPvt) -> Option<NaiveDateTime> {
    let mut sec = pvt.sec;
    let mut nano = pvt.nano;

    if nano < 0 {
        if sec == 0 {
            return None; // would underflow into previous minute
        }
        sec -= 1;
        nano = 1_000_000_000 + nano;
    }

    NaiveDate::from_ymd_opt(pvt.year.into(), pvt.month.into(), pvt.day.into()).and_then(|d| {
        d.and_hms_nano_opt(
            pvt.hour.into(),
            pvt.min.into(),
            sec.into(),
            nano as u32,
        )
    })
}

/// A packet of GPS samples
#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
pub struct GpsPacket {
    /// Timestamp of first sample
    pub timestamp: i64,

    pub freq: f32,

    pub version: u8,

    /// Reference position (mean of all samples).
    pub lon: i32,
    pub lat: i32,
    pub msl: i32,

    /// Packed position+velocity deltas as interleaved u16 values.
    pub data: Vec<u16, { 6 * GPS_PACKET_SZ }>,

    pub ha_min: f32,
    pub ha_max: f32,
    pub ha_mean: f32,

    pub va_min: f32,
    pub va_max: f32,
    pub va_mean: f32,

    pub fix: [u16; 8],
    pub soln: [u16; 8],
}

// XXX: Match with template in note
#[derive(serde::Serialize, Default)]
pub struct GpsPacketMeta {
    /// Timestamp of first sample
    pub timestamp: i64,

    pub freq: f32,

    pub version: u8,

    /// Reference position (mean of all samples).
    pub lon: i32,
    pub lat: i32,
    pub msl: i32,

    pub lonlat_range: f32, // 1e-7 deg
    pub msl_range: f32,    // mm
    pub vel_range: f32,    // mm/s
    pub length: u32,

    pub ha_min: f32,
    pub ha_max: f32,
    pub ha_mean: f32,

    pub va_min: f32,
    pub va_max: f32,
    pub va_mean: f32,

    pub fix: [u16; 8],
    pub soln: [u16; 8],
}

impl GpsPacket {
    pub fn len(&self) -> usize {
        self.data.len() / 6
    }

    pub fn base64(&self) -> Vec<u8, GPS_OUTN> {
        let mut b64: Vec<_, GPS_OUTN> = Vec::new();
        b64.resize_default(GPS_OUTN).unwrap();

        #[cfg(target_endian = "big")]
        compile_error!("serialized samples are assumed little-endian; big-endian not supported.");

        let data = bytemuck::cast_slice(&self.data);
        let written = base64::encode_config_slice(data, base64::STANDARD, &mut b64);
        b64.truncate(written);

        b64
    }

    /// Split package into metadata and base64 payload.
    pub fn split(&self) -> (GpsPacketMeta, Vec<u8, GPS_OUTN>) {
        let b64 = self.base64();

        let meta = GpsPacketMeta {
            timestamp: self.timestamp,
            length: b64.len() as u32,
            freq: self.freq,
            lonlat_range: wire::LON_RANGE,
            msl_range: wire::MSL_RANGE,
            vel_range: wire::VEL_RANGE,
            version: GPS_PACKET_V,
            lon: self.lon,
            lat: self.lat,
            msl: self.msl,
            ha_min: self.ha_min,
            ha_max: self.ha_max,
            ha_mean: self.ha_mean,
            va_min: self.va_min,
            va_max: self.va_max,
            va_mean: self.va_mean,
            fix: self.fix,
            soln: self.soln,
        };

        (meta, b64)
    }
}

/// Accumulates `NavPvt` samples and flushes them as `GpsPacket` bundles.
pub struct GpsCollector {
    queue: Producer<'static, GpsPacket, EPGS_SZ>,
    buf: Vec<NavPvt, { GPS_PACKET_SZ }>,
}

impl GpsCollector {
    pub fn new(queue: Producer<'static, GpsPacket, EPGS_SZ>) -> Self {
        GpsCollector {
            queue,
            buf: Vec::new(),
        }
    }

    /// Add a `NavPvt` sample.  Samples without a 3D fix or valid UTC
    /// date+time are silently dropped.  Triggers `collect` when the buffer
    /// is full.
    pub fn add_sample(&mut self, pvt: NavPvt) {
        if pvt.fix_type < 2 || (pvt.valid & 0x03) != 0x03 {
            return;
        }
        if pvt_timestamp(&pvt).is_none() {
            warn!("GPS: cannot build timestamp from PVT, flushing buffer");
            self.collect();
            return;
        }
        self.buf
            .push(pvt)
            .inspect_err(|_| error!("GPS: sample buf full, discarding sample"))
            .ok();
        self.check_collect();
    }

    pub fn check_collect(&mut self) {
        if self.buf.is_full() {
            info!(
                "GPS buf full, collecting into GpsPacket (queue len: {})",
                self.queue.len()
            );
            self.collect();
        }
    }

    pub fn collect(&mut self) {
        if self.buf.is_empty() {
            return;
        }

        let s = &self.buf[0];
        let (lon, lat, msl) = (s.lon, s.lat, s.height_msl_mm);

        let data: Vec<u16, { 6 * GPS_PACKET_SZ }> = self
            .buf
            .iter()
            .flat_map(|s| {
                [
                    Lon16::from_i32(s.lon - lon).to_u16(),
                    Lat16::from_i32(s.lat - lat).to_u16(),
                    Msl16::from_i32(s.height_msl_mm - msl).to_u16(),
                    Vel16::from_i32(s.vel_n_mm_s).to_u16(),
                    Vel16::from_i32(s.vel_e_mm_s).to_u16(),
                    Vel16::from_i32(s.vel_d_mm_s).to_u16(),
                ]
            })
            .collect();

        // unwrap: timestamp validity checked in add_sample
        let timestamp = pvt_timestamp(&self.buf[0])
            .unwrap()
            .and_utc()
            .timestamp_millis();

        let freq: f32 = if self.buf.len() > 1 {
            self.buf
                .windows(2)
                .map(|w| {
                    pvt_timestamp(&w[1]).unwrap().and_utc().timestamp_millis()
                        - pvt_timestamp(&w[0]).unwrap().and_utc().timestamp_millis()
                })
                .sum::<i64>() as f32
                / (self.buf.len() - 1) as f32
        } else {
            1000.0 // assume 1 Hz when only one sample
        };
        let freq = 1000.0 / freq;

        let n = self.buf.len() as f32;
        let mut ha_min = f32::MAX;
        let mut ha_max = 0.0f32;
        let mut ha_mean = 0.0f32;
        let mut va_min = f32::MAX;
        let mut va_max = 0.0f32;
        let mut va_mean = 0.0f32;
        let mut fix = [0u16; 8];
        let mut soln = [0u16; 8];

        for s in self.buf.iter() {
            let ha = s.h_acc_mm as f32;
            ha_min = ha_min.min(ha);
            ha_max = ha_max.max(ha);
            ha_mean += ha / n;

            let va = s.v_acc_mm as f32;
            va_min = va_min.min(va);
            va_max = va_max.max(va);
            va_mean += va / n;

            let fix_idx = (s.fix_type as usize).min(fix.len() - 1);
            fix[fix_idx] += 1;

            let soln_idx = ((s.flags >> 5) & 0x3) as usize;
            soln[soln_idx] += 1;
        }

        self.buf.clear();

        let p = GpsPacket {
            timestamp,
            freq,
            version: GPS_PACKET_V,
            lon,
            lat,
            msl,
            data,
            ha_min,
            ha_max,
            ha_mean,
            va_min,
            va_max,
            va_mean,
            fix,
            soln,
        };

        debug!(
            "GPS: collected packet, freq: {}, timestamp: {}, samples: {}",
            p.freq,
            p.timestamp,
            p.len()
        );

        let _ = self
            .queue
            .enqueue(p)
            .inspect_err(|_| error!("GPS: could not enqueue GpsPacket"));
    }
}

#[cfg(test)]
mod tests {
    use super::{GpsPacket, GPS_PACKET_SZ, GPS_PACKET_V};

    #[test]
    fn base64_data_package() {
        let p = GpsPacket {
            timestamp: 0,
            lat: 0,
            lon: 0,
            msl: 20,
            freq: 100.0,
            version: GPS_PACKET_V,
            data: (0..(6 * GPS_PACKET_SZ))
                .map(|v| v as u16)
                .collect::<heapless::Vec<_, { 6 * GPS_PACKET_SZ }>>(),
            ha_mean: 10.0,
            ha_min: 10.0,
            ha_max: 10.0,
            va_mean: 10.0,
            va_min: 10.0,
            va_max: 10.0,
            fix: [0u8; 8],
            soln: [0u8; 8],
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }
}

