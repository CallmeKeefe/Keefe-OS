//! Blockchain-based Audit Ledger for KeefeOS
//!
//! Provides immutable logging of security events using blockchain technology.
//! Each block contains a cryptographic hash linking to the previous block.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static mut BLOCKCHAIN_INITIALIZED: AtomicBool = AtomicBool::new(false);
static BLOCK_COUNT: AtomicU64 = AtomicU64::new(0);

/// A single block in the audit chain
#[derive(Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub event_type: u8,
    pub severity: u8,
    pub description_hash: [u8; 32],
    pub previous_hash: [u8; 32],
    pub current_hash: [u8; 32],
    pub nonce: u64,
}

impl Block {
    /// Create a new genesis block
    pub fn genesis() -> Self {
        Self {
            index: 0,
            timestamp: 0,
            event_type: 0,
            severity: 0,
            description_hash: [0; 32],
            previous_hash: [0; 32],
            current_hash: [0; 32],
            nonce: 0,
        }
    }
    
    /// Create a new block with data
    pub fn new(index: u64, event_type: u8, severity: u8, description: &str, previous_hash: [u8; 32]) -> Self {
        let timestamp = get_timestamp();
        let description_hash = hash_data(description.as_bytes());
        
        let mut block = Self {
            index,
            timestamp,
            event_type,
            severity,
            description_hash,
            previous_hash,
            current_hash: [0; 32],
            nonce: 0,
        };
        
        block.current_hash = block.calculate_hash();
        block
    }
    
    /// Calculate the hash of this block
    pub fn calculate_hash(&self) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(&self.index.to_le_bytes());
        data.extend_from_slice(&self.timestamp.to_le_bytes());
        data.push(self.event_type);
        data.push(self.severity);
        data.extend_from_slice(&self.description_hash);
        data.extend_from_slice(&self.previous_hash);
        data.extend_from_slice(&self.nonce.to_le_bytes());
        
        hash_data(&data)
    }
    
    /// Verify the integrity of this block
    pub fn verify(&self) -> bool {
        self.current_hash == self.calculate_hash()
    }
}

/// The blockchain ledger
pub struct Blockchain {
    blocks: Vec<Block>,
}

impl Blockchain {
    /// Create a new blockchain with genesis block
    pub fn new() -> Self {
        let genesis = Block::genesis();
        Self {
            blocks: vec![genesis],
        }
    }
    
    /// Add a new block to the chain
    pub fn add_block(&mut self, event_type: u8, severity: u8, description: &str) {
        let index = self.blocks.len() as u64;
        let previous_hash = self.blocks.last().unwrap().current_hash;
        
        let new_block = Block::new(index, event_type, severity, description, previous_hash);
        self.blocks.push(new_block);
        
        BLOCK_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Verify the entire blockchain integrity
    pub fn verify_chain(&self) -> bool {
        if self.blocks.is_empty() {
            return false;
        }
        
        // Verify genesis block
        if !self.blocks[0].verify() {
            return false;
        }
        
        // Verify each subsequent block
        for i in 1..self.blocks.len() {
            let current = &self.blocks[i];
            let previous = &self.blocks[i - 1];
            
            // Check hash linkage
            if current.previous_hash != previous.current_hash {
                return false;
            }
            
            // Verify block integrity
            if !current.verify() {
                return false;
            }
        }
        
        true
    }
    
    /// Get the number of blocks
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    
    /// Get a specific block by index
    pub fn get_block(&self, index: usize) -> Option<&Block> {
        self.blocks.get(index)
    }
    
    /// Get recent blocks
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

/// Global blockchain instance
static mut GLOBAL_BLOCKCHAIN: Option<Blockchain> = None;

/// Initialize the blockchain subsystem
pub fn init() {
    unsafe {
        GLOBAL_BLOCKCHAIN = Some(Blockchain::new());
        BLOCKCHAIN_INITIALIZED.store(true, Ordering::Release);
    }
    log::info!("Blockchain audit ledger initialized");
}

/// Check if blockchain is initialized
pub fn is_initialized() -> bool {
    unsafe { BLOCKCHAIN_INITIALIZED.load(Ordering::Acquire) }
}

/// Record a security event in the blockchain
pub fn record_event(event_type: super::SecurityEventType, severity: super::SecuritySeverity, description: &str) {
    if !is_initialized() {
        return;
    }
    
    unsafe {
        if let Some(ref mut chain) = GLOBAL_BLOCKCHAIN {
            chain.add_block(event_type as u8, severity as u8, description);
        }
    }
}

/// Verify blockchain integrity
pub fn verify_integrity() -> bool {
    if !is_initialized() {
        return false;
    }
    
    unsafe {
        if let Some(ref chain) = GLOBAL_BLOCKCHAIN {
            return chain.verify_chain();
        }
    }
    false
}

/// Get blockchain statistics
pub fn get_stats() -> (u64, usize) {
    (BLOCK_COUNT.load(Ordering::Relaxed), unsafe {
        GLOBAL_BLOCKCHAIN.as_ref().map(|c| c.len()).unwrap_or(0)
    })
}

/// Simple hash function (replace with SHA-256 in production)
fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
        hash[(i + 1) % 32] ^= byte.wrapping_add(0x9e3779b9) as u8;
    }
    
    for i in 0..32 {
        hash[i] = hash[i].rotate_left((i % 8) as u32);
    }
    
    hash
}

/// Get current timestamp
fn get_timestamp() -> u64 {
    static TIMESTAMP: AtomicU64 = AtomicU64::new(0);
    TIMESTAMP.fetch_add(1, Ordering::Relaxed)
}

/// Export blockchain for external audit
pub fn export_chain() -> Option<Vec<Block>> {
    if !is_initialized() {
        return None;
    }
    
    unsafe {
        GLOBAL_BLOCKCHAIN.as_ref().map(|c| c.blocks.clone())
    }
}
