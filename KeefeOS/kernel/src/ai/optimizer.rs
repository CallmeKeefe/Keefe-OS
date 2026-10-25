//! Experimental performance recommendations for KeefeOS.
//!
//! This module computes deterministic heuristic recommendations. It does not
//! apply scheduler, VM, storage, or power settings: those runtime interfaces
//! are not implemented, and no trained machine-learning model is used here.

#![no_std]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static OPTIMIZER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static OPTIMIZATION_CYCLES: AtomicU64 = AtomicU64::new(0);

/// Initialize the recommendation engine.
pub fn init() {
    OPTIMIZER_INITIALIZED.store(true, Ordering::Release);
    log::info!("Experimental heuristic performance recommendations initialized");
}

/// Returns whether the recommendation engine was initialized.
pub fn is_initialized() -> bool {
    OPTIMIZER_INITIALIZED.load(Ordering::Acquire)
}

/// Snapshot of caller-supplied performance metrics.
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// CPU utilization estimate from 0.0 to 1.0.
    pub cpu_utilization: f32,
    /// Normalized memory throughput estimate.
    pub memory_throughput: f32,
    /// Normalized I/O latency estimate; higher means slower.
    pub io_latency_us: f32,
    /// Normalized context-switch rate estimate.
    pub context_switch_rate: f32,
    /// Cache-hit ratio estimate from 0.0 to 1.0.
    pub cache_hit_ratio: f32,
    /// Branch prediction accuracy estimate from 0.0 to 1.0.
    pub branch_prediction_accuracy: f32,
    /// Normalized power-consumption estimate.
    pub power_consumption: f32,
    /// Normalized temperature estimate.
    pub temperature: f32,
}

impl PerformanceMetrics {
    /// Create an empty metrics snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert metrics to an eight-value normalized vector.
    pub fn to_vector(&self) -> [f32; 8] {
        [
            self.cpu_utilization.clamp(0.0, 1.0),
            self.memory_throughput.max(0.0),
            self.io_latency_us.max(0.0),
            self.context_switch_rate.max(0.0),
            self.cache_hit_ratio.clamp(0.0, 1.0),
            self.branch_prediction_accuracy.clamp(0.0, 1.0),
            self.power_consumption.clamp(0.0, 1.0),
            self.temperature.clamp(0.0, 1.0),
        ]
    }
}

/// Suggested tuning values. These values are advisory only.
#[derive(Debug, Clone)]
pub struct KernelTunables {
    /// Suggested scheduler quantum in milliseconds.
    pub scheduler_quantum_ms: u32,
    /// Suggested page-reclaim threshold percentage.
    pub page_reclaim_threshold: u8,
    /// Suggested read-ahead page count.
    pub readahead_pages: u8,
    /// Suggested dirty-page ratio percentage.
    pub dirty_ratio: u8,
    /// Suggested swappiness from 0 to 100.
    pub swappiness: u8,
    /// Whether VM overcommit would be enabled.
    pub vm_overcommit: bool,
    /// Whether preemption would be enabled.
    pub preempt_enable: bool,
}

impl Default for KernelTunables {
    fn default() -> Self {
        Self {
            scheduler_quantum_ms: 20,
            page_reclaim_threshold: 70,
            readahead_pages: 128,
            dirty_ratio: 20,
            swappiness: 60,
            vm_overcommit: false,
            preempt_enable: false,
        }
    }
}

/// One advisory tuning recommendation.
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    /// Name of the proposed parameter.
    pub target_parameter: &'static str,
    /// Current reference value used to formulate the recommendation.
    pub current_value: i32,
    /// Suggested value; it is not applied by this module.
    pub recommended_value: i32,
    /// Potential improvement estimate; it is not benchmark-verified.
    pub expected_improvement: f32,
    /// Heuristic score, not ML-model confidence.
    pub confidence: f32,
}

/// Coarse workload category inferred from the supplied metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkloadType {
    /// Not enough information to classify.
    Unknown,
    /// High CPU utilization with relatively low I/O delay.
    ComputeBound,
    /// Low cache hit ratio suggesting memory pressure.
    MemoryBound,
    /// High observed I/O latency.
    IOBound,
    /// Low CPU utilization with a high context-switch estimate.
    Interactive,
    /// No single characteristic dominates.
    Mixed,
    /// Caller-supplied metrics suggest a latency-sensitive workload.
    RealTime,
}

/// Classify workload characteristics using deterministic thresholds.
pub fn classify_workload(metrics: &PerformanceMetrics) -> WorkloadType {
    if !is_initialized() {
        return WorkloadType::Unknown;
    }
    if metrics.temperature >= 0.95 && metrics.io_latency_us >= 0.75 {
        WorkloadType::RealTime
    } else if metrics.io_latency_us >= 0.75 {
        WorkloadType::IOBound
    } else if metrics.cache_hit_ratio <= 0.35 {
        WorkloadType::MemoryBound
    } else if metrics.cpu_utilization >= 0.75 && metrics.io_latency_us <= 0.35 {
        WorkloadType::ComputeBound
    } else if metrics.context_switch_rate >= 0.7 && metrics.cpu_utilization <= 0.6 {
        WorkloadType::Interactive
    } else if metrics.cpu_utilization > 0.0 || metrics.memory_throughput > 0.0 {
        WorkloadType::Mixed
    } else {
        WorkloadType::Unknown
    }
}

/// Generate advisory recommendations; no tunable is modified.
pub fn optimize(
    workload: &WorkloadType,
    metrics: &PerformanceMetrics,
) -> Vec<OptimizationRecommendation> {
    let mut recommendations = Vec::new();
    match workload {
        WorkloadType::ComputeBound => recommendations.push(OptimizationRecommendation {
            target_parameter: "scheduler_quantum_ms",
            current_value: 20,
            recommended_value: 30,
            expected_improvement: 0.0,
            confidence: 0.45,
        }),
        WorkloadType::MemoryBound => {
            recommendations.push(OptimizationRecommendation {
                target_parameter: "page_reclaim_threshold",
                current_value: 70,
                recommended_value: 60,
                expected_improvement: 0.0,
                confidence: 0.4,
            });
            recommendations.push(OptimizationRecommendation {
                target_parameter: "readahead_pages",
                current_value: 128,
                recommended_value: if metrics.memory_throughput > 0.65 { 256 } else { 64 },
                expected_improvement: 0.0,
                confidence: 0.35,
            });
        }
        WorkloadType::IOBound => {
            recommendations.push(OptimizationRecommendation {
                target_parameter: "readahead_pages",
                current_value: 128,
                recommended_value: 256,
                expected_improvement: 0.0,
                confidence: 0.4,
            });
            recommendations.push(OptimizationRecommendation {
                target_parameter: "dirty_ratio",
                current_value: 20,
                recommended_value: 25,
                expected_improvement: 0.0,
                confidence: 0.35,
            });
        }
        WorkloadType::Interactive => recommendations.push(OptimizationRecommendation {
            target_parameter: "scheduler_quantum_ms",
            current_value: 20,
            recommended_value: 10,
            expected_improvement: 0.0,
            confidence: 0.45,
        }),
        WorkloadType::RealTime => recommendations.push(OptimizationRecommendation {
            target_parameter: "scheduler_quantum_ms",
            current_value: 20,
            recommended_value: 5,
            expected_improvement: 0.0,
            confidence: 0.25,
        }),
        WorkloadType::Unknown | WorkloadType::Mixed => {
            if metrics.cpu_utilization > 0.0 || metrics.memory_throughput > 0.0 {
                recommendations.push(OptimizationRecommendation {
                    target_parameter: "scheduler_quantum_ms",
                    current_value: 20,
                    recommended_value: 20,
                    expected_improvement: 0.0,
                    confidence: 0.2,
                });
            }
        }
    }
    recommendations
}

/// Attempt to apply a tunable. Returns false because runtime setters are not
/// implemented; this prevents falsely counting a recommendation as applied.
pub fn apply_tunable(parameter: &str, value: i32) -> bool {
    log::warn!(
        "Cannot apply tunable '{}'={} yet: runtime tunable interface is not implemented",
        parameter,
        value
    );
    false
}

/// Run one analysis cycle and return the advisory report.
pub fn optimization_cycle(metrics: &PerformanceMetrics) -> OptimizationStats {
    OPTIMIZATION_CYCLES.fetch_add(1, Ordering::Relaxed);
    let workload = classify_workload(metrics);
    let recommendations = optimize(&workload, metrics);
    OptimizationStats {
        cycles_completed: OPTIMIZATION_CYCLES.load(Ordering::Relaxed),
        improvements_made: 0,
        current_workload: workload,
        active_recommendations: recommendations.len().min(u8::MAX as usize) as u8,
    }
}

/// Current counters for the experimental recommender.
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    /// Number of recommendation cycles evaluated.
    pub cycles_completed: u64,
    /// Number of changes actually applied; currently zero.
    pub improvements_made: u64,
    /// Workload category inferred for the most recent cycle.
    pub current_workload: WorkloadType,
    /// Number of suggestions generated for the most recent cycle.
    pub active_recommendations: u8,
}

impl core::fmt::Display for OptimizationStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Experimental Performance Recommender Statistics:")?;
        writeln!(f, "  Recommendation cycles: {}", self.cycles_completed)?;
        writeln!(f, "  Applied improvements: {}", self.improvements_made)?;
        writeln!(f, "  Workload classification: {:?}", self.current_workload)?;
        writeln!(f, "  Active recommendations: {}", self.active_recommendations)?;
        Ok(())
    }
}

/// Return a coarse 0–100 efficiency score from normalized input metrics.
pub fn calculate_efficiency_score(metrics: &PerformanceMetrics) -> u8 {
    let performance = metrics.cpu_utilization.clamp(0.0, 1.0) * 0.3
        + metrics.memory_throughput.clamp(0.0, 1.0) * 0.3
        + metrics.cache_hit_ratio.clamp(0.0, 1.0) * 0.2
        + metrics.branch_prediction_accuracy.clamp(0.0, 1.0) * 0.2;
    let power_efficiency = 1.0 - metrics.power_consumption.clamp(0.0, 1.0);
    ((performance * 0.7 + power_efficiency * 0.3).clamp(0.0, 1.0) * 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifier_uses_high_io_latency() {
        OPTIMIZER_INITIALIZED.store(true, Ordering::Release);
        let metrics = PerformanceMetrics {
            io_latency_us: 0.9,
            ..PerformanceMetrics::default()
        };
        assert_eq!(classify_workload(&metrics), WorkloadType::IOBound);
    }

    #[test]
    fn recommendations_are_not_applied() {
        assert!(!apply_tunable("scheduler_quantum_ms", 10));
    }

    #[test]
    fn efficiency_score_stays_in_range() {
        let score = calculate_efficiency_score(&PerformanceMetrics::default());
        assert!((0..=100).contains(&score));
    }
}
