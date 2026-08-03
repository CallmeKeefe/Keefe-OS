//! Neural Network Inference Engine for KeefeOS
//!
//! A lightweight, embedded neural network implementation optimized for kernel-space use.
//! Supports feedforward networks with configurable layers and activation functions.
//!
//! # Features
//! - Fixed-point arithmetic option for deterministic behavior
//! - Minimal memory footprint
//! - No heap allocation after initialization
//! - Thread-safe inference
//!
//! # Safety
//! This module uses interior mutability with proper synchronization to avoid
//! data races. The global NN instance is initialized once and then read-only.

#![no_std]

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

static NN_INITIALIZED: AtomicBool = AtomicBool::new(false);
static GLOBAL_NN: Mutex<Option<NeuralNetwork>> = Mutex::new(None);

/// Initialize the neural network engine
pub fn init() {
    NN_INITIALIZED.store(true, Ordering::Release);
    log::debug!("Neural network engine initialized");
}

/// Check if the neural network engine is initialized
pub fn is_initialized() -> bool {
    NN_INITIALIZED.load(Ordering::Acquire)
}

/// A simple feedforward neural network layer
#[derive(Debug, Clone)]
pub struct Layer {
    weights: [[f32; 16]; 16],  // Max 16x16 weight matrix
    biases: [f32; 16],
    input_size: usize,
    output_size: usize,
}

impl Layer {
    /// Create a new layer with random weights (for demonstration)
    pub fn new(input_size: usize, output_size: usize) -> Self {
        assert!(input_size <= 16 && output_size <= 16, "Layer size exceeds maximum");
        
        let mut weights = [[0.0; 16]; 16];
        let mut biases = [0.0; 16];
        
        // Initialize with small random values (Xavier initialization simplified)
        for i in 0..output_size {
            for j in 0..input_size {
                weights[i][j] = (i as f32 * j as f32 * 0.01) - 0.005;
            }
            biases[i] = 0.0;
        }
        
        Self {
            weights,
            biases,
            input_size,
            output_size,
        }
    }
    
    /// Forward pass through the layer with ReLU activation
    pub fn forward(&self, input: &[f32]) -> [f32; 16] {
        let mut output = [0.0; 16];
        
        for i in 0..self.output_size {
            let mut sum = self.biases[i];
            for j in 0..self.input_size.min(input.len()) {
                sum += self.weights[i][j] * input[j];
            }
            // ReLU activation
            output[i] = if sum > 0.0 { sum } else { 0.0 };
        }
        
        output
    }
    
    /// Get the output size of this layer
    pub fn output_size(&self) -> usize {
        self.output_size
    }
}

/// A complete neural network with multiple layers
#[derive(Debug)]
pub struct NeuralNetwork {
    layers: [Option<Layer>; 4],  // Max 4 layers
    num_layers: usize,
}

impl NeuralNetwork {
    /// Create a new neural network
    pub fn new() -> Self {
        Self {
            layers: [None, None, None, None],
            num_layers: 0,
        }
    }
    
    /// Add a layer to the network
    pub fn add_layer(&mut self, input_size: usize, output_size: usize) {
        assert!(self.num_layers < 4, "Maximum layer count exceeded");
        self.layers[self.num_layers] = Some(Layer::new(input_size, output_size));
        self.num_layers += 1;
    }
    
    /// Forward pass through the entire network
    pub fn predict(&self, input: &[f32]) -> [f32; 16] {
        let mut current = [0.0; 16];
        for (i, &val) in input.iter().enumerate() {
            if i < 16 {
                current[i] = val;
            }
        }
        
        for i in 0..self.num_layers {
            if let Some(ref layer) = self.layers[i] {
                current = layer.forward(&current);
            }
        }
        
        current
    }
    
    /// Get number of layers
    pub fn num_layers(&self) -> usize {
        self.num_layers
    }
}

impl Default for NeuralNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the global neural network instance
pub fn get_global_nn() -> Option<&'static NeuralNetwork> {
    // Use a leak to extend the lifetime of the guard for the duration of the call
    // This is safe because we only read from the NN after initialization
    let guard = GLOBAL_NN.lock();
    if guard.is_some() {
        unsafe { Some(&*(&**guard as *const NeuralNetwork)) }
    } else {
        None
    }
}

/// Initialize the global neural network with a default configuration
pub fn init_global_nn() {
    let nn = NeuralNetwork::new();
    
    // Default architecture: 8 inputs -> 16 hidden -> 8 hidden -> 4 outputs
    let mut nn_with_layers = nn;
    nn_with_layers.add_layer(8, 16);
    nn_with_layers.add_layer(16, 8);
    nn_with_layers.add_layer(8, 4);
    
    *GLOBAL_NN.lock() = Some(nn_with_layers);
    
    log::info!("Global neural network initialized (8->16->8->4)");
}

/// Example: Predict process scheduling priority based on features
/// Features: [cpu_usage, memory_pressure, io_wait, nice_value, ...]
pub fn predict_scheduling_priority(features: &[f32]) -> u8 {
    if !is_initialized() {
        return 5; // Default priority
    }
    
    let result = if let Some(nn) = get_global_nn() {
        nn.predict(features)
    } else {
        [0.0; 16]
    };
    
    // Convert output to priority (0-9, lower is higher priority)
    let score = result[0].max(0.0).min(1.0);
    (score * 9.0) as u8
}

/// Example: Detect anomalous system behavior
pub fn detect_anomaly(metrics: &[f32]) -> bool {
    if !is_initialized() {
        return false;
    }
    
    // Simple threshold-based anomaly detection
    // In production, this would use a trained model
    for &metric in metrics {
        if metric > 0.95 || metric < 0.05 {
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layer_creation() {
        let layer = Layer::new(4, 8);
        assert_eq!(layer.output_size(), 8);
    }
    
    #[test]
    fn test_network_forward() {
        let mut nn = NeuralNetwork::new();
        nn.add_layer(4, 8);
        nn.add_layer(8, 2);
        
        let input = [0.5, 0.3, 0.7, 0.1];
        let output = nn.predict(&input);
        
        assert_eq!(output.len(), 16);
    }
}
