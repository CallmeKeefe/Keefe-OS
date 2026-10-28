//! In-memory hash-chain audit-log prototype.
//!
//! This is not a blockchain and its small non-cryptographic digest is not a
//! security primitive. It is an in-memory development log only; it is not
//! persistent, signed, tamper-resistant, or independently verified.


extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

static BLOCKCHAIN_INITIALIZED: AtomicBool = AtomicBool::new(false);
static BLOCK_COUNT: AtomicU64 = AtomicU64::new(0);
static GLOBAL_BLOCKCHAIN: Mutex<Option<Blockchain>> = Mutex::new(None);
static TIMESTAMP: AtomicU64 = AtomicU64::new(0);

/// One record in the prototype hash chain.
#[derive(Debug, Clone)]
pub struct Block {
    /// Sequential record index.
    pub index: u64,
    /// Monotonic counter, not wall-clock time.
    pub timestamp: u64,
    /// Event kind encoded as a byte.
    pub event_type: u8,
    /// Severity encoded as a byte.
    pub severity: u8,
    /// Non-cryptographic digest of the description.
    pub description_hash: [u8; 32],
    /// Digest copied from the previous record.
    pub previous_hash: [u8; 32],
    /// Non-cryptographic digest of this record's fields.
    pub current_hash: [u8; 32],
    /// Reserved for future format compatibility; no proof-of-work is done.
    pub nonce: u64,
}

impl Block {
    /// Create an initialized genesis record.
    pub fn genesis() -> Self {
        let mut block = Self {
            index: 0,
            timestamp: 0,
            event_type: 0,
            severity: 0,
            description_hash: [0; 32],
            previous_hash: [0; 32],
            current_hash: [0; 32],
            nonce: 0,
        };
        block.current_hash = block.calculate_hash();
        block
    }

    /// Create a record linked to the previous digest.
    pub fn new(
        index: u64,
        event_type: u8,
        severity: u8,
        description: &str,
        previous_hash: [u8; 32],
    ) -> Self {
        let mut block = Self {
            index,
            timestamp: TIMESTAMP.fetch_add(1, Ordering::Relaxed),
            event_type,
            severity,
            description_hash: hash_data(description.as_bytes()),
            previous_hash,
            current_hash: [0; 32],
            nonce: 0,
        };
        block.current_hash = block.calculate_hash();
        block
    }

    /// Calculate a non-cryptographic digest for consistency checking.
    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut data = [0u8; 90];
        let mut cursor = 0usize;
        data[cursor..cursor + 8].copy_from_slice(&self.index.to_le_bytes());
        cursor += 8;
        data[cursor..cursor + 8].copy_from_slice(&self.timestamp.to_le_bytes());
        cursor += 8;
        data[cursor] = self.event_type;
        cursor += 1;
        data[cursor] = self.severity;
        cursor += 1;
        data[cursor..cursor + 32].copy_from_slice(&self.description_hash);
        cursor += 32;
        data[cursor..cursor + 32].copy_from_slice(&self.previous_hash);
        cursor += 32;
        data[cursor..cursor + 8].copy_from_slice(&self.nonce.to_le_bytes());
        hash_data(&data[..cursor + 8])
    }

    /// Return whether the stored digest matches the current fields.
    pub fn verify(&self) -> bool {
        self.current_hash == self.calculate_hash()
    }
}

/// In-memory list of audit records.
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    /// Create a chain with a genesis record.
    pub fn new() -> Self {
        Self {
            blocks: alloc::vec![Block::genesis()],
        }
    }

    /// Append an event to the chain.
    pub fn add_block(&mut self, event_type: u8, severity: u8, description: &str) {
        let index = self.blocks.len() as u64;
        let previous_hash = self.blocks.last().map(|block| block.current_hash).unwrap_or([0; 32]);
        self.blocks.push(Block::new(index, event_type, severity, description, previous_hash));
        BLOCK_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    /// Check stored digests and links. This is consistency checking, not proof
    /// against a malicious writer because the digest is not cryptographic.
    pub fn verify_chain(&self) -> bool {
        if self.blocks.is_empty() {
            return false;
        }
        for (index, block) in self.blocks.iter().enumerate() {
            if !block.verify() {
                return false;
            }
            if index > 0 && block.previous_hash != self.blocks[index - 1].current_hash {
                return false;
            }
        }
        true
    }

    /// Return the number of records including genesis.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Return true if the chain contains no records.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get a record by its index.
    pub fn get_block(&self, index: usize) -> Option<&Block> {
        self.blocks.get(index)
    }

    /// Copy the most recent records.
    pub fn get_recent_blocks(&self, count: usize) -> Vec<&Block> {
        let start = self.blocks.len().saturating_sub(count);
        self.blocks[start..].iter().collect()
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the in-memory audit-log prototype.
pub fn init() {
    *GLOBAL_BLOCKCHAIN.lock() = Some(Blockchain::new());
    BLOCK_COUNT.store(0, Ordering::Relaxed);
    BLOCKCHAIN_INITIALIZED.store(true, Ordering::Release);
    log::warn!("In-memory audit-chain prototype initialized; not a blockchain or cryptographic ledger");
}

/// Return whether the in-memory prototype was initialized.
pub fn is_initialized() -> bool {
    BLOCKCHAIN_INITIALIZED.load(Ordering::Acquire)
}

/// Record one security event if the prototype has been initialized.
pub fn record_event(
    event_type: super::SecurityEventType,
    severity: super::SecuritySeverity,
    description: &str,
) {
    if !is_initialized() {
        return;
    }
    if let Some(ref mut chain) = *GLOBAL_BLOCKCHAIN.lock() {
        chain.add_block(event_type as u8, severity as u8, description);
    }
}

/// Check the in-memory hash chain's internal consistency.
pub fn verify_integrity() -> bool {
    if !is_initialized() {
        return false;
    }
    GLOBAL_BLOCKCHAIN.lock().as_ref().map(Blockchain::verify_chain).unwrap_or(false)
}

/// Return the number of event records added and total records including genesis.
pub fn get_stats() -> (u64, usize) {
    let len = GLOBAL_BLOCKCHAIN.lock().as_ref().map(Blockchain::len).unwrap_or(0);
    (BLOCK_COUNT.load(Ordering::Relaxed), len)
}

/// Copy the current in-memory chain for debugging.
pub fn export_chain() -> Option<Vec<Block>> {
    if !is_initialized() {
        return None;
    }
    GLOBAL_BLOCKCHAIN.lock().as_ref().map(|chain| chain.blocks.clone())
}

/// Deliberately non-cryptographic digest for debugging consistency only.
fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut lanes = [
        0xcbf29ce484222325u64,
        0x84222325cbf29ce4u64,
        0x9e3779b97f4a7c15u64,
        0xd6e8feb86659fd93u64,
    ];
    for (index, byte) in data.iter().enumerate() {
        let lane = index & 3;
        lanes[lane] ^= *byte as u64;
        lanes[lane] = lanes[lane].wrapping_mul(0x100000001b3);
        lanes[lane] ^= lanes[lane] >> 29;
    }
    let mut out = [0u8; 32];
    for (index, lane) in lanes.iter().enumerate() {
        out[index * 8..index * 8 + 8].copy_from_slice(&lane.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appended_records_link_and_verify() {
        let mut chain = Blockchain::new();
        chain.add_block(1, 2, "test");
        chain.add_block(2, 3, "another");
        assert_eq!(chain.len(), 3);
        assert!(chain.verify_chain());
    }
}
