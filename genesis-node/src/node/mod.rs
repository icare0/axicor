use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::process::{Command, Child};
use std::env;
use std::sync::atomic::{AtomicU32, Ordering};
use crossbeam::channel::{bounded, Sender, Receiver};
use genesis_compute::ShardEngine;
use crate::network::io_server::ExternalIoServer;
use crate::network::bsp::BspBarrier;
use crate::network::router::RoutingTable;
use crate::network::inter_node::{SpikeBatchHeaderV2, SpikeEventV2, InterNodeRouter};

pub mod recovery;
pub mod shard_thread;

pub enum ComputeCommand {
    RunBatch {
        tick_base: u32,
        batch_size: u32,
        global_dopamine: i16,
    },
    Shutdown,
}

pub enum ComputeFeedback {
    BatchComplete {
        ticks_processed: u32,
        zone_hash: u32,
        pinned_out_ptr: usize,
        output_bytes: usize,
    },
}

pub struct NodeRuntime {
    pub io_server: Arc<ExternalIoServer>,
    pub routing_table: Arc<RoutingTable>,
    pub bsp_barrier: Arc<BspBarrier>,
    pub compute_dispatchers: HashMap<u32, Sender<ComputeCommand>>,
    pub feedback_sender: Sender<ComputeFeedback>,
    pub feedback_receiver: Receiver<ComputeFeedback>,
    pub total_ticks: Arc<AtomicU32>,
    pub local_ip: std::net::Ipv4Addr,
    pub local_port: u16,
    /// [DOD] Маршруты выходов: zone_hash -> (TargetAddr, matrix_hash)
    pub output_routes: HashMap<u32, Vec<(String, u32)>>,
    pub intra_gpu_channels: Vec<(*mut u32, *mut u32, crate::network::intra_gpu::IntraGpuChannel)>,
    pub inter_node_channels: Vec<(*mut u32, crate::network::inter_node::InterNodeChannel)>,
    pub inter_node_router: Arc<crate::network::inter_node::InterNodeRouter>,
    pub axon_head_ptrs: HashMap<u32, *mut u32>,
    pub egress_pool: Arc<crate::network::egress::EgressPool>,
    // [DOD] Владение дочерними процессами (Baker Daemons)
    pub daemons: Mutex<Vec<Child>>,
    // [DOD] Night Phase интервал в тиках
    pub night_interval: u64,
    pub reporter: Arc<crate::simple_reporter::SimpleReporter>,
}

unsafe impl Send for NodeRuntime {}
unsafe impl Sync for NodeRuntime {}

impl NodeRuntime {
    pub fn boot(
        shards: Vec<(u32, ShardEngine, u32, u32, Option<Vec<u32>>, std::path::PathBuf, genesis_core::config::InstanceConfig, std::sync::Arc<crossbeam::queue::SegQueue<genesis_core::ipc::AxonHandoverEvent>>)>,
        io_server: Arc<ExternalIoServer>,
        routing_table: Arc<RoutingTable>,
        bsp_barrier: Arc<BspBarrier>,
        telemetry_swapchain: Arc<crate::network::telemetry::TelemetrySwapchain>,
        local_ip: std::net::Ipv4Addr,
        local_port: u16,
        output_routes: HashMap<u32, Vec<(String, u32)>>,
        intra_gpu_channels: Vec<(*mut u32, *mut u32, crate::network::intra_gpu::IntraGpuChannel)>,
        inter_node_channels: Vec<(*mut u32, crate::network::inter_node::InterNodeChannel)>,
        inter_node_router: Arc<crate::network::inter_node::InterNodeRouter>,
        axon_head_ptrs: HashMap<u32, *mut u32>,
        egress_pool: Arc<crate::network::egress::EgressPool>,
        night_interval: u64,
        reporter: Arc<crate::simple_reporter::SimpleReporter>,
    ) -> Self {
        let (feedback_tx, feedback_rx) = bounded(shards.len() + 32);
        let total_ticks = Arc::new(AtomicU32::new(0));

        let bsp_listener_clone = bsp_barrier.clone();
        tokio::spawn(InterNodeRouter::spawn_ghost_listener(8083, bsp_listener_clone));

        // [DOD] Structured Concurrency: Оркестратор спавнит демонов сам
        let daemons = Self::spawn_baker_daemons(&shards);

        let mut compute_dispatchers = HashMap::new();
        let mut shard_receivers = HashMap::new();
        for (hash, ..) in &shards {
            let (tx, rx) = bounded(1);
            compute_dispatchers.insert(*hash, tx);
            shard_receivers.insert(*hash, rx);
        }

        let rt_handle = tokio::runtime::Handle::current();
        
        let mut zone_idx: u16 = 0;
        for (hash, shard, v_axons, outputs, soma_ids_host, baked_dir, instance_config, incoming_grow) in shards {
            let rx = shard_receivers.remove(&hash).unwrap();
            
            let my_io_ctx = io_server.io_contexts.iter()
                .find(|(h, _)| *h == hash)
                .map(|(_, ctx)| ctx.swapchain.clone())
                .expect("FATAL: IO Context for zone not found");
                
            crate::node::shard_thread::spawn_shard_thread(
                hash, shard, v_axons, outputs, soma_ids_host, baked_dir, instance_config,
                zone_idx, incoming_grow, rt_handle.clone(), night_interval, rx, feedback_tx.clone(),
                bsp_barrier.clone(), my_io_ctx
            );
            zone_idx += 1;
        }

        let node = Self {
            io_server,
            routing_table,
            bsp_barrier,
            compute_dispatchers,
            feedback_sender: feedback_tx,
            feedback_receiver: feedback_rx,
            total_ticks,
            local_ip,
            local_port,
            output_routes,
            intra_gpu_channels,
            inter_node_channels,
            inter_node_router,
            axon_head_ptrs,
            egress_pool,
            daemons: Mutex::new(daemons),
            night_interval,
            reporter,
        };

        node
    }

    fn spawn_baker_daemons(
        shards: &Vec<(u32, ShardEngine, u32, u32, Option<Vec<u32>>, std::path::PathBuf, genesis_core::config::InstanceConfig, std::sync::Arc<crossbeam::queue::SegQueue<genesis_core::ipc::AxonHandoverEvent>>)>,
    ) -> Vec<Child> {
        let mut daemons = Vec::with_capacity(shards.len());
        let exe_path = env::current_exe().expect("FATAL: Failed to get current exe path");
        let daemon_path = exe_path.with_file_name("genesis-baker-daemon");

        for i in 0..shards.len() {
            let zone_idx = i as u16;
            let baked_dir = &shards[i].5; // [DOD FIX] Извлекаем путь к скомпилированной зоне

            // Выжигаем мусорные сокеты от прошлых некорректных завершений
            let socket_path = genesis_core::ipc::default_socket_path(zone_idx);
            let _ = std::fs::remove_file(&socket_path);

            println!("[Orchestrator] Spawning CPU Baker Daemon for zone {} at {:?}", zone_idx, baked_dir);
            let child = Command::new(&daemon_path)
                .arg("--zone")
                .arg(zone_idx.to_string())
                .arg("--baked-dir")
                .arg(baked_dir)
                .spawn()
                .expect("FATAL: Failed to spawn genesis-baker-daemon. Was it compiled?");
            
            daemons.push(child);
        }
        daemons
    }

    // [DOD] Оркестратор изолирован от асинхронного Tokio-контекста
    pub fn run_node_loop(&self, batch_size: u32) {
        let mut current_tick = 0;
        let mut log_counter: u64 = 0;

        // [DOD FIX] Жесткая привязка OS-потока оркестратора к аппаратному контексту
        unsafe { genesis_compute::ffi::gpu_set_device(0); }

        // [DOD] Pre-allocate outbound buffers to avoid heap thrashing (Invariant #3)
        let mut _io_tx_buffer = vec![0u8; genesis_core::constants::MAX_UDP_PAYLOAD];
        let mut _bsp_tx_buffer: Vec<u8> = Vec::with_capacity(genesis_core::constants::MAX_UDP_PAYLOAD);

        println!("[Node] Entering main loop (Batch size: {})", batch_size);

        loop {
            // 1. Wait for Ingress data (Strict BSP network sync)
            self.bsp_barrier.wait_for_data_sync();
            
            // 2. Synchronize BSP and consume Ghost events
            self.bsp_barrier.sync_and_swap();

            // 3. Swap IO Buffers (Acquire semantics)
            for (_, io_ctx) in &self.io_server.io_contexts {
                io_ctx.swapchain.swap();
            }

            // 4. Dispatch batches to compute shards
            let current_dopamine = self.io_server.global_dopamine.load(Ordering::Relaxed) as i16;
            if current_dopamine != 0 && log_counter % 100 == 0 {
                println!("💉 [Node] Dopamine: {}", current_dopamine);
            }

            let num_dispatchers = self.compute_dispatchers.len();
            if num_dispatchers == 0 {
                println!("[!] ERROR: No compute dispatchers found!");
            }
            for tx in self.compute_dispatchers.values() {
                let _ = tx.send(ComputeCommand::RunBatch {
                    tick_base: current_tick as u32,
                    batch_size,
                    global_dopamine: current_dopamine,
                });
            }

            // 5. Collect feedback
            for _ in 0..num_dispatchers {
                if let Ok(feedback) = self.feedback_receiver.recv() {
                    match feedback {
                        ComputeFeedback::BatchComplete { ticks_processed: _, zone_hash, pinned_out_ptr, output_bytes } => {
                            if output_bytes > 0 {
                                // Ship outputs to network targets
                                if let Some(routes) = self.output_routes.get(&zone_hash) {
                                    for (addr, m_hash) in routes {
                                        self.io_server.send_output_batch_pool(
                                            &self.egress_pool,
                                            &addr, 
                                            zone_hash,
                                            *m_hash,
                                            pinned_out_ptr,
                                            output_bytes,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // [DOD] 6. Intra-GPU Ghost Sync + Inter-Node Extraction
            for (src_ptr, dst_ptr, channel) in &self.intra_gpu_channels {
                unsafe { channel.sync_ghosts(*src_ptr, *dst_ptr, std::ptr::null_mut()); }
            }

            for (src_ptr, channel) in &self.inter_node_channels {
                unsafe { channel.extract_spikes(*src_ptr, batch_size, std::ptr::null_mut()); }
            }
            
            // [DOD] 7. GPU Barrier Sync (дожидаемся завершения физики, sync_ghosts и экстракции)
            unsafe { genesis_compute::ffi::gpu_stream_synchronize(std::ptr::null_mut()); }

            // [DOD] 8. Inter-Node Fast Path (Egress)
            for (_, channel) in &self.inter_node_channels {
                let out_count = unsafe { std::ptr::read_volatile(channel.out_count_pinned) };
                if out_count > 0 {
                    let events_slice = unsafe { 
                        std::slice::from_raw_parts(channel.out_events_pinned, out_count as usize) 
                    };
                    // Zero-Copy отправка по UDP
                    self.inter_node_router.flush_outgoing_batch_pool(&self.egress_pool, channel.target_zone_hash, events_slice);
                    self.reporter.udp_out_packets.fetch_add(1, Ordering::Relaxed);
                }
            }

            current_tick += batch_size;
            log_counter  += 1;
            self.reporter.total_ticks.store(current_tick as u64, Ordering::Relaxed);
        }
    }
}

impl Drop for NodeRuntime {
    fn drop(&mut self) {
        let mut daemons = self.daemons.lock().unwrap();
        for (i, daemon) in daemons.iter_mut().enumerate() {
            let _ = daemon.kill(); // Отправляем SIGKILL
            let _ = daemon.wait(); // Ждем подтверждения смерти от ОС, предотвращая зомби
            println!("[Orchestrator] Baker Daemon {} successfully terminated.", i);
        }
    }
}

#[cfg(test)]
mod tests {
    // Legacy isolation test depended on an outdated NodeRuntime::boot and
    // ExternalIoServer API; it was removed to avoid pinning core orchestration
    // signatures to an obsolete test harness.
}
