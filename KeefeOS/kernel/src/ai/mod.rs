//! Experimental deterministic AI-related heuristics for KeefeOS.
//!
//! These modules are prototypes for host-testable heuristics. They are not a
//! trained or remotely hosted model, do not run a general-purpose agent, and
//! do not enforce scheduling, paging, or security policy. Do not use their
//! outputs as a security decision or claim that they predict live workloads.
//!
//! Calling init initializes these prototype interfaces only; it does not load
//! trained weights, connect telemetry, or activate kernel enforcement. General
//! LLM inference, provider/API access, MCP clients, and agent tools belong in a
//! future isolated user-space service after process and IPC boundaries exist.

use core::sync::atomic::{AtomicU64, Ordering};

pub mod scheduler;
pub mod memory_predictor;
pub mod anomaly_detector;
pub mod neural_net;
pub mod optimizer;

/// AI subsystem version
pub const AI_VERSION: &str = "0.1.0";

/// Global statistics for AI operations
static INFERENCE_COUNT: AtomicU64 = AtomicU64::new(0);
static TRAINING_ITERATIONS: AtomicU64 = AtomicU64::new(0);
static PREDICTION_ACCURACY: AtomicU64 = AtomicU64::new(0);

/// Initialize the experimental AI-related prototype interfaces.
///
/// This must not be interpreted as loading a trained model or activating
/// process scheduling, paging, telemetry, or security enforcement.
pub fn init() {
    log::warn!(
        "Initializing KeefeOS AI-related prototypes v{}; no trained model or LLM is loaded",
        AI_VERSION
    );

    neural_net::init();
    scheduler::init();
    memory_predictor::init();
    anomaly_detector::init();
    optimizer::init();

    log::warn!(
        "Prototype interfaces initialized only: heuristic scheduling, memory, anomaly, and tuning recommendations are not connected to live OS enforcement"
    );
}

/// Get global AI statistics
pub fn get_ai_stats() -> AIStats {
    AIStats {
        inference_count: INFERENCE_COUNT.load(Ordering::Relaxed),
        training_iterations: TRAINING_ITERATIONS.load(Ordering::Relaxed),
        prediction_accuracy: PREDICTION_ACCURACY.load(Ordering::Relaxed),
    }
}

/// AI subsystem statistics
#[derive(Debug, Clone)]
pub struct AIStats {
    pub inference_count: u64,
    pub training_iterations: u64,
    pub prediction_accuracy: u64,
}

impl core::fmt::Display for AIStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "AI/ML Statistics:")?;
        writeln!(f, "  Total Inferences: {}", self.inference_count)?;
        writeln!(f, "  Training Iterations: {}", self.training_iterations)?;
        writeln!(f, "  Prediction Accuracy: {}%", self.prediction_accuracy)?;
        Ok(())
    }
}

/// Increment inference counter
#[inline(always)]
pub fn record_inference() {
    INFERENCE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Record training iteration
#[inline(always)]
pub fn record_training() {
    TRAINING_ITERATIONS.fetch_add(1, Ordering::Relaxed);
}

/// Update prediction accuracy metric (0-100)
#[inline(always)]
pub fn update_accuracy(accuracy: u64) {
    PREDICTION_ACCURACY.store(accuracy.min(100), Ordering::Relaxed);
}

/// Feature vector for ML models (fixed-size for no_std)
#[derive(Debug, Clone)]
pub struct FeatureVector {
    data: [f32; 16],
    len: usize,
}

impl FeatureVector {
    pub fn new() -> Self {
        Self {
            data: [0.0; 16],
            len: 0,
        }
    }
    
    pub fn push(&mut self, value: f32) -> Result<(), &'static str> {
        if self.len >= 16 {
            return Err("Feature vector full");
        }
        self.data[self.len] = value;
        self.len += 1;
        Ok(())
    }
    
    pub fn as_slice(&self) -> &[f32] {
        &self.data[..self.len]
    }
    
    pub fn len(&self) -> usize {
        self.len
    }
    
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for FeatureVector {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a feature vector to [0, 1] range
pub fn normalize(vector: &mut [f32]) {
    if vector.is_empty() {
        return;
    }
    
    let mut min = vector[0];
    let mut max = vector[0];
    
    for &val in vector.iter() {
        if val < min {
            min = val;
        }
        if val > max {
            max = val;
        }
    }
    
    let range = max - min;
    if range > 0.0 {
        for val in vector.iter_mut() {
            *val = (*val - min) / range;
        }
    }
}

/// Sigmoid activation function for neural networks
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    // Rational approximation available in core/no_std; not a libm exponential.
    0.5 * (x / (1.0 + x.abs()) + 1.0)
}

/// ReLU activation function
#[inline]
pub fn relu(x: f32) -> f32 {
    if x > 0.0 { x } else { 0.0 }
}

/// Softmax function for output layer
pub fn softmax(values: &mut [f32]) {
    if values.is_empty() {
        return;
    }
    // Rational positive weights approximate softmax without exp() in core/no_std.
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for value in values.iter_mut() {
        let delta = (*value - max).clamp(-32.0, 0.0);
        *value = 1.0 / (1.0 + delta.abs());
        sum += *value;
    }
    if sum > 0.0 {
        for value in values.iter_mut() {
            *value /= sum;
        }
    }
}
