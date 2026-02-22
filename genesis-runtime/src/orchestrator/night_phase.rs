use crate::Runtime;
use genesis_baker::bake::reconnect_empty_dendrites;
use tokio::sync::mpsc::error::TryRecvError;
use crate::network::slow_path::{GeometryRequest, GeometryResponse, AckNewAxon};

pub struct NightPhase;

impl NightPhase {
    pub fn check_and_run(runtime: &mut Runtime, zone_id: u32, night_interval_ticks: u32, current_total_ticks: u64) -> bool {
        if night_interval_ticks == 0 {
            return false;
        }

        if current_total_ticks > 0 && current_total_ticks % (night_interval_ticks as u64) == 0 {
            Self::run_maintenance_pipeline(runtime, zone_id);
            return true;
        }

        false
    }

    fn run_maintenance_pipeline(runtime: &mut Runtime, zone_id: u32) {
        println!("Night Phase triggered for zone {}: Running Maintenance Pipeline", zone_id);
        
        // 1. Sort & Prune (GPU)
        println!("1. Sort & Prune (GPU)");
        let prune_threshold: i16 = 15; // TODO: Fetch from Zone Configuration
        runtime.vram.run_sort_and_prune(prune_threshold);
        runtime.synchronize(); // Ensure kernel completes before download

        // 2. PCIe Download (VRAM -> RAM)
        println!("2. PCIe Download (VRAM -> RAM)");
        let mut _weights = runtime.vram.download_dendrite_weights().expect("Failed to download weights");
        let mut _targets = runtime.vram.download_dendrite_targets().expect("Failed to download targets");

        // 3. Sprouting (CPU/Cone Tracing)
        println!("3. Sprouting & Nudging (CPU)");
        reconnect_empty_dendrites(
            &mut _targets,
            &mut _weights,
            runtime.vram.padded_n,
            &runtime.neurons,
            &runtime.axons,
            &runtime.neuron_types,
            runtime.master_seed,
        );
        
        // 4. Baking
        println!("4. Baking - Density repacking handled by genesis_baker.");

        // 4.5. Process incoming Geography/Structural mutations from neighbors
        println!("4.5 Process Cross-Shard Geometry Handshakes");
        Self::process_geometry_requests(runtime, zone_id);

        // 5. PCIe Upload (RAM -> VRAM)
        println!("5. PCIe Upload (RAM -> VRAM)");
        // Upload the mutated structural data back to the GPU
        runtime.vram.upload_dendrite_weights(&_weights).expect("Failed to upload weights");
        runtime.vram.upload_dendrite_targets(&_targets).expect("Memcpy async failed");
        
        // Ensure network streams are ready
        runtime.synchronize();

        println!("Night Phase complete.");
        runtime.synchronize();
    }

    fn process_geometry_requests(runtime: &mut Runtime, _zone_id: u32) {
        let mut rx = match runtime.geometry_receiver.take() {
            Some(r) => r,
            None => return,
        };

        loop {
            match rx.try_recv() {
                Ok((req, resp_tx)) => {
                    let resp = Self::handle_geometry_request(runtime, req);
                    let _ = resp_tx.send(resp);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        runtime.geometry_receiver = Some(rx);
    }

    fn handle_geometry_request(_runtime: &mut Runtime, req: GeometryRequest) -> GeometryResponse {
        match req {
            GeometryRequest::Handover(axon) => {
                println!("Received Handover request from Axon ID {}", axon.source_axon_id);
                // TODO: Graph allocation of Ghost Axons in `runtime.axons`
                GeometryResponse::Ack(AckNewAxon {
                    source_axon_id: axon.source_axon_id,
                    ghost_id: 9999, // Stub ID
                })
            }
            GeometryRequest::Prune(prune) => {
                println!("Received Prune request for Ghost ID {}", prune.ghost_id);
                // TODO: Free ghost axle slot
                GeometryResponse::Ok
            }
        }
    }
}
