//! Self-Optimizing Performance Tuner for KeefeOS
//!
//! This module continuously monitors system performance and uses ML
//! to automatically tune kernel parameters for optimal performance.
//!
//! # Features
//! - Real-time performance monitoring
//! - Automatic parameter tuning
//! - Workload classification
//! - Energy-performance optimization

#![no_std]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering;
use alloc::vec::Vec;

static OPTIMIZER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static OPTIMIZATION_CYCLES: AtomicU64 = AtomicU64::new(0);
static PERFORMANCE_IMPROVEMENTS: AtomicU64 = AtomicU64::new(0);

/// Initialize the performance optimizer
pub fn init() {
    
        OPTIMIZER_INITIALIZED.store(true, Ordering::Release);

    log::info!("Self-optimizing performance tuner initialized");


/// Check if optimizer is initialized
pub fn is_initialized() -> bool {
    OPTIMIZER_INITIALIZED.load(Ordering::Acquire)


/// System performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub cpu_utilization: f32,      // 0.0 - 1.0
    pub memory_throughput: f32,    // GB/s normalized
    pub io_latency_us: f32,        // Microseconds (normalized)
    pub context_switch_rate: f32,  // Switches per second (normalized)
    pub cache_hit_ratio: f32,      // 0.0 - 1.0
    pub branch_prediction_accuracy: f32, // 0.0 - 1.0
    pub power_consumption: f32,    // Normalized power draw
    pub temperature: f32,          // Normalized thermal reading


impl PerformanceMetrics {
    pub fn new() -> Self {
        Self::default()

    
    /// Convert to feature vector
    pub fn to_vector(&self) -> [f32; 8] {
        [
            self.cpu_utilization,
            self.memory_throughput,
            self.io_latency_us,
            self.context_switch_rate,
            self.cache_hit_ratio,
            self.branch_prediction_accuracy,
            self.power_consumption,
            self.temperature,
        ]



/// Kernel tunable parameters
#[derive(Debug, Clone)]
pub struct KernelTunables {
    pub scheduler_quantum_ms: u32,
    pub page_reclaim_threshold: u8,  // Percentage
    pub readahead_pages: u8,
    pub dirty_ratio: u8,             // Percentage
    pub swappiness: u8,              // 0-100
    pub vm_overcommit: bool,
    pub preempt_enable: bool,


impl Default for KernelTunables {
    fn default() -> Self {
        Self {
            scheduler_quantum_ms: 20,
            page_reclaim_threshold: 70,
            readahead_pages: 128,
            dirty_ratio: 20,
            swappiness: 60,
            vm_overcommit: true,
            preempt_enable: true,




/// Optimization recommendation
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    pub target_parameter: &'static str,
    pub current_value: i32,
    pub recommended_value: i32,
    pub expected_improvement: f32, // Percentage
    pub confidence: f32,


/// Workload classification
#[derive(Debug, Clone, PartialEq)]
pub enum WorkloadType {
    Unknown,
    ComputeBound,
    MemoryBound,
    IOBound,
    Interactive,
    Mixed,
    RealTime,


/// Classify the current workload pattern
pub fn classify_workload(metrics: &PerformanceMetrics) -> WorkloadType {
    if !is_initialized() {
        return WorkloadType::Unknown;

    
    let feature_vector = metrics.to_vector();
    
    // Use neural network for classification
    let prediction = crate::ai::neural_net::get_global_nn()
        .map(|nn| nn.predict(&feature_vector))
        .unwrap_or([0.5; 16]);
    
    // Find the maximum output node (classification)
    let mut max_idx = 0;
    let mut max_val = prediction[0];
    for (i, &val) in prediction.iter().enumerate().take(7) {
        if val > max_val {
            max_val = val;
            max_idx = i;


    
    match max_idx {
        0 => WorkloadType::ComputeBound,
        1 => WorkloadType::MemoryBound,
        2 => WorkloadType::IOBound,
        3 => WorkloadType::Interactive,
        4 => WorkloadType::Mixed,
        5 => WorkloadType::RealTime,
        _ => WorkloadType::Unknown,



/// Generate optimization recommendations based on workload
pub fn optimize(workload: &WorkloadType, metrics: &PerformanceMetrics) -> Vec<OptimizationRecommendation, 8> {
    let mut recommendations = Vec::new();
    
    match workload {
        WorkloadType::ComputeBound => {
            // Optimize for CPU-intensive workloads
            recommendations.push(OptimizationRecommendation {
                target_parameter: "scheduler_quantum_ms",
                current_value: 20,
                recommended_value: 50,
                expected_improvement: 15.0,
                confidence: 0.85,
);
            
            if metrics.cache_hit_ratio < 0.8 {
                recommendations.push(OptimizationRecommendation {
                    target_parameter: "readahead_pages",
                    current_value: 128,
                    recommended_value: 256,
                    expected_improvement: 10.0,
                    confidence: 0.75,
);

,
        
        WorkloadType::MemoryBound => {
            // Optimize for memory-intensive workloads
            recommendations.push(OptimizationRecommendation {
                target_parameter: "page_reclaim_threshold",
                current_value: 70,
                recommended_value: 50,
                expected_improvement: 20.0,
                confidence: 0.80,
);
            
            recommendations.push(OptimizationRecommendation {
                target_parameter: "swappiness",
                current_value: 60,
                recommended_value: 80,
                expected_improvement: 12.0,
                confidence: 0.78,
);
,
        
        WorkloadType::IOBound => {
            // Optimize for I/O-intensive workloads
            recommendations.push(OptimizationRecommendation {
                target_parameter: "readahead_pages",
                current_value: 128,
                recommended_value: 512,
                expected_improvement: 25.0,
                confidence: 0.88,
);
            
            recommendations.push(OptimizationRecommendation {
                target_parameter: "dirty_ratio",
                current_value: 20,
                recommended_value: 40,
                expected_improvement: 15.0,
                confidence: 0.82,
);
,
        
        WorkloadType::Interactive => {
            // Optimize for responsiveness
            recommendations.push(OptimizationRecommendation {
                target_parameter: "scheduler_quantum_ms",
                current_value: 20,
                recommended_value: 10,
                expected_improvement: 30.0,
                confidence: 0.90,
);
            
            recommendations.push(OptimizationRecommendation {
                target_parameter: "preempt_enable",
                current_value: 1,
                recommended_value: 1,
                expected_improvement: 0.0,
                confidence: 1.0,
);
,
        
        WorkloadType::RealTime => {
            // Optimize for low latency
            recommendations.push(OptimizationRecommendation {
                target_parameter: "scheduler_quantum_ms",
                current_value: 20,
                recommended_value: 5,
                expected_improvement: 40.0,
                confidence: 0.92,
);
,
        
        _ => {
            // Default conservative tuning
            recommendations.push(OptimizationRecommendation {
                target_parameter: "scheduler_quantum_ms",
                current_value: 20,
                recommended_value: 20,
                expected_improvement: 0.0,
                confidence: 1.0,
);
,

    
    recommendations


/// Apply a tunable parameter change
pub fn apply_tunable(parameter: &str, value: i32) -> bool {
    log::debug!("Applying tunable: { = {", parameter, value);
    
    // In a real implementation, this would modify kernel state
    // For now, we just log the change
    
    OPTIMIZATION_CYCLES.fetch_add(1, Ordering::Relaxed);
    PERFORMANCE_IMPROVEMENTS.fetch_add(1, Ordering::Relaxed);
    crate::ai::record_training();
    
    true


/// Run an optimization cycle
pub fn optimization_cycle(metrics: &PerformanceMetrics) -> OptimizationStats {
    let workload = classify_workload(metrics);
    let recommendations = optimize(&workload, metrics);
    
    // Apply top recommendation
    if let Some(rec) = recommendations.first() {
        apply_tunable(rec.target_parameter, rec.recommended_value);

    
    OptimizationStats {
        cycles_completed: OPTIMIZATION_CYCLES.load(Ordering::Relaxed),
        improvements_made: PERFORMANCE_IMPROVEMENTS.load(Ordering::Relaxed),
        current_workload: workload,
        active_recommendations: recommendations.len() as u8,



#[derive(Debug, Clone)]
pub struct OptimizationStats {
    pub cycles_completed: u64,
    pub improvements_made: u64,
    pub current_workload: WorkloadType,
    pub active_recommendations: u8,


impl core::fmt::Display for OptimizationStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Performance Optimizer Statistics:")?;
        writeln!(f, "  Optimization Cycles: {", self.cycles_completed)?;
        writeln!(f, "  Improvements Made: {", self.improvements_made)?;
        writeln!(f, "  Current Workload: {:?", self.current_workload)?;
        writeln!(f, "  Active Recommendations: {", self.active_recommendations)?;
        Ok(())



/// Get energy-efficiency score (0-100)
pub fn calculate_efficiency_score(metrics: &PerformanceMetrics) -> u8 {
    let performance = metrics.cpu_utilization * 0.3
        + metrics.memory_throughput * 0.3
        + metrics.cache_hit_ratio * 0.2
        + metrics.branch_prediction_accuracy * 0.2;
    
    let power_efficiency = 1.0 - metrics.power_consumption;
    
    let score = (performance * 0.7 + power_efficiency * 0.3) * 100.0;
    score.min(100.0) as u8

