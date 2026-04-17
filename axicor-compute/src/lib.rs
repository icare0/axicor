//! # Axicor Compute
//!
//! Hardware-native execution backend for the Axicor engine.
//! Implements the Dual-Backend C-ABI abstraction, providing bit-exact
//! Integer Physics (GLIF, GSOP) across NVIDIA (CUDA) and AMD (HIP/ROCm) GPUs.
//!
//! This crate operates exclusively on flat, Headerless SoA memory dumps.
//! Dynamic VRAM allocations (`cudaMalloc`/`hipMalloc`) are strictly prohibited
//! inside the Day Phase hot loop. All network-to-VRAM state patching (Dynamic Capacity Routing)
//! MUST occur exclusively during the BSP synchronization barrier.
//!
//! ## Module Index
//!
//! - `[cuda]` — NVIDIA execution backend (nvcc). Enforces 32-thread Warp alignment and Warp-Aggregated Telemetry.
//! - `[amd]` — AMD execution backend (hipcc). Enforces 64-thread Wavefront alignment.
//! - `[ffi]` — Zero-cost C-ABI bindings for cross-boundary DMA transactions.

#![deny(warnings)]
#![deny(unused_variables)]
#![deny(dead_code)]
pub mod compute;
pub mod ffi;
pub mod memory;
pub mod bindings;
pub mod cpu;

#[cfg(feature = "mock-gpu")]
pub mod mock_ffi;

#[cfg(not(feature = "mock-gpu"))]
pub use ffi::*;

#[cfg(feature = "mock-gpu")]
pub use mock_ffi::*;

pub use compute::shard::ShardEngine;
pub use ffi::ShardVramPtrs;
pub use memory::{VramState, StateOffsets, calculate_state_blob_size, compute_state_offsets};
