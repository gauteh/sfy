//! Remote power-mode control.
//!
//! Selects between a small ladder of power levels, driven by the Notehub
//! environment variable `power_mode` (0-3), with an independently
//! overridable `sync_period` (minutes) env var. Both are read at runtime via
//! `Notecarrier::get_env_var` (`env.get`); this module is pure/host-testable
//! and only concerns itself with *resolving* the desired configuration --
//! applying it (updating `EgpsDutyCycle`/`Imu`/`hub.set`) happens in the
//! main loop.
//!
//! Levels:
//!
//! - [`PowerMode::Normal`] (0): current default. Full egps duty-cycle
//!   (position wake + periodic ~20 min batch), IMU/AXL sampled and
//!   sent continuously.
//! - [`PowerMode::NoBatch`] (1): egps switches to position-only (no
//!   batches at all). IMU/AXL unchanged (continuous).
//! - [`PowerMode::DutyImu`] (2): same egps config as `Normal` (batches
//!   still happen, always ~20 min, see `EgpsDutyCycleConfig`), but
//!   IMU/AXL streaming is confined to exactly those batch windows
//!   instead of running continuously -- much less data is produced, so
//!   `sync_period` can be relaxed.
//! - [`PowerMode::PositionOnly`] (3): egps wakes only for a position fix
//!   every ~12 h (no batch, no continuous streaming), IMU sampling never
//!   streams at all, and a sync is forced after each wake attempt
//!   regardless of whether a fix was obtained (see the main loop's
//!   level-3 sync trigger), independent of `sync_period`.

use crate::gps::duty::EgpsDutyCycleConfig;

/// One rung of the power ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerMode {
    #[default]
    Normal = 0,
    NoBatch = 1,
    DutyImu = 2,
    PositionOnly = 3,
}

impl PowerMode {
    /// Parse from the raw text of the `power_mode` env var. Accepts the
    /// numeric level (`"0"`..`"3"`) or, tolerantly, its name
    /// (case-insensitive, e.g. `"duty_imu"`/`"DutyImu"`). Unknown/garbled
    /// input falls back to `None` so the caller can keep the last-known-good
    /// mode rather than silently reverting to `Normal`.
    pub fn parse(s: &str) -> Option<PowerMode> {
        let s = s.trim();
        match s {
            "0" => Some(PowerMode::Normal),
            "1" => Some(PowerMode::NoBatch),
            "2" => Some(PowerMode::DutyImu),
            "3" => Some(PowerMode::PositionOnly),
            _ => {
                // Compare case-insensitively without allocating (`no_std`,
                // no `alloc`): strip common separators by skipping them
                // character-by-character rather than building a new String.
                let matches = |name: &str| -> bool {
                    let mut a = s.chars().filter(|c| *c != '_' && *c != '-');
                    let mut b = name.chars();
                    loop {
                        match (a.next(), b.next()) {
                            (Some(x), Some(y)) => {
                                if !x.eq_ignore_ascii_case(&y) {
                                    return false;
                                }
                            }
                            (None, None) => return true,
                            _ => return false,
                        }
                    }
                };
                if matches("normal") {
                    Some(PowerMode::Normal)
                } else if matches("nobatch") {
                    Some(PowerMode::NoBatch)
                } else if matches("dutyimu") {
                    Some(PowerMode::DutyImu)
                } else if matches("positiononly") {
                    Some(PowerMode::PositionOnly)
                } else {
                    None
                }
            }
        }
    }

    pub const fn from_build_default(mode: u8) -> PowerMode {
        match mode {
            0 => PowerMode::Normal,
            1 => PowerMode::NoBatch,
            2 => PowerMode::DutyImu,
            _ => PowerMode::PositionOnly,
        }
    }
}

/// How the IMU/AXL sensor's completed packets should be handled (see
/// `Imu::set_streaming`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImuMode {
    /// Always enqueue completed packets (today's default behavior).
    Continuous,
    /// Only enqueue while the egps duty-cycle is in a batch --
    /// caller should set `imu.set_streaming(egps_duty.is_streaming())` every
    /// poll.
    FollowEgpsBatch,
    /// Never enqueue completed packets (power level `PositionOnly`).
    Off,
}

/// Build-time defaults threaded through to [`resolve`], so the power ladder
/// stays tunable per-deployment without adding more runtime knobs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerBuildDefaults {
    /// The "normal"/level 0 & 1 & 2 egps config (position/dwell/batch
    /// duration+period/sleep-threshold), from the existing
    /// `EGPS_*` build-time env vars.
    pub normal_egps: EgpsDutyCycleConfig,
    /// Level 3's position-fix interval (seconds), from `EGPS_L3_POSITION_INTERVAL`.
    pub l3_position_interval_s: u32,
    /// Level 3's max dwell time waiting for a fix (seconds), from `EGPS_L3_POSITION_DWELL`.
    pub l3_position_dwell_s: u32,
    /// Default sync period (minutes), from `SYNC_PERIOD`, used when no
    /// `sync_period` env var override is present.
    pub sync_period_min: u32,
}

/// Fully-resolved configuration for a given power mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerConfig {
    pub mode: PowerMode,
    pub egps: EgpsDutyCycleConfig,
    pub imu: ImuMode,
    pub sync_period_min: u32,
    /// Level 3 only: force a `hub.sync()` after each egps wake attempt,
    /// regardless of whether a fix was obtained, independent of
    /// `sync_period_min`.
    pub force_sync_on_egps_wake: bool,
}

/// Resolve a [`PowerConfig`] for `mode`, using `sync_period_override` (from
/// the `sync_period` env var, minutes) if present, else
/// `defaults.sync_period_min`.
pub fn resolve(
    mode: PowerMode,
    sync_period_override: Option<u32>,
    defaults: &PowerBuildDefaults,
) -> PowerConfig {
    let sync_period_min = sync_period_override.unwrap_or(defaults.sync_period_min);

    match mode {
        PowerMode::Normal => PowerConfig {
            mode,
            egps: defaults.normal_egps,
            imu: ImuMode::Continuous,
            sync_period_min,
            force_sync_on_egps_wake: false,
        },
        PowerMode::NoBatch => PowerConfig {
            mode,
            egps: EgpsDutyCycleConfig {
                batch_duration_ms: 0,
                ..defaults.normal_egps
            },
            imu: ImuMode::Continuous,
            sync_period_min,
            force_sync_on_egps_wake: false,
        },
        PowerMode::DutyImu => PowerConfig {
            mode,
            egps: defaults.normal_egps,
            imu: ImuMode::FollowEgpsBatch,
            sync_period_min,
            force_sync_on_egps_wake: false,
        },
        PowerMode::PositionOnly => PowerConfig {
            mode,
            egps: EgpsDutyCycleConfig::from_secs(
                defaults.l3_position_interval_s,
                defaults.l3_position_dwell_s,
                0, // no batch
                defaults.l3_position_interval_s, // period is moot (duration 0 => never bursts)
            ),
            imu: ImuMode::Off,
            sync_period_min,
            force_sync_on_egps_wake: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> PowerBuildDefaults {
        PowerBuildDefaults {
            normal_egps: EgpsDutyCycleConfig::from_secs(600, 120, 1200, 10800),
            l3_position_interval_s: 43200,
            l3_position_dwell_s: 300,
            sync_period_min: 20,
        }
    }

    #[test]
    fn parses_numeric_and_named_modes() {
        assert_eq!(PowerMode::parse("0"), Some(PowerMode::Normal));
        assert_eq!(PowerMode::parse("1"), Some(PowerMode::NoBatch));
        assert_eq!(PowerMode::parse("2"), Some(PowerMode::DutyImu));
        assert_eq!(PowerMode::parse("3"), Some(PowerMode::PositionOnly));

        assert_eq!(PowerMode::parse("normal"), Some(PowerMode::Normal));
        assert_eq!(PowerMode::parse("Normal"), Some(PowerMode::Normal));
        assert_eq!(PowerMode::parse("no_batch"), Some(PowerMode::NoBatch));
        assert_eq!(PowerMode::parse("DutyImu"), Some(PowerMode::DutyImu));
        assert_eq!(
            PowerMode::parse("position-only"),
            Some(PowerMode::PositionOnly)
        );

        assert_eq!(PowerMode::parse(""), None);
        assert_eq!(PowerMode::parse("4"), None);
        assert_eq!(PowerMode::parse("garbage"), None);
    }

    #[test]
    fn normal_uses_build_default_egps_config_unchanged() {
        let d = defaults();
        let c = resolve(PowerMode::Normal, None, &d);
        assert_eq!(c.egps, d.normal_egps);
        assert_eq!(c.imu, ImuMode::Continuous);
        assert!(!c.force_sync_on_egps_wake);
        assert_eq!(c.sync_period_min, d.sync_period_min);
    }

    #[test]
    fn no_batch_disables_bursts_but_keeps_imu_continuous() {
        let d = defaults();
        let c = resolve(PowerMode::NoBatch, None, &d);
        assert!(c.egps.is_position_only());
        assert_eq!(c.egps.position_interval_ms, d.normal_egps.position_interval_ms);
        assert_eq!(c.imu, ImuMode::Continuous);
    }

    #[test]
    fn duty_imu_keeps_batches_but_gates_imu_to_the_batch_window() {
        let d = defaults();
        let c = resolve(PowerMode::DutyImu, None, &d);
        assert_eq!(c.egps, d.normal_egps);
        assert!(!c.egps.is_position_only());
        assert_eq!(c.imu, ImuMode::FollowEgpsBatch);
    }

    #[test]
    fn position_only_wakes_every_12h_no_imu_and_forces_sync() {
        let d = defaults();
        let c = resolve(PowerMode::PositionOnly, None, &d);
        assert!(c.egps.is_position_only());
        assert_eq!(c.egps.position_interval_ms, d.l3_position_interval_s as i64 * 1000);
        assert_eq!(c.imu, ImuMode::Off);
        assert!(c.force_sync_on_egps_wake);
    }

    #[test]
    fn sync_period_override_takes_precedence_over_build_default() {
        let d = defaults();
        let c = resolve(PowerMode::Normal, Some(180), &d);
        assert_eq!(c.sync_period_min, 180);

        let c = resolve(PowerMode::Normal, None, &d);
        assert_eq!(c.sync_period_min, d.sync_period_min);
    }
}
