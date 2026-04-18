#![deny(warnings)]
#![deny(unused_variables)]
#![deny(dead_code)]
pub mod boot;
pub mod network;
pub mod node;
pub mod orchestrator;
pub mod zone_runtime;
pub mod config;
pub mod ipc;
pub mod input;
pub mod output;
pub mod sentinel;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;
use crate::boot::Bootloader;

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default)]
pub enum CpuProfile {
    #[default]
    Aggressive, // 100% CPU, 0ms latency (spin_loop)
    Balanced,   // High CPU, OS scheduler yielding (yield_now)
    Eco,        // Low CPU, sleep polling (sleep 1ms)
}

#[derive(Parser, Debug)]
#[command(
    name = "axicor-node",
    about = "Distributed Axicor Brain Node Daemon",
    version
)]
struct Cli {
    /// Path to .axic archive
    pub archive: PathBuf,

    /// Specific zones to launch (if not specified, all from archive are launched)
    #[arg(long = "zone")]
    pub zones: Vec<String>,

    #[arg(long, default_value = "9000")]
    pub fast_path_port: u16,

    #[arg(long)]
    pub peer: Vec<String>,

    #[arg(long, default_value = "aggressive")]
    pub cpu_profile: CpuProfile,

    #[arg(long)]
    pub cpu: bool,

    #[arg(long)]
    pub log: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    // 1. Initialize dedicated Tokio Runtime for I/O (2 threads max)
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("Fatal: Failed to build Tokio runtime");

    rt.block_on(async {
        let cli = Cli::parse();
        
        println!(" Opening Axic Archive: {:?}", cli.archive);
        let archive = Arc::new(axicor_core::vfs::AxicArchive::open(&cli.archive)
            .context("Failed to open AXIC archive")?);

        // 2. Load topology from archive
        let brain_bytes = archive.get_file("brain.toml")
            .context("brain.toml not found in archive")?;
        let brain_cfg = axicor_core::config::brain::parse_brain_config_from_str(std::str::from_utf8(brain_bytes)?)
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut zones_to_boot = Vec::new();
        if cli.zones.is_empty() {
            // Launch all zones from config
            for zone in &brain_cfg.zones {
                zones_to_boot.push(zone.name.clone());
            }
        } else {
            // Filter by user request
            for zone_name in &cli.zones {
                if brain_cfg.zones.iter().any(|z| &z.name == zone_name) {
                    zones_to_boot.push(zone_name.clone());
                } else {
                    anyhow::bail!("Zone {} not found in archive", zone_name);
                }
            }
        }

        // 3. Export manifests to /dev/shm for SDK compatibility
        for zone_name in &zones_to_boot {
            let zone_name: &String = zone_name;
            let zone_hash = axicor_core::hash::fnv1a_32(zone_name.as_bytes());
            let manifest_shm_path = axicor_core::ipc::manifest_shm_path(zone_hash);
            
            let manifest_vfs_path = format!("baked/{}/manifest.toml", zone_name);
            if let Some(manifest_bytes) = archive.get_file(&manifest_vfs_path) {
                std::fs::write(&manifest_shm_path, manifest_bytes)
                    .with_context(|| format!("Failed to export manifest to {:?}", manifest_shm_path))?;
            }
        }

        println!("[Node] Starting Axicor Distributed Daemon...");
        
        let project_name = cli.archive.file_stem().unwrap().to_str().unwrap().to_string();
        
        // 2-5. Execution of the 5-Component Fail-Fast Boot Sequence
        let use_gpu = !cli.cpu;
        let boot_result = Bootloader::boot_node_with_profile(archive.clone(), &project_name, &zones_to_boot, cli.cpu_profile, use_gpu).await
            .context("Node Bootstrap Failed")?;

        // [DOD FIX] Immediate Cluster Join
        for &local_hash in boot_result.node_runtime.compute_dispatchers.keys() {
            boot_result.node_runtime.broadcast_route_update(local_hash).await;
            println!(" [Node] Cluster joined. Route announced for 0x{:08X}", local_hash);
        }

        println!("[Node] Bootstrap Successful. Hands-off to NodeRuntime.");

        // Spawn IO Receiver Loop
        let io_server = boot_result.node_runtime.services.io_server.clone();
        tokio::spawn(async move {
            io_server.run_rx_loop().await;
        });

        // Spawn Geometry Server
        boot_result.geometry_server.spawn(boot_result.geometry_data);

        // 6. Spawn Dedicated OS Thread for Lock-Free UDP Egress
        let worker_pool = boot_result.egress_pool.clone();
        let profile = cli.cpu_profile;
        std::thread::Builder::new()
            .name("axicor-egress-tx".into())
            .spawn(move || {
                let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
                loop {
                    if let Some(msg) = worker_pool.ready_queue.pop() {
                        let _ = socket.send_to(&msg.buffer[..msg.size], msg.target);
                        worker_pool.free_queue.push(msg).unwrap();
                    } else {
                        match profile {
                            CpuProfile::Aggressive => std::hint::spin_loop(),
                            CpuProfile::Balanced => std::thread::yield_now(),
                            CpuProfile::Eco => std::thread::sleep(std::time::Duration::from_millis(1)),
                        }
                    }
                }
            }).expect("Fatal: Failed to spawn egress worker thread");

        // 7. Enter the high-performance Node Loop (Synchronous GPU / Asynchronous IO)
        // [Architectural Invariant] This loop dispatches work to dedicated OS threads.
        let mut node = boot_result.node_runtime;
        std::thread::Builder::new()
            .name("axicor-orchestrator".into())
            .spawn(move || {
                #[cfg(target_os = "linux")]
                {
                    let mut cpuset: libc::cpu_set_t = unsafe { std::mem::zeroed() };
                    unsafe { libc::CPU_SET(0, &mut cpuset) };
                    let res = unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset) };
                    if res == 0 {
                        println!(" [Core] Orchestrator locked to OS Thread Core 0");
                    }
                }
                node.run_node_loop();
            })
            .expect("Fatal: Failed to spawn orchestrator OS thread");

        // 8. Wait for termination
        tokio::signal::ctrl_c().await.unwrap();
        println!("[Node] Shutting down...");
        Ok(())
    })
}

