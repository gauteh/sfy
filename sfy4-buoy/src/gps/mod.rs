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

pub const GPS_PACKET_V: u8 = 4;
pub const GPS_PACKET_SZ: usize = 256;
/// Nominal inter-sample interval in milliseconds (25 Hz).
pub const GPS_NOMINAL_MS: i64 = 71; // 1000 / 14 Hz ≈ 71.4 ms, rounded
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
    pub filled: u16,
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
    pub filled: u16,
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
            filled: self.filled,
        };

        (meta, b64)
    }
}

/// Minimal subset of NavPvt fields needed for encoding and gap-fill copies.
/// Avoids storing the full NavPvt struct (~56 bytes) when only a fraction is used.
#[derive(Clone, Copy)]
struct PvtSample {
    lon: i32,
    lat: i32,
    height_msl_mm: i32,
    vel_n_mm_s: i32,
    vel_e_mm_s: i32,
    vel_d_mm_s: i32,
    h_acc_mm: u32,
    v_acc_mm: u32,
    fix_type: u8,
    flags: u8,
}

impl From<&NavPvt> for PvtSample {
    fn from(p: &NavPvt) -> Self {
        PvtSample {
            lon: p.lon,
            lat: p.lat,
            height_msl_mm: p.height_msl_mm,
            vel_n_mm_s: p.vel_n_mm_s,
            vel_e_mm_s: p.vel_e_mm_s,
            vel_d_mm_s: p.vel_d_mm_s,
            h_acc_mm: p.h_acc_mm,
            v_acc_mm: p.v_acc_mm,
            fix_type: p.fix_type,
            flags: p.flags,
        }
    }
}

/// Accumulates `NavPvt` samples and flushes them as `GpsPacket` bundles.
///
/// Samples are encoded incrementally into a `GpsPacket` as they arrive,
/// eliminating the need for a separate `Vec<NavPvt, GPS_PACKET_SZ>` buffer
/// (~14 KB on Apollo3).
pub struct GpsCollector {
    queue: Producer<'static, GpsPacket, EPGS_SZ>,
    /// In-progress packet.  `None` until the first sample of a new packet arrives.
    pending: Option<GpsPacket>,
    /// Running sums for computing ha_mean / va_mean at flush time.
    ha_sum: f32,
    va_sum: f32,
    /// Total real PVT messages received since last flush.
    total_pvts: u32,
    /// Fill samples inserted since last flush (epochs the GPS chip skipped).
    fill_count: u32,
    /// Last real PVT (minimal fields); used as the source for gap-fill copies.
    last_pvt: Option<PvtSample>,
    /// Millisecond timestamp of the last slot encoded (real or fill).
    /// Persists across flushes so cross-packet gaps are filled too.
    last_sample_ts: Option<i64>,
}

impl GpsCollector {
    pub fn new(queue: Producer<'static, GpsPacket, EPGS_SZ>) -> Self {
        GpsCollector {
            queue,
            pending: None,
            ha_sum: 0.0,
            va_sum: 0.0,
            total_pvts: 0,
            fill_count: 0,
            last_pvt: None,
            last_sample_ts: None,
        }
    }

    /// Encode one sample into `pending`, initialising the packet if needed.
    /// Flushes the completed packet to the queue first if it is full.
    fn encode_one(&mut self, s: &PvtSample, ts: i64) {
        // Flush a completed packet before writing the new sample.
        if self
            .pending
            .as_ref()
            .map_or(false, |p| p.data.len() == 6 * GPS_PACKET_SZ)
        {
            self.flush();
        }

        let pending = self.pending.get_or_insert_with(|| GpsPacket {
            timestamp: ts,
            freq: 1000.0 / GPS_NOMINAL_MS as f32,
            version: GPS_PACKET_V,
            lon: s.lon,
            lat: s.lat,
            msl: s.height_msl_mm,
            data: Vec::new(),
            ha_min: f32::MAX,
            ha_max: 0.0_f32,
            ha_mean: 0.0_f32,
            va_min: f32::MAX,
            va_max: 0.0_f32,
            va_mean: 0.0_f32,
            fix: [0u16; 8],
            soln: [0u16; 8],
            filled: 0,
        });

        let (ref_lon, ref_lat, ref_msl) = (pending.lon, pending.lat, pending.msl);

        // 6 channels: lon-delta, lat-delta, msl-delta, vel_n, vel_e, vel_d
        for v in [
            Lon16::from_i32(s.lon - ref_lon).to_u16(),
            Lat16::from_i32(s.lat - ref_lat).to_u16(),
            Msl16::from_i32(s.height_msl_mm - ref_msl).to_u16(),
            Vel16::from_i32(s.vel_n_mm_s).to_u16(),
            Vel16::from_i32(s.vel_e_mm_s).to_u16(),
            Vel16::from_i32(s.vel_d_mm_s).to_u16(),
        ] {
            pending.data.push(v).ok();
        }

        let ha = s.h_acc_mm as f32;
        pending.ha_min = pending.ha_min.min(ha);
        pending.ha_max = pending.ha_max.max(ha);
        self.ha_sum += ha;

        let va = s.v_acc_mm as f32;
        pending.va_min = pending.va_min.min(va);
        pending.va_max = pending.va_max.max(va);
        self.va_sum += va;

        let fix_idx = (s.fix_type as usize).min(pending.fix.len() - 1);
        pending.fix[fix_idx] += 1;
        let soln_idx = ((s.flags as usize >> 5) & 0x3).min(pending.soln.len() - 1);
        pending.soln[soln_idx] += 1;
    }

    /// Finalise the in-progress packet and push it to the queue.
    fn flush(&mut self) {
        if let Some(mut pkt) = self.pending.take() {
            if pkt.data.is_empty() {
                return;
            }
            let n = (pkt.data.len() / 6) as f32;
            pkt.ha_mean = self.ha_sum / n;
            pkt.va_mean = self.va_sum / n;
            pkt.filled = self.fill_count as u16;

            debug!(
                "GPS: collected packet, samples: {}, real: {}, filled: {}",
                n as u32,
                self.total_pvts,
                self.fill_count,
            );

            self.ha_sum = 0.0;
            self.va_sum = 0.0;
            self.total_pvts = 0;
            self.fill_count = 0;

            let _ = self
                .queue
                .enqueue(pkt)
                .inspect_err(|_| error!("GPS: could not enqueue GpsPacket"));
        }
    }

    /// Add a `NavPvt` sample.  Samples where `pvt_timestamp` cannot build a
    /// valid UTC datetime are dropped (and flush the pending packet).  Fix type
    /// and the `valid` flag bits are not pre-filtered — `pvt_timestamp` already
    /// validates the individual date/time fields and rejects truly bogus values.
    ///
    /// Gaps wider than 1.5× the nominal interval are filled with copies of the
    /// last known sample so the encoded buffer stays equidistant at the nominal
    /// frequency even when the GPS chip skips epochs.
    pub fn add_sample(&mut self, pvt: NavPvt) {
        self.total_pvts += 1;
        let ts = match pvt_timestamp(&pvt) {
            Some(t) => t.and_utc().timestamp_millis(),
            None => {
                warn!("GPS: cannot build timestamp from PVT, flushing pending packet");
                self.flush();
                return;
            }
        };

        // Fill missed epochs with copies of the last known sample.
        if let (Some(prev_ts), Some(last)) = (self.last_sample_ts, self.last_pvt) {
            let gap = ts - prev_ts;
            let missed =
                ((gap + GPS_NOMINAL_MS / 2) / GPS_NOMINAL_MS).saturating_sub(1) as u32;
            if missed > 0 {
                debug!("GPS: filling {} missed epoch(s) (gap: {} ms)", missed, gap);
            }
            for i in 1..=missed {
                self.encode_one(&last, prev_ts + i as i64 * GPS_NOMINAL_MS);
                self.fill_count += 1;
                self.last_sample_ts = Some(prev_ts + i as i64 * GPS_NOMINAL_MS);
            }
        }

        let sample = PvtSample::from(&pvt);
        self.encode_one(&sample, ts);
        self.last_sample_ts = Some(ts);
        self.last_pvt = Some(sample);
        self.check_collect();
    }

    pub fn check_collect(&mut self) {
        if self
            .pending
            .as_ref()
            .map_or(false, |p| p.data.len() == 6 * GPS_PACKET_SZ)
        {
            info!(
                "GPS buf full, collecting into GpsPacket (queue len: {})",
                self.queue.len()
            );
            self.flush();
        }
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
            fix: [0u16; 8],
            soln: [0u16; 8],
            filled: 0,
        };

        let b64 = p.base64();
        println!("{}", core::str::from_utf8(&b64).unwrap());
    }
}

