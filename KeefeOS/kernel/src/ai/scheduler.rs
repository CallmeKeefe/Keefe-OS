//! Experimental heuristic scheduler for KeefeOS.
 //!
 //! This module is a deterministic heuristic fallback, not an operating-system
 //! process scheduler and not a trained ML model. It is intentionally safe to
 //! initialize before any user-space scheduling infrastructure exists.


use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use spin::Mutex;

static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static PREDICTION_MODE: AtomicU32 = AtomicU32::new(1);
static SCHEDULER_STATS: Mutex<SchedulerStats> = Mutex::new(SchedulerStats {
    total_decisions: 0,
    accurate_predictions: 0,
    accuracy_percent: 0,
    avg_time_slice: 20,
});

/// Initialize the experimental heuristic scheduler.
pub fn init() {
    SCHEDULER_INITIALIZED.store(true, Ordering::Release);
    log::info!("KeefeOS heuristic scheduling policy initialized (not preemptive)");
}

/// Returns whether the policy has been initialized.
pub fn is_initialized() -> bool {
    SCHEDULER_INITIALIZED.load(Ordering::Acquire)
}

/// Normalized workload features supplied by a caller.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessFeatures {
    /// CPU utilization estimate from 0.0 to 1.0.
    pub cpu_usage: f32,
    /// Memory pressure estimate from 0.0 to 1.0.
    pub memory_pressure: f32,
    /// Ratio of observed time waiting on I/O, from 0.0 to 1.0.
    pub io_wait_ratio: f32,
    /// Nice-like adjustment, nominally between -20 and 19.
    pub nice_value: f32,
    /// Interactivity estimate from 0.0 to 1.0.
    pub interactivity_score: f32,
    /// Cache-miss rate estimate from 0.0 to 1.0.
    pub cache_miss_rate: f32,
    /// Branch-misprediction estimate from 0.0 to 1.0.
    pub branch_misprediction: f32,
    /// Normalized context-switch count.
    pub context_switches: f32,
}

impl ProcessFeatures {
    /// Construct an all-zero feature set.
    pub const fn new() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_pressure: 0.0,
            io_wait_ratio: 0.0,
            nice_value: 0.0,
            interactivity_score: 0.0,
            cache_miss_rate: 0.0,
            branch_misprediction: 0.0,
            context_switches: 0.0,
        }
    }

    /// Convert the features into a stable eight-element vector.
    pub const fn to_vector(&self) -> [f32; 8] {
        [
            self.cpu_usage,
            self.memory_pressure,
            self.io_wait_ratio,
            self.nice_value,
            self.interactivity_score,
            self.cache_miss_rate,
            self.branch_misprediction,
            self.context_switches,
        ]
    }
}

/// Result of the heuristic scheduling policy.
#[derive(Debug, Clone, Copy)]
pub struct SchedulingDecision {
    /// Relative priority from 0 (highest) through 9 (lowest).
    pub priority: u8,
    /// Suggested time slice in milliseconds.
    pub time_slice_ms: u32,
    /// Whether a caller should consider preemption.
    pub should_preempt: bool,
    /// Confidence estimate for this heuristic decision, from 0.0 to 1.0.
    pub confidence: f32,
}

impl Default for SchedulingDecision {
    fn default() -> Self {
        Self {
            priority: 5,
            time_slice_ms: 20,
            should_preempt: false,
            confidence: 0.0,
        }
    }
}

/// Schedule one workload using deterministic rules.
pub fn schedule(features: &ProcessFeatures) -> SchedulingDecision {
    let mode = get_prediction_mode();
    let mut priority: i16 = 5;

    if features.interactivity_score > 0.8 {
        priority -= 3;
    } else if features.interactivity_score > 0.5 {
        priority -= 1;
    }

    if features.io_wait_ratio > 0.5 {
        priority -= 1;
    }
    if features.cpu_usage > 0.7 {
        priority += 1;
    }
    if features.memory_pressure > 0.8 {
        priority += 1;
    }
    if features.nice_value > 0.0 {
        priority += 1;
    } else if features.nice_value < 0.0 {
        priority -= 1;
    }

    if mode == 0 {
        priority = (priority + 1).min(9);
    } else if mode == 2 {
        priority = (priority - 1).max(0);
    }

    let time_slice = calculate_time_slice(features, mode);
    let decision = SchedulingDecision {
        priority: priority.clamp(0, 9) as u8,
        time_slice_ms: time_slice,
        should_preempt: features.interactivity_score > 0.8 && priority < 3,
        // No trained predictor is wired into the current kernel; do not report
        // model confidence as if this were an ML inference result.
        confidence: 0.35,
    };

    let mut stats = SCHEDULER_STATS.lock();
    let old_count = stats.total_decisions;
    stats.total_decisions = old_count.saturating_add(1);
    if old_count == 0 {
        stats.avg_time_slice = time_slice;
    } else {
        let weighted = (stats.avg_time_slice as u64)
            .saturating_mul(old_count)
            .saturating_add(time_slice as u64);
        stats.avg_time_slice = (weighted / (old_count + 1)) as u32;
    }

    decision
}

fn calculate_time_slice(features: &ProcessFeatures, mode: u8) -> u32 {
    let mut time_slice = 20i32;

    if features.cpu_usage > 0.7 {
        time_slice += 10;
    }
    if features.io_wait_ratio > 0.5 {
        time_slice -= 5;
    }
    if features.interactivity_score > 0.8 {
        time_slice -= 5;
    }
    if features.cache_miss_rate > 0.6 {
        time_slice += 5;
    }

    match mode {
        0 => time_slice += 5,
        2 => time_slice -= 3,
        _ => {}
    }

    time_slice.clamp(5, 100) as u32
}

/// Feed a caller-observed outcome back into the lightweight statistics.
pub fn train(_decision: &SchedulingDecision, outcome_good: bool) {
    let mut stats = SCHEDULER_STATS.lock();
    if outcome_good {
        stats.accurate_predictions = stats.accurate_predictions.saturating_add(1);
    }

    if stats.total_decisions > 0 {
        stats.accuracy_percent = ((stats.accurate_predictions as u128 * 100)
            / stats.total_decisions as u128)
            .min(100) as u8;
    }
}

/// Return aggregate policy statistics.
pub fn get_stats() -> SchedulerStats {
    *SCHEDULER_STATS.lock()
}

/// Statistics for the heuristic policy; accuracy is caller-reported feedback.
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedulerStats {
    /// Number of decisions returned by the policy.
    pub total_decisions: u64,
    /// Number of caller-reported positive outcomes.
    pub accurate_predictions: u64,
    /// Percentage of decisions with positive caller-reported outcomes.
    pub accuracy_percent: u8,
    /// Running average suggested time slice in milliseconds.
    pub avg_time_slice: u32,
}

impl core::fmt::Display for SchedulerStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Heuristic Scheduler Statistics:")?;
        writeln!(f, "  Decisions: {}", self.total_decisions)?;
        writeln!(f, "  Positive outcomes: {}", self.accurate_predictions)?;
        writeln!(f, "  Positive outcome rate: {}%", self.accuracy_percent)?;
        writeln!(f, "  Average time slice: {} ms", self.avg_time_slice)?;
        Ok(())
    }
}

/// Set the policy mode: 0 conservative, 1 balanced, or 2 aggressive.
pub fn set_prediction_mode(mode: u8) {
    let bounded = mode.min(2);
    PREDICTION_MODE.store(bounded as u32, Ordering::Relaxed);
    log::debug!("Scheduler policy mode set to {}", bounded);
}

/// Return the configured policy mode.
pub fn get_prediction_mode() -> u8 {
    PREDICTION_MODE.load(Ordering::Relaxed).min(2) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_bounded() {
        let decision = schedule(&ProcessFeatures::new());
        assert!(decision.priority <= 9);
        assert!((5..=100).contains(&decision.time_slice_ms));
    }

    #[test]
    fn interactive_workloads_get_shorter_slices() {
        let interactive = ProcessFeatures {
            interactivity_score: 0.95,
            ..ProcessFeatures::new()
        };
        let cpu_bound = ProcessFeatures {
            cpu_usage: 0.9,
            ..ProcessFeatures::new()
        };
        assert!(schedule(&interactive).time_slice_ms < schedule(&cpu_bound).time_slice_ms);
    }

    #[test]
    fn mode_is_bounded() {
        set_prediction_mode(255);
        assert_eq!(get_prediction_mode(), 2);
        set_prediction_mode(1);
    }
}
