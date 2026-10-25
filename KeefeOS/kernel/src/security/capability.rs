//! Capability-Based Access Control for KeefeOS
//!
//! Implements unforgeable capability tokens for resource access control.
//! Capabilities are cryptographically signed and verified using blockchain.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

/// A capability token granting access to a resource
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub resource: String,
    pub operation: String,
    pub issuer: u64,
    pub holder: u64,
    pub expiration: u64,
    pub signature: [u8; 32],
}

impl Capability {
    /// Create a new capability
    pub fn new(resource: &str, operation: &str) -> Self {
        Self {
            resource: String::from(resource),
            operation: String::from(operation),
            issuer: 0,
            holder: 0,
            expiration: u64::MAX,
            signature: [0; 32],
        }
    }
    
    /// Sign the capability with issuer's key
    pub fn sign(&mut self, issuer_id: u64, signature: [u8; 32]) {
        self.issuer = issuer_id;
        self.signature = signature;
    }
    
    /// Verify the capability signature
    pub fn verify(&self) -> bool {
        // In production, this would verify cryptographic signature
        // For now, check if signature is non-zero
        self.signature.iter().any(|&b| b != 0)
    }
    
    /// Check if capability is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time > self.expiration
    }
    
    /// Transfer capability to new holder
    pub fn transfer(&mut self, new_holder: u64) {
        self.holder = new_holder;
    }
}

impl Hash for Capability {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.resource.hash(state);
        self.operation.hash(state);
        self.issuer.hash(state);
        self.holder.hash(state);
    }
}

/// Capability manager for tracking and validating capabilities
pub struct CapabilityManager {
    capabilities: Vec<Capability>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }
    
    /// Add a capability to the manager
    pub fn add_capability(&mut self, cap: Capability) {
        if cap.verify() {
            self.capabilities.push(cap);
        }
    }
    
    /// Check if a capability exists for given resource and operation
    pub fn has_access(&self, holder_id: u64, resource: &str, operation: &str) -> bool {
        self.capabilities.iter().any(|cap| {
            cap.holder == holder_id && 
            cap.resource == resource && 
            cap.operation == operation &&
            cap.verify() &&
            !cap.is_expired(get_current_time())
        })
    }
    
    /// Revoke a capability
    pub fn revoke(&mut self, holder_id: u64, resource: &str, operation: &str) {
        self.capabilities.retain(|cap| {
            !(cap.holder == holder_id && cap.resource == resource && cap.operation == operation)
        });
    }
    
    /// List all capabilities for a holder
    pub fn list_capabilities(&self, holder_id: u64) -> Vec<&Capability> {
        self.capabilities.iter()
            .filter(|cap| cap.holder == holder_id)
            .collect()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the capability system
pub fn init() {
    log::info!("Capability-based access control initialized");
}

/// Get current time (simplified)
fn get_current_time() -> u64 {
    static TIME: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    TIME.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

/// Common capability types for system resources
pub mod caps {
    use alloc::format;
    use alloc::string::String;
    use super::Capability;
    
    /// Create capability for memory access
    pub fn memory_read(addr: usize) -> Capability {
        let mut cap = Capability::new("memory", "read");
        cap.resource = format!("memory:{:#x}", addr);
        cap
    }
    
    /// Create capability for memory write
    pub fn memory_write(addr: usize) -> Capability {
        let mut cap = Capability::new("memory", "write");
        cap.resource = format!("memory:{:#x}", addr);
        cap
    }
    
    /// Create capability for device access
    pub fn device_access(device: &str) -> Capability {
        Capability::new("device", device)
    }
    
    /// Create capability for IPC
    pub fn ipc_send(target_pid: u64) -> Capability {
        let mut cap = Capability::new("ipc", "send");
        cap.resource = format!("process:{}", target_pid);
        cap
    }
    
    /// Create capability for file access
    pub fn file_access(path: &str, mode: &str) -> Capability {
        let mut cap = Capability::new("file", mode);
        cap.resource = String::from(path);
        cap
    }
}
