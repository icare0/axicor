#![deny(warnings)]
#![deny(unused_variables)]
#![deny(dead_code)]
use clap::Parser;
use std::path::{Path, PathBuf};
use anyhow::Result;
use std::collections::HashMap;

use axicor_baker::{bake, parser, validator};
use axicor_core::config::brain::BrainConfig;
use axicor_core::config::manifest::{ModelManifest, ManifestConnection};

#[derive(Parser)]
#[command(
    name = "baker",
    about = "Genesis Baking Tool — TOML configs → binary .state/.axons blobs + ghost maps"
)]
#[command(version)]
struct Cli {
    #[arg(long)]
    model: Option<PathBuf>,

    #[arg(long, default_value = "config/brain.toml")]
    brain: PathBuf,

    #[arg(long, default_value_t = false)]
    clean: bool,

    #[arg(long, default_value_t = false)]
    yes: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(model_path) = &cli.model {
        bake_entire_model(model_path, cli.clean, cli.yes)
    } else {
        bake_single_brain(&cli.brain, cli.clean, cli.yes)
    }
}

fn bake_single_brain(brain_path: &Path, clean: bool, yes: bool) -> Result<()> {
    let brain_config = axicor_core::config::brain::parse_brain_config(brain_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    let project_dir = brain_path.parent().unwrap_or(Path::new("."));
    let sim_path = project_dir.join(&brain_config.simulation.config);

    if clean {
        perform_clean(&brain_config, project_dir, yes)?;
    }

    println!("[baker] Processing Brain Architecture: {} zones", brain_config.zones.len());
    let mut compiled_zones = HashMap::new();

    for (zone_idx, zone) in brain_config.zones.iter().enumerate() {
        let bp_path = project_dir.join(&zone.blueprints);
        let an_path = project_dir.join(&zone.anatomy);
        let io_path = project_dir.join(&zone.io);
        let baked_dir = project_dir.join(&zone.baked_dir);
        let shard_cfg_path = an_path.parent().unwrap().join("shard.toml");

        let workspace = parse_and_validate(
            &brain_config, &sim_path, &bp_path, &an_path, &io_path, &shard_cfg_path, &baked_dir, &zone.name, zone_idx as u16,
        )?;

        let (shard_soa, compiled_shard, v_seg, num_virtual, gxis, gxos) = build_local_topology(&workspace);
        serialize_artifacts(&shard_soa, &workspace, num_virtual, v_seg, &gxis, &gxos)?;
        compiled_zones.insert(zone.name.clone(), compiled_shard);
    }

    establish_ghost_links(&brain_config, &compiled_zones, project_dir)?;

    let project_name = project_dir.file_name().unwrap().to_str().unwrap();
    let axic_path = project_dir.parent().unwrap().join(format!("{}.axic", project_name));
    println!("\n[baker] 📦 Packing project into VFS Archive: {:?}", axic_path);
    bake::axic::pack_directory_to_axic(project_dir, &axic_path)?;

    Ok(())
}

fn bake_entire_model(model_path: &Path, clean: bool, yes: bool) -> Result<()> {
    println!("[baker] Orchestrating Model Bake: {:?}", model_path);
    let sim_config = axicor_core::config::SimulationConfig::load(model_path)
        .map_err(|e| anyhow::anyhow!(e))?;
    
    let project_dir = model_path.parent().unwrap_or(Path::new("."));
    
    let mut model_manifest = ModelManifest {
        magic: 0x4D4F444C, // "MODL"
        model_id_v1: sim_config.model_id_v1.clone(),
        departments: HashMap::new(),
        connections: Vec::new(),
    };

    // 1. Bake each department
    for dept in &sim_config.departments {
        println!("\n[baker] >>> Baking Department: {} <<<", dept.name);
        let brain_path = project_dir.join(&dept.config);
        bake_single_brain(&brain_path, clean, yes)?;
        
        // Track the department ID for the master manifest
        if let Ok(brain_cfg) = axicor_core::config::brain::parse_brain_config(&brain_path) {
            if let Some(meta) = brain_cfg.depart_id_v1 {
                model_manifest.departments.insert(dept.name.clone(), meta.id);
            }
        }
    }

    // 2. Map global connections
    for conn in &sim_config.connections {
        model_manifest.connections.push(ManifestConnection {
            conn_id_v1: conn.conn_id_v1.clone(),
            from: conn.from.clone(),
            to: conn.to.clone(),
            width: conn.width,
            height: conn.height,
        });
    }

    // 3. Write Master Manifest
    let manifest_toml = toml::to_string(&model_manifest).expect("Failed to serialize model manifest");
    let manifest_path = project_dir.join("manifest.toml");
    std::fs::write(&manifest_path, manifest_toml)?;
    println!("[baker] ✓ Written global manifest.toml");

    Ok(())
}

fn perform_clean(brain_config: &BrainConfig, project_dir: &Path, yes: bool) -> Result<()> {
    if !yes {
        print!("[baker] WARNING: This will permanently delete all baked models. Continue? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        if input.trim().to_lowercase() != "y" {
            println!("Aborting clean operation.");
            return Ok(());
        }
    }

    println!("[baker] Clean flag set. Wiping baked directories...");
    for zone in &brain_config.zones {
        let abs_baked_dir = project_dir.join(&zone.baked_dir);
        if abs_baked_dir.exists() {
            for entry in std::fs::read_dir(&abs_baked_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() { std::fs::remove_dir_all(path)?; } 
                else { std::fs::remove_file(path)?; }
            }
        }
    }
    Ok(())
}

fn establish_ghost_links(brain_config: &BrainConfig, compiled_zones: &HashMap<String, bake::layout::CompiledShard>, project_dir: &Path) -> Result<()> {
    if brain_config.connections.is_empty() { return Ok(()); }
    println!("\n[baker] === Baking Ghost Axon Mappings ===");

    for conn in &brain_config.connections {
        let src_shard = compiled_zones.get(&conn.from).expect("Source zone missing");
        let dst_shard = compiled_zones.get(&conn.to).expect("Dest zone missing");
        let dst_ghost_offset = dst_shard.local_axons_count as u32;

        let target_zone_rel = &brain_config.zones.iter().find(|z| z.name == conn.to).unwrap().baked_dir;
        let out_dir = project_dir.join(target_zone_rel);

        let sent_ghosts = if let (Some(w), Some(h)) = (conn.width, conn.height) {
            println!("[baker] Generating UV Atlas Projection {} -> {} ({}x{})", conn.from, conn.to, w, h);
            bake::atlas_map::bake_atlas_connection(
                &out_dir, &conn.from, &conn.to, &src_shard.packed_positions, src_shard.bounds_um, (w, h), dst_ghost_offset, 42,
            )
        } else {
            let ghosts = bake::ghost_map::build_ghost_mapping(&conn.from, &conn.to, &src_shard.soma_to_axon_map, dst_ghost_offset);
            bake::ghost_map::write_ghosts_file(&out_dir, &conn.from, &conn.to, &ghosts);
            ghosts.header.connection_count
        };
        println!("[baker] ✓ Ghost link {} -> {}: {} axons established.", conn.from, conn.to, sent_ghosts);
    }
    Ok(())
}

pub struct BakeWorkspace {
    pub sim: axicor_core::config::SimulationConfig,
    pub const_mem: axicor_core::config::blueprints::GenesisConstantMemory,
    pub neuron_types: Vec<axicor_core::config::blueprints::NeuronType>,
    pub name_map: std::collections::HashMap<String, u8>,
    pub anatomy: axicor_core::config::anatomy::AnatomyConfig,
    pub io: axicor_core::config::io::IoConfig,
    pub shard_cfg: axicor_core::config::InstanceConfig,
    pub out_dir: PathBuf,
    pub zone_name: String,
    pub zone_idx: u16,
    pub ghost_capacity: usize,
    pub master_seed: u64,
    pub sim_path: PathBuf,
    pub bp_path: PathBuf,
    pub an_path: PathBuf,
    pub io_path: PathBuf,
    pub shard_cfg_path: PathBuf,
    pub brain_config: axicor_core::config::brain::BrainConfig,
}

fn parse_and_validate(brain_config: &axicor_core::config::brain::BrainConfig, sim_path: &Path, bp_path: &Path, an_path: &Path, io_path: &Path, shard_cfg_path: &Path, out_dir: &Path, zone_name: &str, zone_idx: u16) -> Result<BakeWorkspace> {
    println!("[baker] Parsing configs for zone {}...", zone_name);
    let sim = parser::simulation::parse(&std::fs::read_to_string(sim_path)?)?;
    let (const_mem, neuron_types, name_map) = parser::blueprints::parse_blueprints(&std::fs::read_to_string(bp_path)?);
    let anatomy = parser::anatomy::parse(&std::fs::read_to_string(an_path)?)?;
    let io = parser::io::parse(&std::fs::read_to_string(io_path)?)?;

    validator::validate_all(&sim, &const_mem, &anatomy)?;
    let master_seed = bake::seed::seed_from_str(&sim.simulation.master_seed);
    let shard_cfg = axicor_core::config::InstanceConfig::load(shard_cfg_path).map_err(anyhow::Error::msg)?;
    let ghost_capacity = shard_cfg.settings.ghost_capacity as usize;

    Ok(BakeWorkspace {
        sim, const_mem, neuron_types, name_map, anatomy, io, shard_cfg, out_dir: out_dir.to_path_buf(),
        zone_name: zone_name.to_string(), zone_idx, ghost_capacity, master_seed,
        sim_path: sim_path.to_path_buf(), bp_path: bp_path.to_path_buf(), an_path: an_path.to_path_buf(),
        io_path: io_path.to_path_buf(), shard_cfg_path: shard_cfg_path.to_path_buf(), brain_config: brain_config.clone(),
    })
}

fn build_local_topology(workspace: &BakeWorkspace) -> (bake::layout::ShardSoA, bake::layout::CompiledShard, u32, usize, Vec<crate::bake::input_map::BakedGxi>, Vec<crate::bake::output_map::BakedGxo>) {
    bake::topology::build_local_topology_internal(
        &workspace.sim, &workspace.anatomy, &workspace.shard_cfg, &workspace.neuron_types,
        &workspace.const_mem, &workspace.name_map, &workspace.io, &workspace.zone_name,
        workspace.master_seed, workspace.ghost_capacity,
    )
}

fn serialize_artifacts(shard: &bake::layout::ShardSoA, workspace: &BakeWorkspace, num_virtual: usize, v_seg: u32, gxis: &[crate::bake::input_map::BakedGxi], gxos: &[crate::bake::output_map::BakedGxo]) -> Result<()> {
    std::fs::create_dir_all(&workspace.out_dir)?;
    shard.dump_to_disk(&workspace.out_dir);

    if !gxis.is_empty() { crate::bake::input_map::write_gxi_file(&workspace.out_dir, gxis); }
    if !gxos.is_empty() { crate::bake::output_map::write_gxo_file(&workspace.out_dir, gxos); }

    let zone_hash_fnv = axicor_core::hash::fnv1a_32(workspace.zone_name.as_bytes());
    let manifest = axicor_core::config::manifest::ZoneManifest {
        magic: 0x47454E45, depart_id_v1: workspace.brain_config.depart_id_v1.clone(),
        zone_hash: zone_hash_fnv, blueprints_path: workspace.bp_path.to_string_lossy().into_owned(),
        simulation: Some(axicor_core::config::brain::SimulationConfigRef { config: PathBuf::from("BrainDNA/simulation.toml") }),
        memory: axicor_core::config::manifest::ManifestMemory { padded_n: shard.soma_to_axon.len(), virtual_axons: num_virtual, ghost_capacity: workspace.ghost_capacity, v_seg: v_seg as u16 },
        network: axicor_core::config::manifest::ManifestNetwork { slow_path_tcp: 8010 + workspace.zone_idx * 10, external_udp_in: 8081 + workspace.zone_idx * 10, external_udp_out: 8082 + workspace.zone_idx * 10, external_udp_out_target: Some("127.0.0.1:8092".to_string()), fast_path_udp_local: 9001 + workspace.zone_idx * 10, fast_path_peers: workspace.brain_config.zones.iter().enumerate().filter(|(i, _)| *i != workspace.zone_idx as usize).map(|(i, z)| (z.name.clone(), format!("127.0.0.1:{}", 9001 + i * 10))).collect() },
        settings: axicor_core::config::manifest::ManifestSettings { night_interval_ticks: workspace.shard_cfg.settings.night_interval_ticks, save_checkpoints_interval_ticks: workspace.shard_cfg.settings.save_checkpoints_interval_ticks as u64, plasticity: axicor_core::config::manifest::ManifestPlasticity { prune_threshold: workspace.shard_cfg.settings.prune_threshold, max_sprouts: workspace.shard_cfg.settings.max_sprouts } },
        connections: workspace.brain_config.connections.iter().filter(|c| c.from == workspace.zone_name || c.to == workspace.zone_name).map(|c| axicor_core::config::manifest::ManifestConnection { conn_id_v1: c.conn_id_v1.clone(), from: c.from.clone(), to: c.to.clone(), width: c.width, height: c.height }).collect(),
        variants: workspace.const_mem.variants.iter().enumerate().map(|(i, v)| axicor_core::config::manifest::ManifestVariant { id: i as u8, name: format!("Variant_{}", i), threshold: v.threshold, rest_potential: v.rest_potential, leak_rate: v.leak_rate, homeostasis_penalty: v.homeostasis_penalty, spontaneous_firing_period_ticks: v.spontaneous_firing_period_ticks, initial_synapse_weight: v.initial_synapse_weight, gsop_potentiation: v.gsop_potentiation, gsop_depression: v.gsop_depression, homeostasis_decay: v.homeostasis_decay, refractory_period: v.refractory_period, synapse_refractory_period: v.synapse_refractory_period, signal_propagation_length: v.signal_propagation_length, is_inhibitory: v.is_inhibitory != 0, inertia_curve: { let mut curve = [0u8; 16]; for (i, &val) in v.inertia_curve.iter().enumerate().take(16) { curve[i] = val; } curve }, adaptive_leak_max: v.adaptive_leak_max, adaptive_leak_gain: v.adaptive_leak_gain, adaptive_mode: v.adaptive_mode, d1_affinity: v.d1_affinity, d2_affinity: v.d2_affinity }).collect(),
    };
    
    let manifest_toml = toml::to_string(&manifest).expect("Failed to serialize manifest");
    std::fs::write(workspace.out_dir.join("manifest.toml"), manifest_toml)?;

    let dna_dir = workspace.out_dir.join("BrainDNA");
    std::fs::create_dir_all(&dna_dir)?;
    let configs = [ (&workspace.sim_path, "simulation.toml"), (&workspace.bp_path, "blueprints.toml"), (&workspace.an_path, "anatomy.toml"), (&workspace.io_path, "io.toml"), (&workspace.shard_cfg_path, "shard.toml") ];
    for (src, dst_name) in configs {
        if src.exists() {
            if dst_name == "simulation.toml" {
                if let Ok(content) = std::fs::read_to_string(src) {
                    let mutated = content.replace("[sim_v_1]", "[manifest_sim_v_1]").replace("[simulation]", "[manifest_sim_v_1]");
                    let _ = std::fs::write(dna_dir.join(dst_name), mutated);
                }
            } else { std::fs::copy(src, dna_dir.join(dst_name))?; }
        }
    }
    Ok(())
}
