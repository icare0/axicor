use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;

use genesis_core::config::manifest::ZoneManifest;
use genesis_compute::memory::{VramState, calculate_state_blob_size};
use genesis_compute::ShardEngine;
use crate::zone_runtime::ZoneRuntime;
use crate::network::geometry_client::GeometryServer;
use crate::network::telemetry::TelemetryServer;
use crate::network::io_server::ExternalIoServer;
use crate::network::bsp::BspBarrier;
use crate::node::NodeRuntime;
use crate::network::router::RoutingTable;

pub struct Bootloader;

pub struct BootResult {
    pub node_runtime: NodeRuntime,
    pub geometry_server: GeometryServer,
    pub geometry_data: Vec<[f32; 4]>,
    pub telemetry_swapchain: Arc<crate::network::telemetry::TelemetrySwapchain>,
}

/// Инициализирует ShardEngine прямым DMA-копированием из .state и .axons.
/// Этот метод реализует O(1) деривацию размеров на основе файлового контракта:
/// - .state: 782 байта на нейрон (SoA)
/// - .axons: 4 байта на аксон (u32 heads)
pub fn boot_shard_from_disk(baked_dir: &Path) -> Result<ShardEngine> {
    let state_path = baked_dir.join("shard.state");
    let axons_path = baked_dir.join("shard.axons");

    // 1. Zero-Parsing загрузка сырых байт в RAM
    let state_blob = std::fs::read(&state_path)
        .with_context(|| format!("FATAL: Missing .state file at {:?}", state_path))?;
    let axons_blob = std::fs::read(&axons_path)
        .with_context(|| format!("FATAL: Missing .axons file at {:?}", axons_path))?;

    // 2. Деривация размеров за O(1) (782 байта/нейрон)
    let padded_n = (state_blob.len() / 782) as u32;
    let total_axons = (axons_blob.len() / 4) as u32;

    // Верификация целостности (защита от битых файлов)
    let (_, expected_state_size) = calculate_state_blob_size(padded_n as usize);
    if state_blob.len() != expected_state_size {
        anyhow::bail!(
            "FATAL: .state blob size mismatch for {:?}. Expected {}, got {}. Corruption or version skew!",
            state_path, expected_state_size, state_blob.len()
        );
    }

    // 3. Выделение VRAM и DMA-заливка
    let vram = VramState::allocate(padded_n, total_axons);
    vram.upload_state(&state_blob);
    vram.upload_axon_heads(&axons_blob);

    Ok(ShardEngine::new(vram))
}

impl Bootloader {
    /// Full node bootstrap sequence.
    pub async fn boot_node(manifest_path: &Path, dashboard: Arc<crate::tui::DashboardState>) -> Result<BootResult> {
        let manifest_toml = std::fs::read_to_string(manifest_path)
            .with_context(|| format!("Failed to read manifest: {:?}", manifest_path))?;
        let manifest: ZoneManifest = toml::from_str(&manifest_toml)
            .with_context(|| format!("Failed to parse manifest: {:?}", manifest_path))?;

        let baked_dir = manifest_path.parent().unwrap_or(std::path::Path::new("."));
        let local_port = manifest.network.fast_path_udp_local;

        // 1. Servers
        let geo_addr = format!("0.0.0.0:{}", local_port + 1).parse()?;
        let geometry_server = GeometryServer::bind(geo_addr).await?;
        let telemetry_swapchain = TelemetryServer::start(local_port + 2).await;

        // 2. Shard Loading
        let (zone, engine, geo_data) = Self::load_zone(manifest_path, dashboard.clone())?;

        // 3. Orchestration
        let bsp_barrier = Arc::new(std::sync::Mutex::new(BspBarrier::new(100)));
        let routing_table = Arc::new(RoutingTable::new(HashMap::new()));

        // 4. External IO
        let io_socket = tokio::net::UdpSocket::bind(&format!("0.0.0.0:{}", manifest.network.external_udp_in)).await?;
        let io_server = Arc::new(ExternalIoServer::new(
            Arc::new(AtomicBool::new(false)),
            1024, 0, 0,
            dashboard.clone(),
            routing_table.clone(),
            Arc::new(io_socket)
        ).unwrap());

        let node_runtime = NodeRuntime::boot(
            vec![(manifest.zone_hash, engine)],
            io_server,
            routing_table,
            bsp_barrier,
            telemetry_swapchain.clone(),
            std::net::Ipv4Addr::new(127, 0, 0, 1),
            manifest.network.fast_path_udp_local,
        );

        Ok(BootResult {
            node_runtime,
            geometry_server,
            geometry_data: geo_data,
            telemetry_swapchain,
        })
    }

    fn load_zone(manifest_path: &Path, dashboard: Arc<crate::tui::DashboardState>) -> Result<(ZoneRuntime, ShardEngine, Vec<[f32; 4]>)> {
        let manifest_toml = std::fs::read_to_string(manifest_path)?;
        let manifest: ZoneManifest = toml::from_str(&manifest_toml)?;
        let baked_dir = manifest_path.parent().unwrap_or(std::path::Path::new("."));
        
        let engine = boot_shard_from_disk(baked_dir)?;

        // Load geometry data
        let geom_path = baked_dir.join("shard.geom");
        let geom_blob = std::fs::read(&geom_path)
            .with_context(|| format!("FATAL: Missing .geom file at {:?}", geom_path))?;
        let geo_data: Vec<[f32; 4]> = bytemuck::cast_slice(&geom_blob).to_vec();

        let mut const_mem = [genesis_core::config::manifest::GpuVariantParameters::default(); 16];
        for variant in manifest.variants {
            let idx = variant.id as usize;
            if idx < 16 {
                const_mem[idx] = variant.into_gpu();
            }
        }

        let is_sleeping = Arc::new(AtomicBool::new(false));

        let zone = ZoneRuntime {
            name: format!("Zone_{:08X}", manifest.zone_hash),
            artifact_dir: baked_dir.to_path_buf(),
            const_mem,
            config: Default::default(),
            prune_threshold: -50,
            is_sleeping,
            sleep_requested: false,
            last_night_time: std::time::Instant::now(),
            min_night_delay: std::time::Duration::from_secs(30),
            slow_path_queues: Arc::new(crate::network::slow_path::SlowPathQueues::new()),
            hot_reload_queue: Arc::new(crossbeam::queue::SegQueue::new()),
            inter_node_channels: Vec::new(),
            intra_gpu_channels: Vec::new(),
            spatial_grid: Arc::new(std::sync::Mutex::new(crate::orchestrator::spatial_grid::SpatialGrid::new())),
            dashboard,
        };

        Ok((zone, engine, geo_data))
    }
}
