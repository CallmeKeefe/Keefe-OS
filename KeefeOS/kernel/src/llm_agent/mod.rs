//! Local command-demo interface for KeefeOS.
//!
//! This module is not connected to an LLM, network provider, or privileged
//! system-management service. It handles a few recognized strings locally so
//! the kernel can expose a stable prototype API without claiming AI inference.


extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static AGENT_INITIALIZED: AtomicBool = AtomicBool::new(false);
static COMMANDS_PROCESSED: AtomicU64 = AtomicU64::new(0);

/// Broad command category used by the local demo parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    /// Request status information.
    SystemQuery,
    /// Request a security audit.
    SecurityAudit,
    /// Request a tuning recommendation.
    PerformanceTuning,
    /// Request a file operation (unsupported).
    FileOperation,
    /// Request process management (unsupported).
    ProcessManagement,
    /// Request network configuration (unsupported).
    NetworkConfig,
    /// General help or educational assistance.
    UserAssistance,
    /// Request an automation task (unsupported).
    Automation,
    /// Request diagnostic information.
    Diagnostic,
    /// An unrecognized command.
    Unknown,
}

/// Result from the local command demo.
#[derive(Debug, Clone)]
pub struct AgentResponse {
    /// Whether this local command was recognized and answered.
    pub success: bool,
    /// Human-readable response.
    pub message: String,
    /// Optional raw response bytes; empty for this demo.
    pub data: Vec<u8>,
    /// Suggested next command strings.
    pub suggested_actions: Vec<String>,
    /// Heuristic confidence, not an LLM confidence score.
    pub confidence: f32,
}

impl AgentResponse {
    /// Create a response with no data or suggested actions.
    pub fn new(success: bool, message: &str) -> Self {
        Self {
            success,
            message: String::from(message),
            data: Vec::new(),
            suggested_actions: Vec::new(),
            confidence: 0.0,
        }
    }

    /// Set a bounded local-heuristic confidence value.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Append a suggested command.
    pub fn suggest(mut self, command: &str) -> Self {
        self.suggested_actions.push(String::from(command));
        self
    }
}

/// Initialize the local command-demo interface.
pub fn init() {
    AGENT_INITIALIZED.store(true, Ordering::Release);
    log::warn!("KeefeOS command demo initialized; no external LLM is connected");
}

/// Return whether the local command-demo interface was initialized.
pub fn is_initialized() -> bool {
    AGENT_INITIALIZED.load(Ordering::Acquire)
}

/// Return the number of commands handled by the local demo.
pub fn commands_processed() -> u64 {
    COMMANDS_PROCESSED.load(Ordering::Relaxed)
}

/// Categorize a command using simple text matching.
pub fn classify_command(command: &str) -> CommandType {
    let command = command.trim();
    if command.eq_ignore_ascii_case("show system status")
        || command.eq_ignore_ascii_case("system status")
        || command.eq_ignore_ascii_case("status")
    {
        CommandType::SystemQuery
    } else if command.eq_ignore_ascii_case("run security audit")
        || command.eq_ignore_ascii_case("security audit")
    {
        CommandType::SecurityAudit
    } else if command.eq_ignore_ascii_case("optimize performance")
        || command.eq_ignore_ascii_case("performance tuning")
    {
        CommandType::PerformanceTuning
    } else if command.eq_ignore_ascii_case("help") {
        CommandType::UserAssistance
    } else {
        CommandType::Unknown
    }
}

/// Process a small set of local demo commands.
///
/// A response never implies that the OS booted successfully, that a scanner
/// inspected the machine, or that an LLM generated the answer.
pub fn process_command(command: &str) -> AgentResponse {
    if !is_initialized() {
        return AgentResponse::new(false, "Local command demo is not initialized");
    }

    COMMANDS_PROCESSED.fetch_add(1, Ordering::Relaxed);

    match classify_command(command) {
        CommandType::SystemQuery => AgentResponse::new(
            true,
            "KeefeOS status: experimental x86-64 kernel prototype. Early boot has passed the configured QEMU/OVMF RAM matrix; this does not establish a complete OS. Paging safety, process isolation, general drivers, and runtime security monitoring are incomplete.",
        )
        .with_confidence(1.0)
        .suggest("show build status")
        .suggest("show known limitations"),
        CommandType::SecurityAudit => AgentResponse::new(
            false,
            "No security audit was performed: vulnerability scanning and attack detection are placeholders, and secure-boot verification is not implemented.",
        )
        .with_confidence(1.0)
        .suggest("review the security audit documentation"),
        CommandType::PerformanceTuning => AgentResponse::new(
            false,
            "Performance recommendations are experimental heuristics only; no live kernel tunable interface or benchmark feedback is connected.",
        )
        .with_confidence(1.0)
        .suggest("review performance benchmark requirements"),
        CommandType::UserAssistance => AgentResponse::new(
            true,
            "Local command demo only; no external LLM is connected. Try 'show system status' or 'run security audit'.",
        )
        .with_confidence(1.0),
        _ => AgentResponse::new(
            false,
            "Command not supported by the local demo. No file, process, network, or automation operation was executed.",
        )
        .with_confidence(1.0)
        .suggest("show system status")
        .suggest("help"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_status_command() {
        assert_eq!(classify_command("show system status"), CommandType::SystemQuery);
    }

    #[test]
    fn status_command_reports_verified_scope_without_claiming_full_os_support() {
        AGENT_INITIALIZED.store(true, Ordering::Release);
        let response = process_command("status");
        assert!(response.message.contains("QEMU/OVMF"));
        assert!(response.message.contains("does not establish a complete OS"));
        assert!(response.message.contains("process isolation"));
    }

    #[test]
    fn unknown_commands_are_not_executed() {
        AGENT_INITIALIZED.store(true, Ordering::Release);
        let response = process_command("delete user files");
        assert!(!response.success);
        assert!(response.message.contains("No file"));
    }
}
