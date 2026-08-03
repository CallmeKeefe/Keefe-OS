//! ML-Based Process Scheduler for KeefeOS
//!
//! This module implements an intelligent process scheduler that uses machine learning
//! to predict process behavior and optimize CPU allocation.
//!
//! # Features
//! - Predictive scheduling based on historical process behavior
//! - Dynamic time slice adjustment
//! - Priority boosting for interactive processes
//! - Energy-aware scheduling

#![no_std]

use core::sync::atomic::{AtomicU32, AtomicBool, Ordering;
use alloc::vec::Vec;

static SCHEDULER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static PREDICTION_MODE: AtomicU32 = AtomicU32::new(0);

/// Initialize the ML scheduler
pub fn init() {
    
        SCHEDULER_INITIALIZED.store(true, Ordering::Release);

    log::info!("ML-based scheduler initialized");


/// Check if scheduler is initialized
pub fn is_initialized() -> bool {
    SCHEDULER_INITIALIZED.load(Ordering::Acquire)


/// Process characteristics for ML prediction
#[derive(Debug, Clone, Default)]
pub struct ProcessFeatures {
    pub cpu_usage: f32,           // 0.0 - 1.0
    pub memory_pressure: f32,     // 0.0 - 1.0
    pub io_wait_ratio: f32,       // 0.0 - 1.0
    pub nice_value: f32,          // -20 to 19 normalized
    pub interactivity_score: f32, // 0.0 - 1.0 (higher = more interactive)
    pub cache_miss_rate: f32,     // 0.0 - 1.0
    pub branch_misprediction: f32,// 0.0 - 1.0
    pub context_switches: f32,    // Normalized count


impl ProcessFeatures {
    pub fn new() -> Self {
        Self::default()

    
    /// Convert to feature vector for neural network
    pub fn to_vector(&self) -> [f32; 8] {
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



/// Scheduling decision from ML model
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    pub priority: u8,              // 0-9 (0 = highest)
    pub time_slice_ms: u32,        // Recommended time slice
    pub should_preempt: bool,      // Whether to preempt current process
    pub confidence: f32,           // Model confidence (0.0 - 1.0)


impl Default for SchedulingDecision {
    fn default() -> Self {
        Self {
            priority: 5,
            time_slice_ms: 20,
            should_preempt: false,
            confidence: 0.5,




/// Global scheduler state
struct SchedulerState {
    total_decisions: u64,
    accurate_predictions: u64,
    avg_time_slice: u32,


static mut SCHED_STATE: Option<SchedulerState> = None;

/// Make a scheduling decision based on process features
pub fn schedule(features: &ProcessFeatures) -> SchedulingDecision {
    if !is_initialized() {
        return SchedulingDecision::default();

    
    let feature_vector = features.to_vector();
    
    // Use neural network for prediction (from neural_net module)
    let priority = crate::ai::neural_net::predict_scheduling_priority(&feature_vector);
    
    // Calculate optimal time slice based on characteristics
    let time_slice = calculate_time_slice(features);
    
    // Determine if preemption is beneficial
    let should_preempt = features.interactivity_score > 0.8 && priority < 3;
    
    // Update statistics
    
        if let Some(ref mut state) = SCHED_STATE {
            state.total_decisions += 1;
            state.avg_time_slice = (state.avg_time_slice + time_slice) / 2;


    
    SchedulingDecision {
        priority,
        time_slice_ms: time_slice,
        should_preempt,
        confidence: 0.85, // Placeholder - would come from model



/// Calculate optimal time slice based on process characteristics
fn calculate_time_slice(features: &ProcessFeatures) -> u32 {
    // Base time slice
    let mut time_slice = 20u32;
    
    // CPU-bound processes get longer time slices
    if features.cpu_usage > 0.7 {
        time_slice += 10;

    
    // I/O-bound processes get shorter time slices for better responsiveness
    if features.io_wait_ratio > 0.5 {
        time_slice -= 5;

    
    // Interactive processes get priority treatment
    if features.interactivity_score > 0.8 {
        time_slice = time_slice.saturating_sub(5);

    
    // High cache miss rate suggests memory-bound, adjust accordingly
    if features.cache_miss_rate > 0.6 {
        time_slice += 5;

    
    time_slice.max(5).min(100) // Clamp between 5ms and 100ms


/// Train the scheduler model with feedback
pub fn train(decision: &SchedulingDecision, outcome_good: bool) {
    
        if let Some(ref mut state) = SCHED_STATE {
            if outcome_good {
                state.accurate_predictions += 1;



    
    crate::ai::record_training();


/// Get scheduler statistics
pub fn get_stats() -> SchedulerStats {
    
        if let Some(ref state) = SCHED_STATE {
            let accuracy = if state.total_decisions > 0 {
                (state.accurate_predictions * 100) / state.total_decisions
 else {
                0
;
            
            SchedulerStats {
                total_decisions: state.total_decisions,
                accurate_predictions: state.accurate_predictions,
                accuracy_percent: accuracy as u8,
                avg_time_slice: state.avg_time_slice,

 else {
            SchedulerStats::default()




#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_decisions: u64,
    pub accurate_predictions: u64,
    pub accuracy_percent: u8,
    pub avg_time_slice: u32,


impl core::fmt::Display for SchedulerStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "ML Scheduler Statistics:")?;
        writeln!(f, "  Total Decisions: {", self.total_decisions)?;
        writeln!(f, "  Accurate Predictions: {", self.accurate_predictions)?;
        writeln!(f, "  Accuracy: {%", self.accuracy_percent)?;
        writeln!(f, "  Avg Time Slice: {ms", self.avg_time_slice)?;
        Ok(())



/// Set the prediction mode (0 = conservative, 1 = balanced, 2 = aggressive)
pub fn set_prediction_mode(mode: u8) {
    PREDICTION_MODE.store(mode.min(2) as u32, Ordering::Relaxed);
    log::debug!("Scheduler prediction mode set to {", mode);


/// Get current prediction mode
pub fn get_prediction_mode() -> u8 {
    PREDICTION_MODE.load(Ordering::Relaxed) as u8

