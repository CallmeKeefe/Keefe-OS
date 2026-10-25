//! Fail-closed capability API for KeefeOS.
//!
//! No trusted issuer key, cryptographic signature verifier, trusted clock, or
//! process-isolation enforcement is integrated. A non-zero signature byte is
//! not a signature. Until a reviewed verifier exists, capabilities cannot be
//! issued, transferred, or authorized.

#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// A proposed capability record. It is not an authorization grant unless a
/// future trusted verifier accepts it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    pub resource: String,
    pub operation: String,
    pub issuer: u64,
    pub holder: u64,
    pub expiration: u64,
    pub signature: [u8; 32],
}

impl Capability {
    /// Construct an unsigned capability request, not an authorization grant.
    pub fn new(resource: &str, operation: &str) -> Self {
        Self {
            resource: String::from(resource),
            operation: String::from(operation),
            issuer: 0,
            holder: 0,
            expiration: 0,
            signature: [0; 32],
        }
    }

    /// Refuse to sign until a real issuer-key and signature scheme exists.
    pub fn sign(&mut self, _issuer_id: u64, _signature: [u8; 32]) -> Result<(), &'static str> {
        Err("capability signing is not implemented; request remains unsigned")
    }

    /// Always fails closed until a cryptographic verifier and trusted keys exist.
    pub const fn verify(&self) -> bool {
        false
    }

    /// Check expiry against a caller-supplied trusted time value.
    pub fn is_expired(&self, current_time: u64) -> bool {
        current_time >= self.expiration
    }

    /// Refuse transfer until the original grant can be verified and re-issued.
    pub fn transfer(&mut self, _new_holder: u64) -> Result<(), &'static str> {
        Err("capability transfer is not implemented; no trusted issuer is available")
    }
}

/// Manager for capability request records. It never authorizes unverified grants.
pub struct CapabilityManager {
    capabilities: Vec<Capability>,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self { capabilities: Vec::new() }
    }

    /// Add a capability only if a trusted verifier accepts it.
    pub fn add_capability(&mut self, cap: Capability) -> Result<(), &'static str> {
        if !cap.verify() {
            return Err("capability rejected: signature verification is not implemented");
        }
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
        Ok(())
    }

    /// Check for an exact, verified, unexpired grant.
    ///
    /// No grant can currently pass verify(); this returns false until a real
    /// verifier is integrated.
    pub fn has_access(&self, holder_id: u64, resource: &str, operation: &str) -> bool {
        self.capabilities.iter().any(|cap| {
            cap.holder == holder_id
                && cap.resource == resource
                && cap.operation == operation
                && cap.verify()
                && !cap.is_expired(current_time())
        })
    }

    /// Remove matching local records. This is not a persistent revocation system.
    pub fn revoke(&mut self, holder_id: u64, resource: &str, operation: &str) {
        self.capabilities.retain(|cap| {
            !(cap.holder == holder_id && cap.resource == resource && cap.operation == operation)
        });
    }

    /// List records for diagnostics. Listing a record does not imply validity.
    pub fn list_capabilities(&self, holder_id: u64) -> Vec<&Capability> {
        self.capabilities.iter().filter(|cap| cap.holder == holder_id).collect()
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize capability status reporting without claiming enforcement is active.
pub fn init() {
    log::warn!("Capability authorization is NOT IMPLEMENTED; unverified grants are denied");
}

fn current_time() -> u64 {
    // A monotonic diagnostic counter only, not wall-clock or trusted expiry time.
    static TIME: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    TIME.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}

/// Common capability request constructors. They create unsigned requests only.
pub mod caps {
    use alloc::format;
    use alloc::string::String;
    use super::Capability;

    pub fn memory_read(addr: usize) -> Capability {
        let mut cap = Capability::new("memory", "read");
        cap.resource = format!("memory:{:#x}", addr);
        cap
    }

    pub fn memory_write(addr: usize) -> Capability {
        let mut cap = Capability::new("memory", "write");
        cap.resource = format!("memory:{:#x}", addr);
        cap
    }

    pub fn device_access(device: &str) -> Capability {
        Capability::new("device", device)
    }

    pub fn ipc_send(target_pid: u64) -> Capability {
        let mut cap = Capability::new("ipc", "send");
        cap.resource = format!("process:{}", target_pid);
        cap
    }

    pub fn file_access(path: &str, mode: &str) -> Capability {
        let mut cap = Capability::new("file", mode);
        cap.resource = String::from(path);
        cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonzero_signature_is_not_treated_as_verified() {
        let mut cap = Capability::new("memory:0x1000", "read");
        assert!(cap.sign(1, [0xA5; 32]).is_err());
        assert!(!cap.verify());
    }

    #[test]
    fn manager_rejects_unsigned_or_unverifiable_capabilities() {
        let mut manager = CapabilityManager::new();
        let cap = Capability::new("file:/secret", "read");
        assert!(manager.add_capability(cap).is_err());
        assert!(!manager.has_access(7, "file:/secret", "read"));
    }

    #[test]
    fn changing_holder_without_reissue_is_rejected() {
        let mut cap = Capability::new("device:console", "write");
        assert!(cap.transfer(42).is_err());
        assert_eq!(cap.holder, 0);
    }
}
