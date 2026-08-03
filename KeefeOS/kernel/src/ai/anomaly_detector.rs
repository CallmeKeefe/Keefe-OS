//! Anomaly Detection System for KeefeOS Security
//!
//! This module implements ML-based anomaly detection to identify:
//! - Intrusion attempts
//! - Malicious behavior patterns
//! - Resource abuse
//! - Privilege escalation attempts
//!
//! # Features
//! - Real-time syscall pattern analysis
//! - Behavioral profiling
//! - Zero-day attack detection
//! - Automated threat response

#![no_std]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use alloc::vec::Vec;

static DETECTOR_INITIALIZED: AtomicBool = AtomicBool::new(false);
static ANOMALIES_DETECTED: AtomicU64 = AtomicU64::new(0);
static FALSE_POSITIVES: AtomicU64 = AtomicU64::new(0);

/// Initialize the anomaly detector
pub fn init() {
    DETECTOR_INITIALIZED.store(true, Ordering::Release);
    log::info!("AI-powered anomaly detector initialized");
}

/// Check if detector is initialized
pub fn is_initialized() -> bool {
    DETECTOR_INITIALIZED.load(Ordering::Acquire)
}

/// Security-relevant system metrics
#[derive(Debug, Clone, Default)]
pub struct SecurityMetrics {
    pub syscall_rate: f32,           // Normalized syscalls per second
    pub privilege_level_changes: f32,// Frequency of ring transitions
    pub memory_allocation_rate: f32, // Pages allocated per second
    pub network_activity: f32,       // Normalized network I/O
    pub file_access_pattern: f32,    // 0.0 = normal, 1.0 = suspicious
    pub cpu_anomaly_score: f32,      // Unusual CPU usage patterns
    pub timing_anomaly: f32,         // Timing-based side channel indicators
}

impl SecurityMetrics {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Convert to feature vector for ML model
    pub fn to_vector(&self) -> [f32; 7] {
        [
            self.syscall_rate,
            self.privilege_level_changes,
            self.memory_allocation_rate,
            self.network_activity,
            self.file_access_pattern,
            self.cpu_anomaly_score,
            self.timing_anomaly,
        ]
    }
}

/// Threat classification result
#[derive(Debug, Clone)]
pub struct ThreatAssessment {
    pub is_anomalous: bool,
    pub threat_level: u8,        // 0-10 (10 = critical)
    pub threat_category: ThreatCategory,
    pub confidence: f32,
    pub recommended_action: Action,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThreatCategory {
    None,
    Suspicious,
    IntrusionAttempt,
    MalwareBehavior,
    PrivilegeEscalation,
    ResourceAbuse,
    SideChannelAttack,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    None,
    Log,
    Alert,
    Throttle,
    Isolate,
    Terminate,
}

impl Default for ThreatAssessment {
    fn default() -> Self {
        Self {
            is_anomalous: false,
            threat_level: 0,
            threat_category: ThreatCategory::None,
            confidence: 0.0,
            recommended_action: Action::None,
        }
    }
}

/// Analyze security metrics and detect anomalies
pub fn analyze(metrics: &SecurityMetrics) -> ThreatAssessment {
    if !is_initialized() {
        return ThreatAssessment::default();
    }
    
    let feature_vector = metrics.to_vector();
    
    // Use neural network for anomaly detection
    let prediction = crate::ai::neural_net::get_global_nn()
        .map(|nn| nn.predict(&feature_vector))
        .unwrap_or([0.5; 16]);
    
    let anomaly_score = prediction[2]; // Third output node = anomaly score
    
    let mut assessment = ThreatAssessment {
        is_anomalous: anomaly_score > 0.7,
        threat_level: (anomaly_score * 10.0) as u8,
        threat_category: ThreatCategory::None,
        confidence: anomaly_score,
        recommended_action: Action::None,
    };
    
    // Classify threat type based on which features are anomalous
    if assessment.is_anomalous {
        ANOMALIES_DETECTED.fetch_add(1, Ordering::Relaxed);
        
        assessment.threat_category = classify_threat(metrics, &prediction);
        assessment.recommended_action = determine_action(assessment.threat_level, &assessment.threat_category);
    }
    
    assessment
}

/// Classify the type of threat based on metrics and NN output
fn classify_threat(metrics: &SecurityMetrics, prediction: &[f32]) -> ThreatCategory {
    // Check for specific threat signatures
    if metrics.privilege_level_changes > 0.9 {
        return ThreatCategory::PrivilegeEscalation;
    }
    
    if metrics.syscall_rate > 0.95 && metrics.memory_allocation_rate > 0.8 {
        return ThreatCategory::ResourceAbuse;
    }
    
    if metrics.timing_anomaly > 0.8 {
        return ThreatCategory::SideChannelAttack;
    }
    
    if metrics.file_access_pattern > 0.85 {
        return ThreatCategory::IntrusionAttempt;
    }
    
    // Use NN classification outputs
    if prediction.len() > 3 && prediction[3] > 0.7 {
        return ThreatCategory::MalwareBehavior;
    }
    
    ThreatCategory::Suspicious
}

/// Determine appropriate response action
fn determine_action(threat_level: u8, category: &ThreatCategory) -> Action {
    match (threat_level, category) {
        (0..=3, _) => Action::Log,
        (4..=5, ThreatCategory::Suspicious) => Action::Alert,
        (4..=6, _) => Action::Alert,
        (7..=8, ThreatCategory::ResourceAbuse) => Action::Throttle,
        (7..=9, _) => Action::Isolate,
        (9..=10, _) => Action::Terminate,
        _ => Action::Log,
    }
}

/// Build a behavioral profile for a process
pub fn build_profile(syscall_history: &[u16]) -> ProcessProfile {
    let mut profile = ProcessProfile::default();
    
    if syscall_history.is_empty() {
        return profile;
    }
    
    // Calculate syscall distribution
    for &syscall in syscall_history {
        profile.syscall_counts[(syscall % 256) as usize] += 1;
    }
    
    profile.total_syscalls = syscall_history.len() as u64;
    profile.normalize();
    
    profile
}

/// Process behavioral profile
#[derive(Debug, Clone)]
pub struct ProcessProfile {
    syscall_counts: [u64; 256],
    total_syscalls: u64,
    entropy: f32,
}

impl Default for ProcessProfile {
    fn default() -> Self {
        Self {
            syscall_counts: [0; 256],
            total_syscalls: 0,
            entropy: 0.0,
        }
    }
}

impl ProcessProfile {
    fn normalize(&mut self) {
        if self.total_syscalls == 0 {
            return;
        }
        
        // Calculate entropy of syscall distribution
        let mut entropy = 0.0;
        for &count in &self.syscall_counts {
            if count > 0 {
                let p = (count as f32) / (self.total_syscalls as f32);
                entropy -= p * p.ln();
            }
        }
        self.entropy = entropy / 8.0; // Normalize to 0-1 range
    }
    
    pub fn entropy(&self) -> f32 {
        self.entropy
    }
    
    /// Compare profile against known malicious patterns
    pub fn is_suspicious(&self) -> bool {
        // High entropy might indicate packing/obfuscation
        self.entropy > 0.9
    }
}

/// Get detection statistics
pub fn get_stats() -> DetectionStats {
    let anomalies = ANOMALIES_DETECTED.load(Ordering::Relaxed);
    let false_pos = FALSE_POSITIVES.load(Ordering::Relaxed);
    
    let accuracy = if anomalies > 0 {
        ((anomalies - false_pos) * 100) / anomalies
    } else {
        100
    };
    
    DetectionStats {
        anomalies_detected: anomalies,
        false_positives: false_pos,
        accuracy_percent: accuracy as u8,
    }
}

#[derive(Debug, Clone, Default)]
pub struct DetectionStats {
    pub anomalies_detected: u64,
    pub false_positives: u64,
    pub accuracy_percent: u8,
}

impl core::fmt::Display for DetectionStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Anomaly Detection Statistics:")?;
        writeln!(f, "  Anomalies Detected: {}", self.anomalies_detected)?;
        writeln!(f, "  False Positives: {}", self.false_positives)?;
        writeln!(f, "  Accuracy: {}%", self.accuracy_percent)?;
        Ok(())
    }
}

/// Report a false positive for model improvement
pub fn report_false_positive() {
    FALSE_POSITIVES.fetch_add(1, Ordering::Relaxed);
    crate::ai::record_training();
}
