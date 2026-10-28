//! Experimental heuristic memory-access predictor.
//!
//! This module provides deterministic heuristics only. It does not train a
//! model, modify hardware page tables, or prove that a page fault was avoided.
//! The APIs are retained for callers while real telemetry and paging are built.


use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static PREDICTOR_INITIALIZED: AtomicBool = AtomicBool::new(false);
static PAGE_FAULTS_PREVENTED: AtomicU64 = AtomicU64::new(0);

/// Initialize the heuristic predictor.
pub fn init() {
    PREDICTOR_INITIALIZED.store(true, Ordering::Release);
    log::info!("Experimental heuristic memory predictor initialized");
}

/// Returns whether the heuristic predictor was initialized.
pub fn is_initialized() -> bool {
    PREDICTOR_INITIALIZED.load(Ordering::Acquire)
}

/// Summary of one observed memory access pattern.
#[derive(Debug, Clone, Default)]
pub struct MemoryPattern {
    /// Sequential-access estimate from 0.0 to 1.0.
    pub sequential_access: f32,
    /// Common byte stride; positive values predict ascending addresses.
    pub stride: i32,
    /// Temporal-locality estimate from 0.0 to 1.0.
    pub temporal_locality: f32,
    /// Spatial-locality estimate from 0.0 to 1.0.
    pub spatial_locality: f32,
    /// Observed write ratio from 0.0 to 1.0.
    pub write_ratio: f32,
    /// Normalized access-frequency estimate.
    pub access_frequency: f32,
    /// Time since the page was last observed, in milliseconds.
    pub page_age_ms: u64,
}

impl MemoryPattern {
    /// Create an empty observation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the pattern as a seven-value vector for experimentation.
    pub fn to_vector(&self) -> [f32; 7] {
        [
            self.sequential_access.clamp(0.0, 1.0),
            (self.stride as f32 / 4096.0).clamp(-1.0, 1.0),
            self.temporal_locality.clamp(0.0, 1.0),
            self.spatial_locality.clamp(0.0, 1.0),
            self.write_ratio.clamp(0.0, 1.0),
            self.access_frequency.max(0.0),
            ((self.page_age_ms.min(10_000)) as f32) / 10_000.0,
        ]
    }
}

/// Heuristic recommendation to prefetch pages.
#[derive(Debug, Clone)]
pub struct PrefetchRecommendation {
    /// Whether the current heuristic recommends considering prefetch.
    pub should_prefetch: bool,
    /// Suggested number of pages; this is advisory only.
    pub pages_to_prefetch: u8,
    /// Heuristic score from 0.0 to 1.0, not ML-model confidence.
    pub confidence: f32,
    /// Candidate addresses inferred from the observed stride.
    pub predicted_addresses: [u64; 4],
}

impl Default for PrefetchRecommendation {
    fn default() -> Self {
        Self {
            should_prefetch: false,
            pages_to_prefetch: 0,
            confidence: 0.0,
            predicted_addresses: [0; 4],
        }
    }
}

/// Recommend candidate addresses from simple stride and locality heuristics.
pub fn predict_page_fault(
    current_addr: u64,
    pattern: &MemoryPattern,
) -> PrefetchRecommendation {
    if !is_initialized() {
        return PrefetchRecommendation::default();
    }

    let sequential = pattern.sequential_access.clamp(0.0, 1.0);
    let spatial = pattern.spatial_locality.clamp(0.0, 1.0);
    let frequency = pattern.access_frequency.max(0.0);
    let stride = pattern.stride.unsigned_abs() as u64;
    let should_prefetch =
        sequential >= 0.65 && spatial >= 0.35 && frequency >= 0.1 && stride > 0;
    let confidence = (sequential * 0.5 + spatial * 0.3 + (frequency / (frequency + 1.0)) * 0.2)
        .clamp(0.0, 1.0);

    let mut rec = PrefetchRecommendation {
        should_prefetch,
        pages_to_prefetch: if should_prefetch { 4 } else { 0 },
        confidence,
        predicted_addresses: [0; 4],
    };

    if should_prefetch {
        for i in 0..rec.predicted_addresses.len() {
            let delta = stride.saturating_mul((i as u64) + 1);
            rec.predicted_addresses[i] = if pattern.stride >= 0 {
                current_addr.checked_add(delta).unwrap_or(0)
            } else {
                current_addr.checked_sub(delta).unwrap_or(0)
            };
        }
    }

    rec
}

/// Score a page for eviction. Higher scores indicate a better heuristic candidate.
pub fn score_page_for_eviction(pattern: &MemoryPattern) -> f32 {
    let temporal = pattern.temporal_locality.clamp(0.0, 1.0);
    let spatial = pattern.spatial_locality.clamp(0.0, 1.0);
    let frequency = pattern.access_frequency.max(0.0);
    let age = (pattern.page_age_ms.min(60_000) as f32) / 60_000.0;

    (0.45 * (1.0 - temporal)
        + 0.25 * (1.0 - spatial)
        + 0.20 * age
        + 0.10 * (1.0 / (1.0 + frequency)))
        .clamp(0.0, 1.0)
}

/// Estimate a rough working-set size from locality observations.
pub fn estimate_working_set(patterns: &[MemoryPattern]) -> usize {
    if patterns.is_empty() {
        return 0;
    }

    let locality_sum: f32 = patterns
        .iter()
        .map(|p| {
            (p.temporal_locality.clamp(0.0, 1.0)
                + p.spatial_locality.clamp(0.0, 1.0))
                / 2.0
        })
        .sum();
    let average_locality = locality_sum / patterns.len() as f32;
    let estimated_pages = 1000.0 * (1.0 - average_locality * 0.7);
    estimated_pages.clamp(1.0, 1000.0) as usize
}

/// Estimate memory pressure from recent observations.
pub fn forecast_memory_pressure(recent_patterns: &[MemoryPattern]) -> f32 {
    if recent_patterns.is_empty() {
        return 0.5;
    }

    let count = recent_patterns.len() as f32;
    let average_frequency = recent_patterns
        .iter()
        .map(|p| p.access_frequency.max(0.0) / (p.access_frequency.max(0.0) + 1.0))
        .sum::<f32>()
        / count;
    let average_age = recent_patterns
        .iter()
        .map(|p| (p.page_age_ms.min(10_000) as f32) / 10_000.0)
        .sum::<f32>()
        / count;

    (average_frequency * 0.6 + (1.0 - average_age) * 0.4).clamp(0.0, 1.0)
}

/// Return the number of heuristic prefetch outcomes reported by callers.
pub fn get_prevented_faults() -> u64 {
    PAGE_FAULTS_PREVENTED.load(Ordering::Relaxed)
}

/// Record a successful prefetch outcome reported by a caller.
///
/// The counter is caller-reported; it is not independently verified telemetry.
pub fn record_successful_prefetch() {
    PAGE_FAULTS_PREVENTED.fetch_add(1, Ordering::Relaxed);
}

/// Summary of the predictor's caller-reported outcomes.
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Number of outcomes that callers reported as successful.
    pub faults_prevented: u64,
    /// Caller-reported percentage; zero until real evaluation logic is added.
    pub prefetch_accuracy: u8,
    /// Estimated working-set pages, if a caller calculates it.
    pub avg_working_set_pages: usize,
}

impl core::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Experimental Memory Predictor Statistics:")?;
        writeln!(f, "  Caller-reported successful outcomes: {}", self.faults_prevented)?;
        writeln!(f, "  Caller-reported accuracy: {}%", self.prefetch_accuracy)?;
        writeln!(f, "  Estimated working set: {} pages", self.avg_working_set_pages)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pattern_is_bounded() {
        let pattern = MemoryPattern::new();
        let result = forecast_memory_pressure(core::slice::from_ref(&pattern));
        assert!((0.0..=1.0).contains(&result));
        assert!(score_page_for_eviction(&pattern) <= 1.0);
    }

    #[test]
    fn uninitialized_predictor_does_not_recommend_prefetch() {
        PREDICTOR_INITIALIZED.store(false, Ordering::Release);
        let pattern = MemoryPattern {
            sequential_access: 1.0,
            stride: 64,
            spatial_locality: 1.0,
            access_frequency: 10.0,
            ..MemoryPattern::default()
        };
        assert!(!predict_page_fault(0x1000, &pattern).should_prefetch);
    }
}
