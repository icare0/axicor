pub mod geometry_client;
pub mod intra_gpu;
pub mod ring_buffer;
pub mod slow_path;
pub mod telemetry;
// pub mod external;
pub mod bsp;
pub mod channel;
pub mod egress;
pub mod ghosts;
pub mod inter_node;
pub mod io_server;
pub mod replication;
pub mod router;
pub mod socket;

// Removed bytemuck imports as they are unused here

// Spike contracts now moved to axicor-core::ipc
pub use axicor_core::ipc::{SpikeBatchHeader, SpikeEvent};

#[cfg(test)]
mod test_intra_gpu;
