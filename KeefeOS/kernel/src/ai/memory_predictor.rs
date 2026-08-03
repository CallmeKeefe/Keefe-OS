//! Predictive Memory Management for KeefeOS
//!
//! This module uses machine learning to predict memory access patterns
//! and optimize page allocation, prefetching, and swapping decisions.
//!
//! # Features
//! - Page fault prediction and pre-loading
//! - Working set estimation
//! - Intelligent page replacement
//! - Memory pressure forecasting

#![no_std]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering;
use alloc::vec::Vec;

static PREDICTOR_INITIALIZED: AtomicBool = AtomicBool::new(false);
static PAGE_FAULTS_PREVENTED: AtomicU64 = AtomicU64::new(0);

/// Initialize the memory predictor
pub fn init() {
    
        PREDICTOR_INITIALIZED.store(true, Ordering::Release);

    log::info!("Predictive memory manager initialized");


/// Check if predictor is initialized
pub fn is_initialized() -> bool {
    PREDICTOR_INITIALIZED.load(Ordering::Acquire)


/// Memory access pattern features
#[derive(Debug, Clone, Default)]
pub struct MemoryPattern {
    pub sequential_access: f32,    // 0.0 - 1.0 (1.0 = purely sequential)
    pub stride: i32,               // Common access stride
    pub temporal_locality: f32,    // 0.0 - 1.0
    pub spatial_locality: f32,     // 0.0 - 1.0
    pub write_ratio: f32,          // 0.0 - 1.0
    pub access_frequency: f32,     // Normalized accesses per second
    pub page_age_ms: u64,          // Time since last access


impl MemoryPattern {
    pub fn new() -> Self {
        Self::default()

    
    /// Convert to feature vector
    pub fn to_vector(&self) -> [f32; 7] {
        [
            self.sequential_access,
            ((self.stride + 1000) as f32) / 2000.0, // Normalize stride
            self.temporal_locality,
            self.spatial_locality,
            self.write_ratio,
            self.access_frequency,
            ((self.page_age_ms % 10000) as f32) / 10000.0,
        ]



/// Prefetch recommendation
#[derive(Debug, Clone)]
pub struct PrefetchRecommendation {
    pub should_prefetch: bool,
    pub pages_to_prefetch: u8,
    pub confidence: f32,
    pub predicted_addresses: [u64; 4],


impl Default for PrefetchRecommendation {
    fn default() -> Self {
        Self {
            should_prefetch: false,
            pages_to_prefetch: 0,
            confidence: 0.0,
            predicted_addresses: [0; 4],




/// Predict if a page fault will occur and recommend prefetching
pub fn predict_page_fault(current_addr: u64, pattern: &MemoryPattern) -> PrefetchRecommendation {
    if !is_initialized() {
        return PrefetchRecommendation::default();

    
    let feature_vector = pattern.to_vector();
    
    // Use neural network to predict access pattern
    let prediction = crate::ai::neural_net::get_global_nn()
        .map(|nn| nn.predict(&feature_vector))
        .unwrap_or([0.5; 16]);
    
    let fault_probability = prediction[0];
    let should_prefetch = fault_probability > 0.6 && pattern.sequential_access > 0.5;
    
    let mut rec = PrefetchRecommendation {
        should_prefetch,
        pages_to_prefetch: if should_prefetch { 4 else { 0,
        confidence: fault_probability,
        predicted_addresses: [0; 4],
;
    
    // Predict next addresses based on stride
    if pattern.sequential_access > 0.7 {
        let stride = pattern.stride as u64;
        for i in 0..4 {
            rec.predicted_addresses[i] = current_addr + (stride * (i as u64 + 1));

        
        if should_prefetch {
            PAGE_FAULTS_PREVENTED.fetch_add(rec.pages_to_prefetch as u64, Ordering::Relaxed);


    
    rec


/// Decide which page to evict using ML-based scoring
pub fn score_page_for_eviction(pattern: &MemoryPattern) -> f32 {
    if !is_initialized() {
        // Fallback to simple LRU-like scoring
        return 1.0 - pattern.temporal_locality;

    
    let feature_vector = pattern.to_vector();
    
    // Higher score = better candidate for eviction
    let score = if let Some(nn) = crate::ai::neural_net::get_global_nn() {
        let output = nn.predict(&feature_vector);
        output[1] // Second output node represents eviction score
 else {
        0.5
;
    
    score


/// Estimate working set size based on access patterns
pub fn estimate_working_set(patterns: &[MemoryPattern]) -> usize {
    if patterns.is_empty() {
        return 0;

    
    let avg_locality: f32 = patterns.iter()
        .map(|p| (p.temporal_locality + p.spatial_locality) / 2.0)
        .sum::<f32>() / patterns.len() as f32;
    
    // Higher locality = smaller working set relative to total pages
    let working_set_factor = 1.0 - (avg_locality * 0.7);
    
    // Estimate: assume 1000 total pages, adjust by locality
    (1000.0 * working_set_factor) as usize


/// Forecast memory pressure (0.0 = low, 1.0 = critical)
pub fn forecast_memory_pressure(recent_patterns: &[MemoryPattern]) -> f32 {
    if recent_patterns.is_empty() {
        return 0.5;

    
    // Analyze trends in access frequency and page age
    let avg_frequency: f32 = recent_patterns.iter()
        .map(|p| p.access_frequency)
        .sum::<f32>() / recent_patterns.len() as f32;
    
    let avg_age: f32 = recent_patterns.iter()
        .map(|p| p.page_age_ms as f32)
        .sum::<f32>() / recent_patterns.len() as f32;
    
    // High frequency + low age = increasing pressure
    let pressure = (avg_frequency * 0.6) + ((1.0 - (avg_age / 10000.0)) * 0.4);
    pressure.min(1.0).max(0.0)


/// Get statistics about prevented page faults
pub fn get_prevented_faults() -> u64 {
    PAGE_FAULTS_PREVENTED.load(Ordering::Relaxed)


/// Record a successful prefetch prediction
pub fn record_successful_prefetch() {
    PAGE_FAULTS_PREVENTED.fetch_add(1, Ordering::Relaxed);
    crate::ai::record_training();


/// Memory predictor statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub faults_prevented: u64,
    pub prefetch_accuracy: u8,
    pub avg_working_set_pages: usize,


impl core::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Predictive Memory Statistics:")?;
        writeln!(f, "  Page Faults Prevented: {", self.faults_prevented)?;
        writeln!(f, "  Prefetch Accuracy: {%", self.prefetch_accuracy)?;
        writeln!(f, "  Avg Working Set: { pages", self.avg_working_set_pages)?;
        Ok(())


