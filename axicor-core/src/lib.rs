//! # Axicor Core
//!
//! Shared types, constants, and SoA (Structure of Arrays) memory layout for the Axicor 
//! spiking neural network engine. This crate enforces zero-cost C-ABI contracts 
//! for cross-platform DMA and GPU execution, strictly avoiding runtime allocations.
//!
//! ## Module Index
//!
//! - `[layout]`  GPU-aligned data structures (`BurstHeads8`, `VariantParameters`).
//! - `[ipc]`  Cross-platform shared memory and Zero-Copy IPC primitives.
//! - `[physics]`  Integer GLIF neuron model and GSOP plasticity math.
//! - `[signal]`  Axon propagation math (branchless, zero-float).

#![deny(warnings)]
#![deny(unused_variables)]
#![deny(dead_code)]
pub mod hash;
pub mod config;
pub mod constants;
pub mod coords;
pub mod ipc;
pub mod layout;
pub mod physics;
pub mod seed;
pub mod signal;
pub mod time;
pub mod types;
pub mod vfs;

#[cfg(test)]
#[path = "test_gsop_math.rs"]
mod test_gsop_math;

#[cfg(test)]
mod test_tick;
