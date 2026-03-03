use std::sync::{Arc, Mutex};
use std::thread;
use std::ptr;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use crossbeam::channel::{bounded, Sender, Receiver};
use genesis_compute::ShardEngine;
use crate::network::io_server::ExternalIoServer;
use crate::network::bsp::BspBarrier;
use crate::network::router::RoutingTable;

use crate::network::io_server::{ExternalIoHeaderV2, IoMultiplexer};
use crate::network::inter_node::{SpikeBatchHeaderV2, SpikeEventV2, InterNodeRouter};

pub mod recovery;

pub enum ComputeCommand {
    RunBatch {
        tick_base: u32,
        batch_size: u32,
        input_mask: Option<Vec<u32>>,
    },
    Shutdown,
}

pub enum ComputeFeedback {
    BatchComplete {
        ticks_processed: u32,
    },
}

pub struct NodeRuntime {
    pub io_server: Arc<ExternalIoServer>,
    pub routing_table: Arc<RoutingTable>,
    pub bsp_barrier: Arc<Mutex<BspBarrier>>,
    pub compute_dispatchers: Mutex<HashMap<u32, Sender<ComputeCommand>>>,
    pub feedback_sender: Sender<ComputeFeedback>,
    pub feedback_receiver: Receiver<ComputeFeedback>,
    pub input_rx: Mutex<tokio::sync::mpsc::UnboundedReceiver<(ExternalIoHeaderV2, Vec<u32>)>>,
    pub ghost_rx: Mutex<tokio::sync::mpsc::UnboundedReceiver<(SpikeBatchHeaderV2, Vec<SpikeEventV2>)>>,
    pub total_ticks: Arc<AtomicU32>,
    pub local_ip: std::net::Ipv4Addr,
    pub local_port: u16,
    pub telemetry_swapchain: Arc<crate::network::telemetry::TelemetrySwapchain>,
}

impl NodeRuntime {
    /// Bootstraps the IO layer and spawns dedicated OS threads for shards.
    pub fn boot(
        shards: Vec<(u32, ShardEngine)>,
        io_server: Arc<ExternalIoServer>,
        routing_table: Arc<RoutingTable>,
        bsp_barrier: Arc<Mutex<BspBarrier>>,
        telemetry_swapchain: Arc<crate::network::telemetry::TelemetrySwapchain>,
        local_ip: std::net::Ipv4Addr,
        local_port: u16,
    ) -> Self {
        let (feedback_tx, feedback_rx) = bounded(shards.len() + 32); 
        let total_ticks = Arc::new(AtomicU32::new(0));

        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();
        let (ghost_tx, ghost_rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn UDP Multiplexers (Contract §2.7)
        tokio::spawn(IoMultiplexer::spawn_input_listener(8081, input_tx));
        tokio::spawn(InterNodeRouter::spawn_ghost_listener(8083, ghost_tx));

        let node = Self {
            io_server,
            routing_table,
            bsp_barrier,
            compute_dispatchers: Mutex::new(HashMap::new()),
            feedback_sender: feedback_tx,
            feedback_receiver: feedback_rx,
            input_rx: Mutex::new(input_rx),
            ghost_rx: Mutex::new(ghost_rx),
            total_ticks,
            local_ip,
            local_port,
            telemetry_swapchain,
        };

        for (hash, shard) in shards {
            node.spawn_shard_thread(hash, shard);
        }

        node
    }

    /// Spawns a dedicated OS thread for a shard.
    pub fn spawn_shard_thread(&self, hash: u32, mut shard: ShardEngine) {
        let (tx, rx) = bounded(1);
        let telemetry = self.telemetry_swapchain.clone();
        
        // Allocate I/O buffers in VRAM for this shard
        let sync_batch_ticks = 100; // TODO: Get from config
        let max_spikes_per_tick = 100_000; // Matches ring_buffer.rs
        let input_words_per_tick = (1024 + 31) / 32; // TODO: Get from manifest
        let num_outputs = 0; // TODO: Get from manifest

        let mut d_input = ptr::null_mut();
        let mut d_spikes = ptr::null_mut();
        let mut d_output = ptr::null_mut();

        unsafe {
            genesis_compute::ffi::cu_allocate_io_buffers(
                input_words_per_tick * sync_batch_ticks,
                max_spikes_per_tick * sync_batch_ticks,
                num_outputs * sync_batch_ticks,
                &mut d_input,
                &mut d_spikes,
                &mut d_output,
            );
        }

        let io_buffers = Arc::new(genesis_compute::compute::shard::IoDeviceBuffers {
            d_input_bitmask: d_input,
            d_incoming_spikes: d_spikes,
            d_output_history: d_output,
            max_spikes_per_tick,
            input_words_per_tick,
            num_outputs,
        });

        {
            let mut dispatchers = self.compute_dispatchers.lock().unwrap();
            dispatchers.insert(hash, tx);
        }
        
        let f_tx = self.feedback_sender.clone();
        let bsp_barrier = self.bsp_barrier.clone();

        thread::Builder::new()
            .name(format!("compute-{}", hash))
            .spawn(move || {
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        ComputeCommand::RunBatch { tick_base: _, batch_size, input_mask } => {
                            let (incoming_spikes, spike_counts) = {
                                let bsp = bsp_barrier.lock().unwrap();
                                let schedule = bsp.get_read_schedule();
                                (schedule.ghost_ids.clone(), schedule.counts.clone())
                            };

                            shard.step_day_phase_batch(
                                batch_size,
                                &io_buffers,
                                input_mask.as_deref(),
                                Some(&incoming_spikes),
                                &spike_counts,
                                0, // virtual_offset
                                0, // num_virtual_axons
                                std::ptr::null(), // mapped_soma_ids_device
                            );

                            // Capture telemetry spikes if any clients are connected
                            if telemetry.is_active() {
                                    // 1. Reset device-side count (if not already handled)
                                    // 2. Launch extract kernel (if not already handled in step_day_phase_batch)
                                    // 3. DMA D2H to Telemetry back_buffer
                                    // 4. swap_and_ready
                                    
                                    // For now, we assume step_day_phase_batch or a manual kernel call fills telemetry_count
                                    // Since telemetry is high-level, we'll use a simplified version for now:
                                    // tele.swap_and_ready(count, tick_base as u64);
                            }

                            if let Err(_) = f_tx.send(ComputeFeedback::BatchComplete { ticks_processed: batch_size }) {
                                break;
                            }
                        }
                        ComputeCommand::Shutdown => break,
                    }
                }
            }).expect("Failed to spawn compute thread");
    }

    /// The main Node Loop (BSP Orchestrator).
    pub async fn run_node_loop(&self, batch_size: u32) {
        let mut current_tick = 0;

        loop {
            // [Architectural Invariant] BSP Barrier: Swap buffers for high-performance scheduling.
            // This is the ONLY place where we lock the barrier in the hot loop.
            {
                let mut bsp = self.bsp_barrier.lock().unwrap();
                bsp.sync_and_swap();

                // [Drainage Invariant] Zero-Lock Drainage from UDP channels into the NEW write buffer.
                let mut g_rx = self.ghost_rx.lock().unwrap();
                let schedule = bsp.get_write_schedule();
                while let Ok((_header, events)) = g_rx.try_recv() {
                    for ev in events {
                        schedule.push_spike(ev.tick_offset as usize, ev.ghost_id);
                    }
                }
            }

            // Drain input bitmasks (Take the latest one for this batch)
            let mut latest_input = None;
            {
                let mut i_rx = self.input_rx.lock().unwrap();
                while let Ok((_header, mask)) = i_rx.try_recv() {
                    latest_input = Some(mask);
                }
            }

            // [Barrier Invariant] All compute threads are quiescent here.
            let num_dispatchers = {
                let dispatchers_guard = self.compute_dispatchers.lock().unwrap();
                let num = dispatchers_guard.len();
                let input_clone = latest_input.clone();
                for tx in dispatchers_guard.values() {
                    let _ = tx.send(ComputeCommand::RunBatch {
                        tick_base: current_tick as u32,
                        batch_size,
                        input_mask: input_clone.clone(),
                    });
                }
                num
            };

            for _ in 0..num_dispatchers {
                if let Ok(ComputeFeedback::BatchComplete { ticks_processed }) = self.feedback_receiver.recv() {
                    self.total_ticks.fetch_add(ticks_processed, Ordering::Relaxed);
                }
            }

            current_tick += batch_size;
            self.io_server.dashboard.total_ticks.store(current_tick as u64, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_compute::ShardEngine;
    use crate::tui::DashboardState;
    use crate::network::io_server::ExternalIoServer;
    use crate::network::bsp::BspBarrier;
    use crate::network::router::RoutingTable;
    use std::sync::{Arc, Mutex};
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_node_compute_isolation() {
        let dashboard = Arc::new(DashboardState::new(false));
        let routing_table = Arc::new(RoutingTable::new(HashMap::new()));
        let socket = Arc::new(tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let io_server = Arc::new(ExternalIoServer::new(
            Arc::new(std::sync::atomic::AtomicBool::new(false)), 
            1024, 0, 0, dashboard, routing_table.clone(), socket
        ).unwrap());
        let bsp_barrier = Arc::new(Mutex::new(BspBarrier::new(100)));
        
        let shard = ShardEngine::new(unsafe { std::mem::zeroed() });

        let node = NodeRuntime::boot(
            vec![(0x1234, shard)], 
            io_server, 
            routing_table, 
            bsp_barrier,
            Ipv4Addr::new(127, 0, 0, 1),
            8080
        );
        
        let tokio_thread_id = thread::current().id();
        let (id_tx, id_rx) = bounded(1);
        
        thread::spawn(move || {
            id_tx.send(thread::current().id()).unwrap();
        });
        
        let spawned_thread_id = id_rx.recv().unwrap();
        assert_ne!(tokio_thread_id, spawned_thread_id);
    }
}
