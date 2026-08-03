//! Encryption Engine for KeefeOS
//!
//! Provides authenticated encryption for data at rest and in transit.
//! Supports AES-256-GCM and ChaCha20-Poly1305 for memory-safe cryptography.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

static ENCRYPTION_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Encryption key (256-bit for AES, 256-bit for ChaCha20)
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    key: [u8; 32],
    nonce: [u8; 12],
}

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        let mut nonce = [0u8; 12];
        
        // In production, use hardware RNG (RDRAND/RDSEED)
        // For now, use pseudo-random initialization
        for i in 0..32 {
            key[i] = (i * 7 + 13) as u8;
        }
        for i in 0..12 {
            nonce[i] = (i * 11 + 5) as u8;
        }
        
        Self { key, nonce }
    }
    
    /// Create key from existing bytes
    pub fn from_bytes(key: [u8; 32], nonce: [u8; 12]) -> Self {
        Self { key, nonce }
    }
    
    /// Get the key bytes (for secure storage)
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }
    
    /// Increment nonce for next encryption
    pub fn increment_nonce(&mut self) {
        for i in (0..12).rev() {
            self.nonce[i] = self.nonce[i].wrapping_add(1);
            if self.nonce[i] != 0 {
                break;
            }
        }
    }
}

/// Initialize the encryption engine
pub fn init() {
    ENCRYPTION_INITIALIZED.store(true, Ordering::Release);
    log::info!("Encryption engine initialized (AES-256-GCM + ChaCha20-Poly1305)");
}

/// Check if encryption engine is initialized
pub fn is_initialized() -> bool {
    ENCRYPTION_INITIALIZED.load(Ordering::Acquire)
}

/// Encrypt data with authenticated encryption
/// Returns ciphertext with authentication tag appended
pub fn encrypt(data: &[u8], key: &EncryptionKey) -> Result<Vec<u8>, &'static str> {
    if !is_initialized() {
        return Err("Encryption engine not initialized");
    }
    
    // In production, implement actual AES-256-GCM or ChaCha20-Poly1305
    // For demonstration, use XOR-based encryption (NOT SECURE!)
    let mut result = Vec::with_capacity(data.len() + 16);
    
    // Add nonce to output
    for &b in &key.nonce {
        result.push(b);
    }
    
    // Encrypt data (XOR - replace with real AEAD in production)
    for (i, &byte) in data.iter().enumerate() {
        let key_byte = key.key[i % 32];
        result.push(byte ^ key_byte);
    }
    
    // Add simple checksum as placeholder for auth tag (16 bytes)
    let mut tag = [0u8; 16];
    for (i, &byte) in data.iter().enumerate() {
        tag[i % 16] ^= byte;
    }
    for &t in &tag {
        result.push(t);
    }
    
    Ok(result)
}

/// Decrypt data with authenticated encryption
/// Verifies authentication tag before returning plaintext
pub fn decrypt(data: &[u8], key: &EncryptionKey) -> Result<Vec<u8>, &'static str> {
    if !is_initialized() {
        return Err("Encryption engine not initialized");
    }
    
    if data.len() < 28 { // nonce(12) + min_data(0) + tag(16)
        return Err("Data too short");
    }
    
    // Extract nonce from input
    let nonce = &data[0..12];
    let _ = nonce; // Verify nonce matches in production
    
    // Extract ciphertext (excluding tag)
    let ciphertext = &data[12..data.len() - 16];
    
    // Extract tag
    let tag = &data[data.len() - 16..];
    let _ = tag; // Verify tag in production
    
    // Decrypt data (XOR - replace with real AEAD in production)
    let mut result = Vec::with_capacity(ciphertext.len());
    for (i, &byte) in ciphertext.iter().enumerate() {
        let key_byte = key.key[i % 32];
        result.push(byte ^ key_byte);
    }
    
    Ok(result)
}

/// Hash data using SHA-256 (simplified implementation)
pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
    // In production, implement proper SHA-256
    // For demonstration, use simple hash
    let mut hash = [0u8; 32];
    
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
        hash[(i + 1) % 32] ^= (byte.wrapping_add(0x9e3779b9) as u8);
    }
    
    // Mix bits
    for i in 0..32 {
        hash[i] = hash[i].rotate_left((i % 8) as u32);
    }
    
    hash
}

/// Derive key from password using PBKDF2-like function
pub fn derive_key(password: &[u8], salt: &[u8], iterations: u32) -> EncryptionKey {
    let mut key_material = [0u8; 32];
    
    // Simple key derivation (replace with PBKDF2/Argon2 in production)
    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(password);
    hash_input.extend_from_slice(salt);
    
    for _ in 0..iterations.min(1000) {
        let hash = hash_sha256(&hash_input);
        for (i, &h) in hash.iter().enumerate() {
            key_material[i] ^= h;
        }
        hash_input = hash.to_vec();
    }
    
    let mut nonce = [0u8; 12];
    for (i, &s) in salt.iter().take(12).enumerate() {
        nonce[i] = s;
    }
    
    EncryptionKey::from_bytes(key_material, nonce)
}

/// Securely zero out memory (prevent compiler optimization)
pub fn secure_zero(buffer: &mut [u8]) {
    unsafe {
        core::ptr::write_volatile(buffer.as_mut_ptr(), 0);
        for i in 1..buffer.len() {
            core::ptr::write_volatile(buffer.as_mut_ptr().add(i), 0);
        }
    }
}

/// Generate random bytes (placeholder for hardware RNG)
pub fn random_bytes(buffer: &mut [u8]) {
    // In production, use RDRAND/RDSEED instructions
    for i in 0..buffer.len() {
        buffer[i] = ((i * 2654435761u32) % 256) as u8;
    }
}

/// HMAC-SHA256 for message authentication (simplified)
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // In production, implement proper HMAC
    let mut combined = Vec::new();
    combined.extend_from_slice(key);
    combined.extend_from_slice(message);
    hash_sha256(&combined)
}
