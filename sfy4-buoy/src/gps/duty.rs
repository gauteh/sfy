//! Duty-cycled EGPS state machine.
//!
//! This is pure, host-testable logic: it takes the current RTC time (in
//! milliseconds) as input and returns what the caller should do with the
//! GPS module (power it on/off, set output rate, start/stop feeding
//! samples into `GpsCollector`). It does not touch any hardware itself.
//!
//! Three states:
//!
//! - [`EgpsState::Idle`]: GPS is fully powered off (`d8` GPIO). No samples
//!   are collected. There is no "backup sleep" idle mode: the MAX-M10S's
//!   `UBX-RXM-PMREQ` backup sleep can only be woken by a hardware EXTINT
//!   pulse or a power cycle, and this board does not wire up EXTINT, so a
//!   full power-off/re-init is used for every idle gap.
//! - [`EgpsState::AcquiringFix`]: GPS is powered/re-initialised and running
//!   at a low output rate until a valid fix is obtained (or the dwell time
//!   elapses).
//! - [`EgpsState::Batch`]: a high-rate burst is in progress; samples are
//!   fed to `GpsCollector`/`EGPSQ`. This covers two cases: a full scheduled
//!   duty-cycle batch (`EGPS_BATCH_DURATION_S`), or -- on an otherwise
//!   position-only wake -- a brief post-fix sample
//!   (`EGPS_POSITION_SAMPLE_MS`) so something is still queued for the next
//!   sync even when no full batch is due.
//!
//! `position_interval_ms == 0` ("0 gap") is a special case: the module is
//! never idled or power-cycled and just runs continuously -- this is the
//! default, and reproduces the historical always-on behavior. See
//! [`EgpsDutyCycleConfig::is_continuous`].

/// Fixed duration (seconds) of an egps batch burst whenever batches are
/// enabled (`batch_duration_ms > 0`) -- not build-time configurable, since
/// there is no reason for it to differ from this (matches the axl/IMU
/// `spectrum` feature's fixed 20-minute window, see `waves::welch::Welch`).
pub const EGPS_BATCH_DURATION_S: u32 = 1200; // 20 min

/// Minimum duration (milliseconds) to stream at full output rate right
/// after acquiring a fix on a *position-only* wake (`is_batch == false`,
/// i.e. one that isn't a scheduled duty-cycle batch) -- long enough to
/// reliably capture at least one full `GPS_PACKET_SZ`-sample packet at the
/// nominal sample rate, plus margin for the GPS module's rate-switch delay.
/// Without this, position-only wakes would only ever update the RTC
/// time-sync/last-known-position and never queue anything into
/// `GpsCollector`/`EGPSQ`, so nothing new would go out on the next sync.
pub const EGPS_POSITION_SAMPLE_MS: i64 =
    super::GPS_PACKET_SZ as i64 * super::GPS_NOMINAL_MS + 5_000;

/// Configuration for the duty-cycle state machine, all values in
/// milliseconds. Build with [`EgpsDutyCycleConfig::from_secs`] from the
/// (seconds-based) build-time env vars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EgpsDutyCycleConfig {
    /// How often to wake egps for a position/time fix outside of a burst.
    pub position_interval_ms: i64,
    /// Max time to wait for a valid fix per wake before giving up.
    pub position_dwell_ms: i64,
    /// Length of a high-rate burst. `0` disables bursts entirely
    /// (position-only mode).
    pub batch_duration_ms: i64,
    /// Start-to-start interval between batches.
    pub batch_period_ms: i64,
}

impl EgpsDutyCycleConfig {
    pub const fn from_secs(
        position_interval_s: u32,
        position_dwell_s: u32,
        batch_duration_s: u32,
        batch_period_s: u32,
    ) -> Self {
        EgpsDutyCycleConfig {
            position_interval_ms: position_interval_s as i64 * 1000,
            position_dwell_ms: position_dwell_s as i64 * 1000,
            batch_duration_ms: batch_duration_s as i64 * 1000,
            batch_period_ms: batch_period_s as i64 * 1000,
        }
    }

    /// `true` if bursts are disabled entirely (position-only mode).
    pub const fn is_position_only(&self) -> bool {
        self.batch_duration_ms <= 0
    }

    /// `true` if there is "no gap" between wakes -- the module is never
    /// idled or power-cycled and just runs continuously. This is the
    /// default (`position_interval_ms == 0`) and reproduces the historical
    /// always-on behavior.
    pub const fn is_continuous(&self) -> bool {
        self.position_interval_ms <= 0
    }
}

/// State of the duty-cycle state machine.
#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum EgpsState {
    /// GPS idle, fully powered off via `d8`. Will wake at `next_wake_at`.
    Idle { next_wake_at: i64 },
    /// GPS powered/resumed, waiting for a valid fix (or dwell timeout).
    AcquiringFix { started_at: i64, is_batch: bool },
    /// High-rate streaming in progress, started at `started_at`, scheduled
    /// to end at `ends_at`. `is_batch` is `true` for a full scheduled
    /// duty-cycle batch (`EGPS_BATCH_DURATION_S`/`is_continuous`), or
    /// `false` for the brief post-fix sample taken on an otherwise
    /// position-only wake (`EGPS_POSITION_SAMPLE_MS`) -- only real batches
    /// reschedule `next_batch_start` when they end.
    Batch {
        started_at: i64,
        ends_at: i64,
        is_batch: bool,
    },
}

/// Action the caller should take in response to a `poll`/`fix_acquired` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum EgpsAction {
    /// Nothing to do.
    None,
    /// Fully power off via `d8`.
    EnterIdlePowerOff,
    /// Wake from a full power-off: re-run cold-init sequence, set output
    /// rate low (1 Hz).
    WakePowerOnReinit,
    /// Valid fix obtained: restore full output rate and start feeding
    /// `GpsCollector`. Either a full scheduled batch, or -- on an
    /// otherwise position-only wake -- a brief post-fix sample so
    /// something is still queued for the next sync.
    StartBatch,
    /// Batch ended: flush any partial packet, stop feeding
    /// samples, then fully power off.
    EndBatchIdlePowerOff,
}

/// Duty-cycled EGPS state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EgpsDutyCycle {
    config: EgpsDutyCycleConfig,
    state: EgpsState,
    /// Time (RTC ms) at which the next batch is allowed to start.
    next_batch_start: i64,
}

impl EgpsDutyCycle {
    /// Create a new state machine, starting in `Idle` with an immediate
    /// wake (so the very first `poll` call triggers an acquisition), and
    /// bursts allowed to start immediately (subject to
    /// `config.is_position_only()`).
    pub fn new(config: EgpsDutyCycleConfig, now: i64) -> Self {
        EgpsDutyCycle {
            config,
            state: EgpsState::Idle { next_wake_at: now },
            next_batch_start: now,
        }
    }

    /// Replace the configuration in place — e.g. in response to a live
    /// power-mode change from Notehub environment variables (see
    /// `sfy::power`). Does not reset or otherwise touch the current runtime
    /// state (`state`, `next_batch_start`): any wake/burst already
    /// scheduled under the old config completes as scheduled. The new
    /// config takes effect starting with the next transition computed by
    /// `enter_idle` (i.e. the next time the state machine goes idle).
    pub fn set_config(&mut self, config: EgpsDutyCycleConfig) {
        self.config = config;
    }

    pub fn state(&self) -> EgpsState {
        self.state
    }

    pub fn config(&self) -> &EgpsDutyCycleConfig {
        &self.config
    }

    /// Whether the caller should currently be feeding samples into
    /// `GpsCollector`/`EGPSQ`.
    pub fn is_streaming(&self) -> bool {
        matches!(self.state, EgpsState::Batch { .. })
    }

    /// Drive the state machine forward in time. Call regularly (e.g. from
    /// the main loop) with the current RTC time in milliseconds. Does not
    /// itself handle a freshly-acquired fix -- call [`Self::fix_acquired`]
    /// for that.
    pub fn poll(&mut self, now: i64) -> EgpsAction {
        match self.state {
            EgpsState::Idle { next_wake_at } => {
                if now >= next_wake_at {
                    let is_burst = !self.config.is_position_only()
                        && (self.config.is_continuous() || now >= self.next_batch_start);
                    self.state = EgpsState::AcquiringFix {
                        started_at: now,
                        is_batch: is_burst,
                    };
                    EgpsAction::WakePowerOnReinit
                } else {
                    EgpsAction::None
                }
            }
            EgpsState::AcquiringFix { started_at, .. } => {
                if now - started_at >= self.config.position_dwell_ms {
                    // Dwell timeout: give up on this wake and go back to idle.
                    self.enter_idle(now)
                } else {
                    EgpsAction::None
                }
            }
            EgpsState::Batch {
                started_at,
                ends_at,
                is_batch,
            } => {
                if now >= ends_at {
                    if is_batch {
                        self.next_batch_start = started_at + self.config.batch_period_ms;
                    }
                    self.enter_idle(now);
                    EgpsAction::EndBatchIdlePowerOff
                } else {
                    EgpsAction::None
                }
            }
        }
    }

    /// Call when a valid `EgpsTime` fix has just been obtained while in
    /// `AcquiringFix`. Returns `EgpsAction::None` if called in any other
    /// state.
    pub fn fix_acquired(&mut self, now: i64) -> EgpsAction {
        match self.state {
            EgpsState::AcquiringFix { is_batch, .. } => {
                if is_batch {
                    self.state = EgpsState::Batch {
                        started_at: now,
                        // "0 gap" (continuous): the burst never ends -- the
                        // module just runs continuously, matching the
                        // historical always-on default.
                        ends_at: if self.config.is_continuous() {
                            i64::MAX
                        } else {
                            now + self.config.batch_duration_ms
                        },
                        is_batch: true,
                    };
                    EgpsAction::StartBatch
                } else if self.config.is_continuous() {
                    // "0 gap" position-only: there's no natural wake
                    // boundary to hang a short sample on -- it would just
                    // degenerate back into continuous streaming, so leave
                    // this (unused in practice) combination as always-idle
                    // (no batch, no idling, just re-acquire the fix).
                    self.enter_idle(now)
                } else {
                    // Position-only wake with a real gap between wakes:
                    // still stream briefly right after the fix so at least
                    // one packet's worth of raw samples is queued for the
                    // next sync, instead of this wake producing no data.
                    self.state = EgpsState::Batch {
                        started_at: now,
                        ends_at: now + EGPS_POSITION_SAMPLE_MS,
                        is_batch: false,
                    };
                    EgpsAction::StartBatch
                }
            }
            _ => EgpsAction::None,
        }
    }

    /// Transition to `Idle`, scheduling the next position wake. In
    /// continuous mode (`is_continuous`, "0 gap") this never actually
    /// idles or power-cycles the module -- it goes straight back to
    /// `AcquiringFix` and returns `EgpsAction::None`.
    fn enter_idle(&mut self, now: i64) -> EgpsAction {
        if self.config.is_continuous() {
            let is_burst = !self.config.is_position_only();
            self.state = EgpsState::AcquiringFix {
                started_at: now,
                is_batch: is_burst,
            };
            return EgpsAction::None;
        }
        let next_wake_at = now + self.config.position_interval_ms;
        self.state = EgpsState::Idle { next_wake_at };
        EgpsAction::EnterIdlePowerOff
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duty_config() -> EgpsDutyCycleConfig {
        // 10 min position interval, 2 min dwell, 20 min burst, 3 h period --
        // same as the plan's defaults.
        EgpsDutyCycleConfig::from_secs(600, 120, 1200, 10800)
    }

    fn position_only_config() -> EgpsDutyCycleConfig {
        EgpsDutyCycleConfig::from_secs(600, 120, 0, 10800)
    }

    #[test]
    fn position_only_never_starts_a_full_burst_but_still_samples_briefly() {
        let cfg = position_only_config();
        assert!(cfg.is_position_only());
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Immediate wake.
        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: false, .. }
        ));

        // Fix acquired -> a brief sample starts (not a full burst), so
        // there's still something queued for the next sync.
        let fix_at = 1_000;
        assert_eq!(sm.fix_acquired(fix_at), EgpsAction::StartBatch);
        assert!(sm.is_streaming());
        assert!(matches!(
            sm.state(),
            EgpsState::Batch { is_batch: false, .. }
        ));

        // The sample ends well before a full batch_duration_ms would.
        let ends_at = fix_at + EGPS_POSITION_SAMPLE_MS;
        assert!(EGPS_POSITION_SAMPLE_MS < cfg.batch_duration_ms.max(1_200_000));
        assert_eq!(sm.poll(ends_at - 1), EgpsAction::None);
        assert!(sm.is_streaming());
        let action = sm.poll(ends_at);
        assert_eq!(action, EgpsAction::EndBatchIdlePowerOff);
        assert!(!sm.is_streaming());

        // Run for many cycles: never see a *full* burst (i.e. one lasting
        // batch_duration_ms), only the brief post-fix sample each time.
        let mut now = ends_at;
        for _ in 0..50 {
            now += 60_000;
            let action = sm.poll(now);
            if action == EgpsAction::WakePowerOnReinit {
                let fix_at = now + 1_000;
                assert_eq!(sm.fix_acquired(fix_at), EgpsAction::StartBatch);
                assert!(matches!(
                    sm.state(),
                    EgpsState::Batch { is_batch: false, .. }
                ));
                now = fix_at + EGPS_POSITION_SAMPLE_MS;
                sm.poll(now);
                assert!(!sm.is_streaming());
            }
        }
    }

    #[test]
    fn duty_cycle_burst_starts_and_ends_on_schedule() {
        let cfg = duty_config();
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Immediate wake, burst allowed immediately (next_batch_start == 0).
        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: true, .. }
        ));

        // Fix acquired quickly -> burst starts.
        let fix_at = 5_000;
        assert_eq!(sm.fix_acquired(fix_at), EgpsAction::StartBatch);
        assert!(sm.is_streaming());

        // Burst should last batch_duration_ms from fix_at.
        let ends_at = fix_at + cfg.batch_duration_ms;
        assert_eq!(sm.poll(ends_at - 1), EgpsAction::None);
        assert!(sm.is_streaming());

        let action = sm.poll(ends_at);
        assert_eq!(action, EgpsAction::EndBatchIdlePowerOff);
        assert!(!sm.is_streaming());

        // Next position wake happens after position_interval_ms, and does
        // NOT immediately request another burst (since batch_period is
        // much longer than position_interval).
        let next_wake_at = match sm.state() {
            EgpsState::Idle { next_wake_at } => next_wake_at,
            other => panic!("expected Idle, got {:?}", other),
        };
        assert_eq!(next_wake_at, ends_at + cfg.position_interval_ms);

        sm.poll(next_wake_at);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: false, .. }
        ));

        // ... but once batch_period has elapsed since the last burst
        // start, the next position wake starts a new burst.
        let burst_start = fix_at;
        let mut now = next_wake_at;
        // fast-forward through position-only wakes until batch_period has
        // elapsed since burst_start. Each position-only wake now streams
        // briefly (EGPS_POSITION_SAMPLE_MS) before idling again, rather
        // than idling immediately.
        loop {
            let this_fix_at = now + 1_000;
            sm.fix_acquired(this_fix_at);
            assert!(matches!(
                sm.state(),
                EgpsState::Batch { is_batch: false, .. }
            ));
            let sample_ends = this_fix_at + EGPS_POSITION_SAMPLE_MS;
            sm.poll(sample_ends);
            if let EgpsState::Idle { next_wake_at } = sm.state() {
                now = next_wake_at;
            } else {
                panic!("expected Idle");
            }
            if now >= burst_start + cfg.batch_period_ms {
                break;
            }
            sm.poll(now);
        }
        // Every idle wake -- including this one -- fully powers off and
        // re-inits the module (no backup-sleep idle mode).
        let action = sm.poll(now);
        assert_eq!(action, EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: true, .. }
        ));
    }

    #[test]
    fn dwell_timeout_falls_back_to_idle() {
        let cfg = duty_config();
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        sm.poll(0);
        assert!(matches!(sm.state(), EgpsState::AcquiringFix { .. }));

        // No fix acquired; dwell elapses.
        let timeout_at = cfg.position_dwell_ms;
        assert_eq!(sm.poll(timeout_at - 1), EgpsAction::None);

        let action = sm.poll(timeout_at);
        assert_eq!(action, EgpsAction::EnterIdlePowerOff);
        match sm.state() {
            EgpsState::Idle { next_wake_at } => {
                assert_eq!(next_wake_at, timeout_at + cfg.position_interval_ms);
            }
            other => panic!("expected Idle, got {:?}", other),
        }
        assert!(!sm.is_streaming());
    }

    #[test]
    fn set_config_takes_effect_on_next_transition_only() {
        // Start in position-only mode so `is_batch` is never
        // latched true, keeping this test focused on the position-interval
        // config swap (a burst already latched-in under the old config runs
        // to completion under the old config's duration -- that's covered
        // implicitly by `enter_idle` always reading the *current* config).
        let cfg = position_only_config();
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Immediate wake under the original config.
        sm.poll(0);
        assert!(matches!(sm.state(), EgpsState::AcquiringFix { .. }));

        // Swap in a much longer position interval mid-flight -- the wake
        // already in progress is unaffected.
        let new_cfg = EgpsDutyCycleConfig::from_secs(3600, 120, 0, 10800);
        sm.set_config(new_cfg);
        assert_eq!(sm.config(), &new_cfg);

        // Fix acquired -> a brief sample starts, then enter_idle (once the
        // sample ends) uses the *new* config's position interval (1 h),
        // not the original (10 min).
        let fix_at = 1_000;
        sm.fix_acquired(fix_at);
        assert!(matches!(
            sm.state(),
            EgpsState::Batch { is_batch: false, .. }
        ));
        let sample_ends = fix_at + EGPS_POSITION_SAMPLE_MS;
        sm.poll(sample_ends);
        match sm.state() {
            EgpsState::Idle { next_wake_at, .. } => {
                assert_eq!(next_wake_at, sample_ends + new_cfg.position_interval_ms);
            }
            other => panic!("expected Idle, got {:?}", other),
        }
    }

    #[test]
    fn continuous_config_never_idles_or_power_cycles() {
        // "0 gap": position_interval_ms == 0.
        let cfg = EgpsDutyCycleConfig::from_secs(0, 120, 1200, 10800);
        assert!(cfg.is_continuous());
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Only the very first wake actually powers the module on.
        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: true, .. }
        ));

        // Fix acquired -> burst starts and never ends (runs continuously).
        assert_eq!(sm.fix_acquired(1_000), EgpsAction::StartBatch);
        assert!(sm.is_streaming());

        // Streaming continues indefinitely -- well past the configured
        // batch_duration_ms/batch_period_ms, no idle/power actions at all.
        let mut now = 1_000i64;
        for _ in 0..20 {
            now += 3_600_000; // +1 h
            assert_eq!(sm.poll(now), EgpsAction::None);
            assert!(sm.is_streaming());
        }
    }

    #[test]
    fn continuous_position_only_never_bursts_or_idles() {
        // "0 gap" combined with batches disabled (position-only, always on).
        let cfg = EgpsDutyCycleConfig::from_secs(0, 120, 0, 10800);
        assert!(cfg.is_continuous());
        assert!(cfg.is_position_only());
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: false, .. }
        ));

        // Fix acquired -> straight back to acquiring the next fix, no idle
        // action and never a burst.
        assert_eq!(sm.fix_acquired(1_000), EgpsAction::None);
        assert!(!sm.is_streaming());
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_batch: false, .. }
        ));
    }
}
