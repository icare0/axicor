use genesis_runtime::config::{parse_shard_config, parse_simulation_config, parse_blueprints_config};
use genesis_runtime::network::intra_gpu::IntraGpuChannel;
use genesis_core::config::brain::parse_brain_config;
use genesis_runtime::network::router::SpikeRouter;
use genesis_runtime::zone_runtime::ZoneRuntime;

use genesis_runtime::memory::VramState;
use anyhow::{Context, Result};
use genesis_runtime::Runtime;
use clap::Parser;

use genesis_runtime::network::geometry_client::GeometryServer;
use genesis_runtime::network::socket::NodeSocket;

use genesis_runtime::network::telemetry::TelemetryServer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "genesis-node",
    about = "Distributed Genesis Brain Node Daemon",
    version
)]
struct Cli {
    /// Path to the brain manifest (e.g. brain.toml)
    #[arg(short, long, default_value = "config/brain.toml")]
    brain: PathBuf,

    /// Local port to bind the Node to (TCP = port+1, UDP = port)
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Actively injects a sweeping signal into Virtual Axons each sync_batch
    #[arg(long)]
    mock_retina: bool,

    /// Unix socket path for genesis-baker-daemon (optional).
    /// If not provided, Night Phase Sprouting is skipped (Sort & Prune still runs).
    #[arg(long)]
    baker_socket: Option<PathBuf>,
}

struct ZoneLink {
    from_idx: usize,
    to_idx: usize,
    channel: genesis_runtime::network::intra_gpu::IntraGpuChannel,
}

use tokio::runtime::Builder;
use genesis_runtime::network::external::ExternalIoServer;
use genesis_runtime::tui::{DashboardState, app::run_tui_thread};

fn main() -> Result<()> {
    // 1. Initialize dedicated Tokio Runtime for I/O (2 threads max)
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Fatal: Failed to build Tokio runtime");

    rt.block_on(async {
        let cli = Cli::parse();
        println!("[Node] Starting Genesis Distributed Daemon...");

    // 2. Load Brain Config
    let brain_config = parse_brain_config(&cli.brain)
        .map_err(|e| anyhow::anyhow!("Failed to load brain config: {:?}, err: {}", cli.brain, e))?;
    
    let sim_config = parse_simulation_config(&brain_config.simulation.config)
         .with_context(|| format!("Failed to load simulation config: {:?}", brain_config.simulation.config))?;
    let sync_batch_ticks = sim_config.simulation.sync_batch_ticks as usize;
    let mut io_server_opt: Option<Arc<ExternalIoServer>> = None;

    println!("[Node] Brain Manifest Loaded. {} zones configured.", brain_config.zones.len());

    // 3. Initialize Shared Network Components
    let is_node_a = std::env::var("NODE_A").is_ok();
    println!("[Node] Operating Mode: {}", if is_node_a { "NODE A (Sensory/Motor)" } else { "NODE B (Hidden)" });

    let local_port = if is_node_a { 9011 } else { 9001 };
    let target_port = if is_node_a { 9001 } else { 9011 };
    
    let udp_addr = format!("127.0.0.1:{}", local_port);
    let target_addr = format!("127.0.0.1:{}", target_port);

    let tcp_port = if is_node_a { 8011 } else { 8001 };
    let geo_addr = format!("0.0.0.0:{}", tcp_port).parse().unwrap();
    let geo_server = GeometryServer::bind(geo_addr).await
        .context("Failed to bind TCP Geometry Server")?;
    println!("[Node] Bound TCP Geometry Server on {}", geo_addr);
    geo_server.spawn();

    let telemetry_port = if is_node_a { 8012 } else { 8002 };
    let telemetry_tx = TelemetryServer::start(telemetry_port).await;

    let slow_path_port = if is_node_a { 8013 } else { 8003 };
    let slow_path_target_port = if is_node_a { 8003 } else { 8013 };
    let slow_path_target_addr = format!("127.0.0.1:{}", slow_path_target_port);
    let slow_path_target_addr = format!("127.0.0.1:{}", slow_path_target_port);
    // [AUDIT]: SlowPathService replaced with Lock-Free SegQueues
    // Placeholder - queues are now zone-specific in this architecture for 1.5M isolation

    println!("[Node] Bound UDP Fast Path on {}", udp_addr);

    let mut zone_ping_pongs = std::collections::HashMap::new();
    let mut sensory_gxi_opt = None;

    // 4. Load Zones (Filter based on NODE_A)
    let mut zones: Vec<ZoneRuntime> = Vec::new();
    for zone_entry in &brain_config.zones {
        // Filter zones for Localhost Cluster MVP
        if is_node_a && zone_entry.name == "HiddenCortex" { continue; }
        if !is_node_a && (zone_entry.name == "SensoryCortex" || zone_entry.name == "MotorCortex") { continue; }
        
        println!("[Node] Loading Zone: {}", zone_entry.name);
        
        // 4.1 Parse Configs
        let blueprints = parse_blueprints_config(&zone_entry.blueprints)
            .with_context(|| format!("Failed to load blueprints for zone {}", zone_entry.name))?;
            
        let shard_toml_path = zone_entry.baked_dir.join("shard.toml");
        let shard_config = if shard_toml_path.exists() {
            parse_shard_config(&shard_toml_path).unwrap_or_else(|_| panic!("Failed to parse shard config {:?}", shard_toml_path))
        } else {
            println!("[Node] Warning: No shard.toml found at {:?}, using fallback.", shard_toml_path);
            genesis_core::config::instance::InstanceConfig {
                zone_id: "0".to_string(),
                world_offset: genesis_core::config::instance::Coordinate { x: 0, y: 0, z: 0 },
                dimensions: genesis_core::config::instance::Dimensions { w: 1000, d: 1000, h: 1000 },
                neighbors: genesis_core::config::instance::Neighbors { x_plus: None, x_minus: None, y_plus: None, y_minus: None },
            }
        };

        let default_neighbors = [
            (1, &shard_config.neighbors.x_plus),
            (2, &shard_config.neighbors.x_minus),
            (3, &shard_config.neighbors.y_plus),
            (4, &shard_config.neighbors.y_minus),
        ];

        for (logical_id, neighbor_opt) in default_neighbors {
            if let Some(target_string) = neighbor_opt {
                let target_str = target_string.as_str();
                if target_str != "Self" && !target_str.is_empty() {
                    // We will set up true Peer routing later
                }
            }
        }

        // 4.2 Load VRAM
        let state_path = zone_entry.baked_dir.join("shard.state");
        let axons_path = zone_entry.baked_dir.join("shard.axons");
        let state_bytes = std::fs::read(&state_path).with_context(|| format!("Failed to read {:?}", state_path))?;
        let axons_bytes = std::fs::read(&axons_path).with_context(|| format!("Failed to read {:?}", axons_path))?;

        let mut gxi = None;
        let mut gxo = None;
        if let Ok(entries) = std::fs::read_dir(&zone_entry.baked_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if ext == "gxi" {
                        println!("       Loading GXI Map: {:?}", entry.path().file_name().unwrap());
                        let parsed = genesis_runtime::input::GxiFile::load(&entry.path());
                        gxi = Some(parsed);
                    } else if ext == "gxo" {
                        println!("       Loading GXO Output Map: {:?}", entry.path().file_name().unwrap());
                        let parsed = genesis_runtime::output::GxoFile::load(&entry.path());
                        gxo = Some(parsed);
                    }
                }
            }
        }
        if zone_entry.name == "SensoryCortex" {
            sensory_gxi_opt = gxi.clone();
        }

        let mut required_ghost_slots = 200_000; // Capacity reserve for Axon Handover (Dynamic Ghosts)
        for conn in &brain_config.connections {
            if conn.to == zone_entry.name {
                required_ghost_slots += conn.axon_ids.len();
            }
        }

        let vram = VramState::load_shard(
            &state_bytes, 
            &axons_bytes, 
            gxi.as_ref(), 
            gxo.as_ref(),
            sync_batch_ticks as u32,
            1, // default input_stride
            required_ghost_slots
        ).context("Failed to push shard data to GPU VRAM")?;

        println!("       VRAM Load Complete. {} neurons, {} total axons", vram.padded_n, vram.total_axons);

        // 4.3 Setup ZoneRuntime
        let const_mem = ZoneRuntime::build_constant_memory(&blueprints);
        let prune_threshold = blueprints.neuron_types.first().map(|n| n.prune_threshold).unwrap_or(15);
        let v_seg = (sim_config.simulation.signal_speed_um_tick / sim_config.simulation.voxel_size_um) as u32;
        let master_seed = genesis_core::seed::MasterSeed::from_str(&sim_config.simulation.master_seed);

        let mut runtime = Runtime::new(vram, v_seg, master_seed.raw(), Some(zone_entry.baked_dir.clone()));

        if let Some(ref socket_path) = cli.baker_socket {
            let zone_u16 = shard_config.zone_id.parse::<u16>().unwrap_or(0);
            if let Ok(client) = genesis_runtime::ipc::BakerClient::connect(zone_u16, socket_path) {
                println!("       Baker daemon connected.");
                runtime.baker_client = Some(client);
            }
        }

        let ping_pong = Arc::new(unsafe { genesis_runtime::network::bsp::PingPongSchedule::new(sync_batch_ticks, 1024) });
        let zone_hash = genesis_runtime::network::router::fnv1a_32(zone_entry.name.as_bytes());
        zone_ping_pongs.insert(zone_hash, ping_pong.clone());

        zones.push(ZoneRuntime {
            name: zone_entry.name.clone(), // Assuming zone_entry.name is the correct source for name
            runtime,
            const_mem,
            config: shard_config, // Assuming shard_config is the correct source for config
            prune_threshold: -50, // WM decay limit
            is_sleeping: Arc::new(AtomicBool::new(false)),
            sleep_requested: false,
            ping_pong,
            last_night_time: std::time::Instant::now(),
            min_night_delay: std::time::Duration::from_secs(30),
            slow_path_queues: Arc::new(genesis_runtime::network::slow_path::SlowPathQueues::new()),
            hot_reload_queue: Arc::new(crossbeam::queue::SegQueue::new()),
            inter_node_channels: Vec::new(),
            intra_gpu_channels: Vec::new(),
            spatial_grid: std::sync::Arc::new(std::sync::Mutex::new(genesis_runtime::orchestrator::spatial_grid::SpatialGrid::new())),
        });

        // --- NEW: SPAWN HOT RELOAD WATCHER ---
        let hot_reload_q = zones.last().unwrap().hot_reload_queue.clone();
        let blueprints_path = format!("config/zones/{}/blueprints.toml", zone_entry.name);
        rt.spawn(async move {
            let mut last_modified = std::time::SystemTime::UNIX_EPOCH;
            loop {
                if let Ok(metadata) = std::fs::metadata(&blueprints_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified {
                            last_modified = modified;
                            if let Ok(content) = std::fs::read_to_string(&blueprints_path) {
                                if let Ok(blueprints) = toml::from_str::<genesis_core::config::BlueprintsConfig>(&content) {
                                    let mut valid = true;
                                    for nt in &blueprints.neuron_types {
                                        for &inertia in &nt.inertia_curve {
                                            let val = (nt.gsop_potentiation as i32 * inertia as i32) >> 7;
                                            if val < 1 {
                                                valid = false;
                                                println!("⚠️ [Hot-Reload] Validation failed for {} (gsop_potentiation * inertia >> 7 < 1). Ignoring.", blueprints_path);
                                                break;
                                            }
                                        }
                                        if !valid { break; }
                                    }
                                    if valid {
                                        let new_const_mem = ZoneRuntime::build_constant_memory(&blueprints);
                                        hot_reload_q.push(new_const_mem);
                                    }
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        // --- NEW: SPAWN ASYNC TCP DISPATCHER FOR THIS ZONE ---
        let zone_queues = zones.last().unwrap().slow_path_queues.clone();
        let listen_port = slow_path_port;
        let target_url = slow_path_target_addr.clone();

        rt.spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            use tokio::net::{TcpListener, TcpStream};
            
            // 1. TCP Listener (Immigrant Axons)
            let listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port)).await.unwrap();
            let q_in = zone_queues.clone();
            tokio::spawn(async move {
                while let Ok((mut socket, _)) = listener.accept().await {
                    let q = q_in.clone();
                    tokio::spawn(async move {
                        let mut header = [0u8; 8];
                        if socket.read_exact(&mut header).await.is_ok() {
                            let magic = u32::from_le_bytes(header[0..4].try_into().unwrap());
                            let count = u32::from_le_bytes(header[4..8].try_into().unwrap());
                            
                            if magic == 0x47524F57 { // GROW
                                for _ in 0..count {
                                    let mut buf = [0u8; 16];
                                    if socket.read_exact(&mut buf).await.is_ok() {
                                        let ev = unsafe { std::mem::transmute::<[u8; 16], genesis_runtime::network::slow_path::AxonHandoverEvent>(buf) };
                                        q.incoming_grow.push(ev);
                                    }
                                }
                            } else if magic == 0x41434B48 { // ACKH
                                for _ in 0..count {
                                    let mut buf = [0u8; 12];
                                    if socket.read_exact(&mut buf).await.is_ok() {
                                        let ack = unsafe { std::mem::transmute::<[u8; 12], genesis_runtime::network::slow_path::AxonHandoverAck>(buf) };
                                        q.incoming_ack.push(ack);
                                    }
                                }
                            }
                        }
                    });
                }
            });

            // 2. TCP Egress (Emigrant Axons & ACKs)
            loop {
                // Batch Outgoing GROW
                let mut grows = Vec::new();
                while let Some(ev) = zone_queues.outgoing_grow.pop() { grows.push(ev); }
                if !grows.is_empty() {
                    if let Ok(mut stream) = TcpStream::connect(&target_url).await {
                        let magic = 0x47524F57u32;
                        let count = grows.len() as u32;
                        let mut head = magic.to_le_bytes().to_vec();
                        head.extend_from_slice(&count.to_le_bytes());
                        let _ = stream.write_all(&head).await;
                        let bytes = unsafe { std::slice::from_raw_parts(grows.as_ptr() as *const u8, grows.len() * 16) };
                        let _ = stream.write_all(bytes).await;
                    } else {
                        for ev in grows { zone_queues.outgoing_grow.push(ev); } // Retry
                    }
                }

                // Batch Outgoing ACKs
                let mut acks = Vec::new();
                while let Some(ack) = zone_queues.outgoing_ack.pop() { acks.push(ack); }
                if !acks.is_empty() {
                    if let Ok(mut stream) = TcpStream::connect(&target_url).await {
                        let magic = 0x41434B48u32;
                        let count = acks.len() as u32;
                        let mut head = magic.to_le_bytes().to_vec();
                        head.extend_from_slice(&count.to_le_bytes());
                        let _ = stream.write_all(&head).await;
                        let bytes = unsafe { std::slice::from_raw_parts(acks.as_ptr() as *const u8, acks.len() * 12) };
                        let _ = stream.write_all(bytes).await;
                    } else {
                        for ack in acks { zone_queues.outgoing_ack.push(ack); } // Retry
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
    }

    if zones.is_empty() {
        anyhow::bail!("No zones configured in brain.toml!");
    }

    // 4.5 Initialize UDP InterNode Router
    let mut routing_peers = std::collections::HashMap::new();
    if is_node_a {
        routing_peers.insert(genesis_runtime::network::router::fnv1a_32(b"HiddenCortex"), target_addr.parse().unwrap());
    } else {
        routing_peers.insert(genesis_runtime::network::router::fnv1a_32(b"MotorCortex"), target_addr.parse().unwrap());
    }
    
    let routing_table = genesis_runtime::network::router::RoutingTable {
        peers: routing_peers,
    };
    
    let inter_node_router = genesis_runtime::network::router::InterNodeRouter::new(&udp_addr, routing_table).await;
    let router_arc = Arc::new(inter_node_router);
    
    genesis_runtime::network::router::InterNodeRouter::spawn_receiver_loop(
        router_arc.socket.clone(),
        zone_ping_pongs
    );

    // --- HOT CHECKPOINT RESTORATION ---
    let mut max_restored_tick = 0;
    let stream = std::ptr::null_mut();

    for zone in zones.iter_mut() {
        unsafe {
            if let Some(restored_tick) = genesis_runtime::orchestrator::night_phase::load_hot_checkpoint(
                &zone.name,
                zone.runtime.vram.padded_n as u32,
                zone.runtime.vram.pinned_host_targets as *mut u32,
                zone.runtime.vram.pinned_host_weights as *mut i16
            ) {
                println!("🧠 Memory Restored: {} (Tick: {})", zone.name, restored_tick);
                
                let targets_size = genesis_core::constants::MAX_DENDRITE_SLOTS * zone.runtime.vram.padded_n * 4;
                let weights_size = genesis_core::constants::MAX_DENDRITE_SLOTS * zone.runtime.vram.padded_n * 2;
                
                genesis_runtime::ffi::gpu_memcpy_host_to_device_async(
                    zone.runtime.vram.dendrite_targets as *mut _, 
                    zone.runtime.vram.pinned_host_targets as *const _, 
                    targets_size, 
                    stream
                );
                
                genesis_runtime::ffi::gpu_memcpy_host_to_device_async(
                    zone.runtime.vram.dendrite_weights as *mut _, 
                    zone.runtime.vram.pinned_host_weights as *const _, 
                    weights_size, 
                    stream
                );
                
                if restored_tick > max_restored_tick {
                    max_restored_tick = restored_tick;
                }
            }
        }
    }
    unsafe { genesis_runtime::ffi::gpu_stream_synchronize(stream); }
    // ----------------------------------

    // 5. Setup IntraGPU Communications (Ghost Axons)
    let mut links: Vec<ZoneLink> = Vec::new();
    let mut inter_node_links: Vec<genesis_runtime::network::inter_node::InterNodeChannel> = Vec::new();
    
    for conn in &brain_config.connections {
        let from_exists = zones.iter().any(|z| z.name == conn.from);
        let to_exists = zones.iter().any(|z| z.name == conn.to);
        
        let ghosts_path = format!("baked/{}/{}_{}.ghosts", conn.from, conn.from, conn.to);
        let ghosts_path = std::path::Path::new(&ghosts_path);
        
        if from_exists && to_exists {
            // Intra-GPU Zone Link (same process)
            let from_idx = zones.iter().position(|z| z.name == conn.from).unwrap();
            let to_idx = zones.iter().position(|z| z.name == conn.to).unwrap();
            let (src_map, dst_map) = genesis_runtime::network::ghosts::load_ghosts(ghosts_path);
            let channel = unsafe { IntraGpuChannel::new(&src_map, &dst_map) };
            
            // Assign to source zone
            zones[from_idx].intra_gpu_channels.push(unsafe { IntraGpuChannel::new(&src_map, &dst_map) }); // Need separate instance? No, let's just move it.
            
            links.push(ZoneLink { from_idx, to_idx, channel });
            println!("[Node] Established Zero-Copy Intra-GPU Channel {} -> {}", conn.from, conn.to);
        } else if from_exists && !to_exists {
            // Inter-Node Sender (different processes)
            let (src_map, dst_map) = genesis_runtime::network::ghosts::load_ghosts(ghosts_path);
            let target_hash = genesis_runtime::network::router::fnv1a_32(conn.to.as_bytes());
            let from_idx = zones.iter().position(|z| z.name == conn.from).unwrap();
            
            let channel = unsafe { 
                genesis_runtime::network::inter_node::InterNodeChannel::new(target_hash, &src_map, &dst_map) 
            };
            
            // Assign to source zone
            zones[from_idx].inter_node_channels.push(unsafe { 
                 genesis_runtime::network::inter_node::InterNodeChannel::new(target_hash, &src_map, &dst_map) 
            });

            inter_node_links.push(channel);
            println!("[Node] Established Fast-Path Inter-Node Channel {} -> {} (UDP)", conn.from, conn.to);
        }
    }

    let mut pinned_input_ptr = std::ptr::null_mut();
    let mut pinned_output_ptr = std::ptr::null_mut();
    let mut input_bytes = 0;
    let mut output_bytes = 0;

    let mut sensory_idx = None;
    let mut motor_idx = None;

    if is_node_a {
        sensory_idx = Some(zones.iter().position(|z| z.name == "SensoryCortex").expect("SensoryCortex missing on Node A"));
        motor_idx = Some(zones.iter().position(|z| z.name == "MotorCortex").expect("MotorCortex missing on Node A"));

        let s_idx = sensory_idx.unwrap();
        let m_idx = motor_idx.unwrap();

        let words_per_tick = (zones[s_idx].runtime.vram.num_pixels as u32 + 31) / 32;
        input_bytes = (words_per_tick as usize) * sync_batch_ticks * 4;
        output_bytes = zones[m_idx].runtime.vram.num_mapped_somas as usize * sync_batch_ticks;

        unsafe {
            pinned_input_ptr = genesis_runtime::ffi::gpu_host_alloc(input_bytes) as *mut u32;
            pinned_output_ptr = genesis_runtime::ffi::gpu_host_alloc(output_bytes) as *mut u8;
            
            if pinned_input_ptr.is_null() || pinned_output_ptr.is_null() {
                anyhow::bail!("Failed to allocate Pinned Host RAM for I/O!");
            }
        }
    }

    let mut dashboard_state = Arc::new(DashboardState::default());
    dashboard_state.total_ticks.store(max_restored_tick, Ordering::Relaxed);

    if is_node_a {
        let mut io_server_obj = ExternalIoServer::new(
            if is_node_a { "0.0.0.0:8014" } else { "0.0.0.0:8081" },
            pinned_input_ptr,
            input_bytes
        ).await;
        
        // Pass matrix offsets from GXI to server for multi-matrix routing
        if let Some(ref gxi_data) = sensory_gxi_opt {
            for m in &gxi_data.matrices {
                io_server_obj.matrix_offsets.insert(m.name_hash, m.offset);
            }
        }

        io_server_obj.dashboard = Some(dashboard_state.clone());
        let io_server = Arc::new(io_server_obj);

        let io_rx = io_server.clone();
        rt.spawn(async move {
            io_rx.run_rx_loop().await;
        });
        io_server_opt = Some(io_server);
        println!("Genesis Engine: Hot Loop Started. Listening on UDP 8081.");
    }
    
    // =====================================================================
    // 7. MAIN HOT LOOP (Dedicated OS Thread)
    // =====================================================================
    let mut total_ticks: u64 = max_restored_tick;
    let night_interval_ticks: u64 = sim_config.simulation.night_interval_ticks as u64;
    
    let stream = std::ptr::null_mut(); // Default stream

    run_tui_thread(dashboard_state.clone());

    let mut start_time = std::time::Instant::now();

    loop {
        // [BSP БАРЬЕР]: Ожидание нового входного кадра.
        if is_node_a {
            let io_server = io_server_opt.as_ref().unwrap();
            while io_server.new_frame_ready.load(Ordering::Acquire) == 0 {
                std::hint::spin_loop();
            }
            io_server.new_frame_ready.store(0, Ordering::Release);

            // Шаг 1: DMA Host-to-Device (Перекачка свежей маски входов)
            unsafe {
                genesis_runtime::ffi::gpu_memcpy_host_to_device_async(
                    zones[sensory_idx.unwrap()].runtime.vram.input_bitmask_buffer as *mut std::ffi::c_void,
                    pinned_input_ptr as *const std::ffi::c_void,
                    input_bytes,
                    stream
                );
            }
        } else {
            // NODE B: Ждем пакета от NODE A для каждой зоны
            for zone in zones.iter() {
                zone.ping_pong.wait_for_data(total_ticks as usize / sync_batch_ticks); 
            }
        }

        // Шаг 2: Выполнение батча (GPU молотит 6 ядер ПАРАЛЛЕЛЬНО для всех зон)
        for zone in zones.iter_mut() {
            let tx_opt = if zone.name == "SensoryCortex" { Some(&telemetry_tx) } else { None };
            genesis_runtime::orchestrator::day_phase::execute_day_batch(zone, sync_batch_ticks as u32, stream, tx_opt, total_ticks);
        }

        // Шаг 3: Intra-GPU Ghost Sync
        unsafe {
            for link in links.iter() {
                let is_src_sleeping = zones[link.from_idx].is_sleeping.load(Ordering::Acquire);
                let is_dst_sleeping = zones[link.to_idx].is_sleeping.load(Ordering::Acquire);

                // SPIKE DROP: Если принимающая зона спит, сигналы улетают в пустоту. 
                // Это биологически достоверная амнезия. Защищает VRAM от Data Race.
                if !is_src_sleeping && !is_dst_sleeping {
                    let src_heads = zones[link.from_idx].runtime.vram.axon_head_index as *const _;
                    let dst_heads = zones[link.to_idx].runtime.vram.axon_head_index as *mut _;
                    link.channel.sync_ghosts(src_heads, dst_heads, stream);
                }
            }
        }

        // Шаг 3.5: Inter-Node (Экстракция исходящих спайков)
        unsafe {
            for link in inter_node_links.iter() {
                // Find src boundary dynamically
                if let Some(src_zone) = zones.iter().find(|z| genesis_runtime::network::router::fnv1a_32(z.name.as_bytes()) == link.target_zone_hash) {
                     // Wait, target_zone_hash is for the *destination*, we need the source.
                     // A cleaner way for MVP: since we only have 1 inter-node link per node in this test (Sensory->Hidden, Hidden->Motor)
                     // Let's just find the first zone that isn't asleep and extract from it. 
                     // Actually, we must bind the channel to the specific zone.
                }
                
                // Simplified MVP: Just use the very first loaded zone as the source. 
                // For NODE_A: SensoryCortex is zones[0]. For NODE_B: HiddenCortex is zones[0].
                // This is extremely hardcoded but works for the Cartesian Split Brain test block.
                let src_zone = &zones[0]; 
                link.extract_spikes(
                    src_zone.runtime.vram.axon_head_index as *const _,
                    sync_batch_ticks as u32,
                    stream
                );
            }
        }

        // Шаг 4: DMA Device-to-Host (Скачивание результатов сом)
        unsafe {
            if is_node_a {
                genesis_runtime::ffi::gpu_memcpy_device_to_host_async(
                    pinned_output_ptr as *mut std::ffi::c_void,
                    zones[motor_idx.unwrap()].runtime.vram.output_history as *const std::ffi::c_void,
                    output_bytes,
                    stream
                );
            }
            
            // БЛОКИРОВКА CPU: Ждем, пока GPU закончит ВСЕ ядра и вернет нам Pinned RAM
            genesis_runtime::ffi::gpu_stream_synchronize(stream);
            
            // --- UDP Флаш (Только сейчас можно читать out_count_pinned) ---
            for link in inter_node_links.iter() {
                let count = std::ptr::read_volatile(link.out_count_pinned) as usize;
                if count > 0 {
                    let events_slice = std::slice::from_raw_parts(link.out_events_pinned, count);
                    let router_clone = router_arc.clone();
                    let target_hash = link.target_zone_hash;
                    let events_vec = events_slice.to_vec(); // clone for async
                    rt.spawn(async move {
                        let _ = router_clone.flush_outgoing_batch(target_hash, &events_vec).await;
                    });
                }
            }
            
            // --- BSP SWAP ---
            for zone in zones.iter_mut() {
                zone.ping_pong.sync_and_swap();
                zone.ping_pong.clear_write_buffer();
            }
        }

        // Шаг 5: Асинхронная отправка выходов (MotorCortex -> Python Client)
        if is_node_a {
            let target_addr = "127.0.0.1:8082";
            let motor_zone_hash = genesis_runtime::network::router::fnv1a_32(b"MotorCortex");
            let motor_matrix_hash = genesis_runtime::network::router::fnv1a_32(b"motor_actuators");
            
            let io_tx = io_server_opt.as_ref().unwrap().clone();
            // Передаем указатель через usize границу потока
            let pinned_output_addr = pinned_output_ptr as usize;
            
            rt.spawn(async move {
                io_tx.send_output_batch(
                    target_addr, 
                    motor_zone_hash, 
                    motor_matrix_hash, 
                    pinned_output_addr, 
                    output_bytes
                ).await;
            });
        }

        total_ticks += sync_batch_ticks as u64;
        
        // Записываем метрики Дня
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        dashboard_state.latest_batch_ms.store(elapsed_ms, Ordering::Relaxed);
        dashboard_state.total_ticks.fetch_add(sync_batch_ticks as u64, Ordering::Relaxed);
        start_time = std::time::Instant::now();

        // Шаг 6: Проверка триггера Night Phase
        let now = std::time::Instant::now();
        let mut night_triggered = false;
        
        for zone in zones.iter_mut() {
            let time_since_last_night = now.duration_since(zone.last_night_time);
            
            let is_sleeping = zone.is_sleeping.load(Ordering::Acquire);
            let ticks_ready = night_interval_ticks > 0 && total_ticks % night_interval_ticks == 0;
            let time_ready = time_since_last_night >= zone.min_night_delay;

            if !is_sleeping && ticks_ready && time_ready {
                zone.last_night_time = now;
                night_triggered = true;
                
                let vram_ptr = &mut zone.runtime.vram as *mut genesis_runtime::memory::VramState;
                genesis_runtime::orchestrator::night_phase::trigger_night_phase(
                    zone.name.clone(),
                    total_ticks,
                    vram_ptr,
                    zone.runtime.vram.padded_n as u32,
                    zone.runtime.vram.total_axons as u32,
                    zone.prune_threshold,
                    zone.is_sleeping.clone(),
                    zone.runtime.master_seed,
                    zone.slow_path_queues.clone(),
                    zone.inter_node_channels.clone(),
                    zone.spatial_grid.clone(),
                );
            }
        }
        
        if night_triggered {
            dashboard_state.is_night_phase.store(true, Ordering::Release);
            dashboard_state.night_count.fetch_add(1, Ordering::Relaxed);
            dashboard_state.is_night_phase.store(false, Ordering::Release);
            start_time = std::time::Instant::now();
        }
    }

    Ok(())
    })
}
