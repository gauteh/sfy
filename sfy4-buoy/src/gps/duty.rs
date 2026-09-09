//! Duty-cycled EGPS state machine (feature `egps-duty-cycle`).
//!
//! This is pure, host-testable logic: it takes the current RTC time (in
//! milliseconds) as input and returns what the caller should do with the
//! GPS module (power it on/off, resume from backup sleep, set output rate,
//! start/stop feeding samples into `GpsCollector`). It does not touch any
//! hardware itself.
//!
//! Three states:
//!
//! - [`EgpsState::Idle`]: GPS is powered down or in UBX backup sleep
//!   (depending on the idle gap length vs. `sleep_threshold_ms`). No
//!   samples are collected.
//! - [`EgpsState::AcquiringFix`]: GPS is powered/resumed and running at a
//!   low output rate until a valid fix is obtained (or the dwell time
//!   elapses).
//! - [`EgpsState::SpectrumBurst`]: a high-rate burst is in progress;
//!   samples are fed to `GpsCollector`/`EGPSQ` for the configured duration.

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
    pub spectrum_duration_ms: i64,
    /// Start-to-start interval between spectrum bursts.
    pub spectrum_period_ms: i64,
    /// Idle gaps <= this use UBX backup sleep; gaps above this fully power
    /// off via the `d8` GPIO.
    pub sleep_threshold_ms: i64,
}

impl EgpsDutyCycleConfig {
    pub const fn from_secs(
        position_interval_s: u32,
        position_dwell_s: u32,
        spectrum_duration_s: u32,
        spectrum_period_s: u32,
        sleep_threshold_s: u32,
    ) -> Self {
        EgpsDutyCycleConfig {
            position_interval_ms: position_interval_s as i64 * 1000,
            position_dwell_ms: position_dwell_s as i64 * 1000,
            spectrum_duration_ms: spectrum_duration_s as i64 * 1000,
            spectrum_period_ms: spectrum_period_s as i64 * 1000,
            sleep_threshold_ms: sleep_threshold_s as i64 * 1000,
        }
    }

    /// `true` if bursts are disabled entirely (position-only mode).
    pub const fn is_position_only(&self) -> bool {
        self.spectrum_duration_ms <= 0
    }
}

/// How the GPS module should idle between wakes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleMode {
    /// Keep `d8` power on, use UBX backup `sleep()`. Fast resume, some
    /// standby current.
    BackupSleep,
    /// Cut `d8` power entirely. Near-zero standby current, needs a cold
    /// re-init on next wake.
    PowerOff,
}

/// Decide which idle strategy to use for an idle gap of `gap_ms`, given the
/// configured `threshold_ms`.
pub const fn idle_mode_for_gap(gap_ms: i64, threshold_ms: i64) -> IdleMode {
    if gap_ms <= threshold_ms {
        IdleMode::BackupSleep
    } else {
        IdleMode::PowerOff
    }
}

/// State of the duty-cycle state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EgpsState {
    /// GPS idle (powered down or in backup sleep). Will wake at
    /// `next_wake_at` using `mode` idling strategy.
    Idle { next_wake_at: i64, mode: IdleMode },
    /// GPS powered/resumed, waiting for a valid fix (or dwell timeout).
    AcquiringFix { started_at: i64, is_spectrum_burst: bool },
    /// High-rate spectrum burst in progress, started at `started_at`,
    /// scheduled to end at `ends_at`.
    SpectrumBurst { started_at: i64, ends_at: i64 },
}

/// Action the caller should take in response to a `poll`/`fix_acquired` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EgpsAction {
    /// Nothing to do.
    None,
    /// Enter backup sleep (keep `d8` powered).
    EnterIdleBackupSleep,
    /// Fully power off via `d8`.
    EnterIdlePowerOff,
    /// Wake from backup sleep: call `resume()`, set output rate low (1 Hz).
    WakeResume,
    /// Wake from a full power-off: re-run cold-init sequence, set output
    /// rate low (1 Hz).
    WakePowerOnReinit,
    /// Valid fix obtained and this wake starts a spectrum burst: restore
    /// full output rate and start feeding `GpsCollector`.
    StartSpectrumBurst,
    /// Spectrum burst ended: flush any partial packet, stop feeding
    /// samples, then enter backup sleep.
    EndSpectrumBurstIdleBackupSleep,
    /// Spectrum burst ended: flush any partial packet, stop feeding
    /// samples, then fully power off.
    EndSpectrumBurstIdlePowerOff,
}

/// Duty-cycled EGPS state machine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EgpsDutyCycle {
    config: EgpsDutyCycleConfig,
    state: EgpsState,
    /// Time (RTC ms) at which the next spectrum burst is allowed to start.
    next_spectrum_start: i64,
}

impl EgpsDutyCycle {
    /// Create a new state machine, starting in `Idle` with an immediate
    /// wake (so the very first `poll` call triggers an acquisition), and
    /// bursts allowed to start immediately (subject to
    /// `config.is_position_only()`).
    pub fn new(config: EgpsDutyCycleConfig, now: i64) -> Self {
        EgpsDutyCycle {
            config,
            state: EgpsState::Idle {
                next_wake_at: now,
                mode: IdleMode::PowerOff,
            },
            next_spectrum_start: now,
        }
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
        matches!(self.state, EgpsState::SpectrumBurst { .. })
    }

    /// Drive the state machine forward in time. Call regularly (e.g. from
    /// the main loop) with the current RTC time in milliseconds. Does not
    /// itself handle a freshly-acquired fix -- call [`Self::fix_acquired`]
    /// for that.
    pub fn poll(&mut self, now: i64) -> EgpsAction {
        match self.state {
            EgpsState::Idle { next_wake_at, mode } => {
                if now >= next_wake_at {
                    let is_burst =
                        !self.config.is_position_only() && now >= self.next_spectrum_start;
                    self.state = EgpsState::AcquiringFix {
                        started_at: now,
                        is_spectrum_burst: is_burst,
                    };
                    match mode {
                        IdleMode::BackupSleep => EgpsAction::WakeResume,
                        IdleMode::PowerOff => EgpsAction::WakePowerOnReinit,
                    }
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
            EgpsState::SpectrumBurst { started_at, ends_at } => {
                if now >= ends_at {
                    self.next_spectrum_start = started_at + self.config.spectrum_period_ms;
                    match self.enter_idle(now) {
                        EgpsAction::EnterIdleBackupSleep => {
                            EgpsAction::EndSpectrumBurstIdleBackupSleep
                        }
                        EgpsAction::EnterIdlePowerOff => EgpsAction::EndSpectrumBurstIdlePowerOff,
                        _ => unreachable!("enter_idle only returns idle-entry actions"),
                    }
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
            EgpsState::AcquiringFix { is_spectrum_burst, .. } => {
                if is_spectrum_burst {
                    self.state = EgpsState::SpectrumBurst {
                        started_at: now,
                        ends_at: now + self.config.spectrum_duration_ms,
                    };
                    EgpsAction::StartSpectrumBurst
                } else {
                    self.enter_idle(now)
                }
            }
            _ => EgpsAction::None,
        }
    }

    /// Transition to `Idle`, scheduling the next position wake and picking
    /// the idle strategy based on the gap length vs. `sleep_threshold_ms`.
    fn enter_idle(&mut self, now: i64) -> EgpsAction {
        let next_wake_at = now + self.config.position_interval_ms;
        let gap = next_wake_at - now;
        let mode = idle_mode_for_gap(gap, self.config.sleep_threshold_ms);
        self.state = EgpsState::Idle { next_wake_at, mode };
        match mode {
            IdleMode::BackupSleep => EgpsAction::EnterIdleBackupSleep,
            IdleMode::PowerOff => EgpsAction::EnterIdlePowerOff,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn duty_config() -> EgpsDutyCycleConfig {
        // 10 min position interval, 2 min dwell, 20 min burst, 3 h period, 30
        // min sleep threshold -- same as the plan's defaults.
        EgpsDutyCycleConfig::from_secs(600, 120, 1200, 10800, 1800)
    }

    fn position_only_config() -> EgpsDutyCycleConfig {
        EgpsDutyCycleConfig::from_secs(600, 120, 0, 10800, 1800)
    }

    #[test]
    fn position_only_never_starts_burst() {
        let cfg = position_only_config();
        assert!(cfg.is_position_only());
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Immediate wake.
        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_spectrum_burst: false, .. }
        ));

        // Fix acquired -> straight back to idle, no burst.
        let action = sm.fix_acquired(1_000);
        assert!(matches!(
            action,
            EgpsAction::EnterIdleBackupSleep | EgpsAction::EnterIdlePowerOff
        ));
        assert!(!sm.is_streaming());

        // Run for many cycles: never see a StartSpectrumBurst action.
        let mut now = 1_000i64;
        for _ in 0..50 {
            now += 60_000;
            let action = sm.poll(now);
            assert_ne!(action, EgpsAction::StartSpectrumBurst);
            if let EgpsAction::WakeResume | EgpsAction::WakePowerOnReinit = action {
                let action = sm.fix_acquired(now + 1_000);
                assert_ne!(action, EgpsAction::StartSpectrumBurst);
                now += 1_000;
            }
        }
    }

    #[test]
    fn duty_cycle_burst_starts_and_ends_on_schedule() {
        let cfg = duty_config();
        let mut sm = EgpsDutyCycle::new(cfg, 0);

        // Immediate wake, burst allowed immediately (next_spectrum_start == 0).
        assert_eq!(sm.poll(0), EgpsAction::WakePowerOnReinit);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_spectrum_burst: true, .. }
        ));

        // Fix acquired quickly -> burst starts.
        let fix_at = 5_000;
        assert_eq!(sm.fix_acquired(fix_at), EgpsAction::StartSpectrumBurst);
        assert!(sm.is_streaming());

        // Burst should last spectrum_duration_ms from fix_at.
        let ends_at = fix_at + cfg.spectrum_duration_ms;
        assert_eq!(sm.poll(ends_at - 1), EgpsAction::None);
        assert!(sm.is_streaming());

        let action = sm.poll(ends_at);
        assert!(matches!(
            action,
            EgpsAction::EndSpectrumBurstIdleBackupSleep | EgpsAction::EndSpectrumBurstIdlePowerOff
        ));
        assert!(!sm.is_streaming());

        // Next position wake happens after position_interval_ms, and does
        // NOT immediately request another burst (since spectrum_period is
        // much longer than position_interval).
        let (next_wake_at, _) = match sm.state() {
            EgpsState::Idle { next_wake_at, mode } => (next_wake_at, mode),
            other => panic!("expected Idle, got {:?}", other),
        };
        assert_eq!(next_wake_at, ends_at + cfg.position_interval_ms);

        sm.poll(next_wake_at);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_spectrum_burst: false, .. }
        ));

        // ... but once spectrum_period has elapsed since the last burst
        // start, the next position wake starts a new burst.
        let burst_start = fix_at;
        let mut now = next_wake_at;
        // fast-forward through position-only wakes until spectrum_period has
        // elapsed since burst_start.
        loop {
            sm.fix_acquired(now + 1_000);
            if let EgpsState::Idle { next_wake_at, .. } = sm.state() {
                now = next_wake_at;
            } else {
                panic!("expected Idle");
            }
            if now >= burst_start + cfg.spectrum_period_ms {
                break;
            }
            sm.poll(now);
        }
        // position_interval_ms (10 min) <= sleep_threshold_ms (30 min), so
        // every idle gap after the first uses backup sleep, not power-off.
        let action = sm.poll(now);
        assert_eq!(action, EgpsAction::WakeResume);
        assert!(matches!(
            sm.state(),
            EgpsState::AcquiringFix { is_spectrum_burst: true, .. }
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
        assert!(matches!(
            action,
            EgpsAction::EnterIdleBackupSleep | EgpsAction::EnterIdlePowerOff
        ));
        match sm.state() {
            EgpsState::Idle { next_wake_at, .. } => {
                assert_eq!(next_wake_at, timeout_at + cfg.position_interval_ms);
            }
            other => panic!("expected Idle, got {:?}", other),
        }
        assert!(!sm.is_streaming());
    }

    #[test]
    fn idle_mode_threshold_decision() {
        let threshold_ms = 30 * 60 * 1000;
        assert_eq!(
            idle_mode_for_gap(threshold_ms, threshold_ms),
            IdleMode::BackupSleep
        );
        assert_eq!(
            idle_mode_for_gap(threshold_ms - 1, threshold_ms),
            IdleMode::BackupSleep
        );
        assert_eq!(
            idle_mode_for_gap(threshold_ms + 1, threshold_ms),
            IdleMode::PowerOff
        );
        assert_eq!(idle_mode_for_gap(0, threshold_ms), IdleMode::BackupSleep);
    }
}
