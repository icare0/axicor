use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, SaveProjectEvent, BakeProjectEvent, LoadGraphEvent};
use layout_api::ActiveBundle;

pub fn project_pipeline_system(
    mut save_events: EventReader<SaveProjectEvent>,
    mut bake_events: EventReader<BakeProjectEvent>,
    mut load_events: EventWriter<LoadGraphEvent>,
    graph: Res<BrainTopologyGraph>,
    bundle: Res<ActiveBundle>,
) {
    for _ in save_events.read() {
        let (Some(config), Some(proj_name)) = (&graph.config, &graph.active_project) else { continue; };
        let proj_dir = std::path::PathBuf::from("Genesis-Models").join(proj_name);

        if let Some(toc) = bundle.toc() {
            for path_str in toc.keys() {
                if path_str.starts_with("baked/") { continue; }
                if let Some(data) = bundle.get_file(path_str) {
                    let full_path = proj_dir.join(path_str);
                    if let Some(p) = full_path.parent() { let _ = std::fs::create_dir_all(p); }
                    let _ = std::fs::write(full_path, data);
                }
            }
        }

        std::fs::write(proj_dir.join("brain.toml"), toml::to_string(config).unwrap()).unwrap();

        for zone in &config.zones {
            let zone_dir = proj_dir.join(&zone.name);
            std::fs::create_dir_all(&zone_dir).unwrap();

            if let Some(io) = graph.io_configs.get(&zone.name) {
                let io_file = zone.io.file_name().unwrap_or_default();
                std::fs::write(zone_dir.join(io_file), toml::to_string(io).unwrap()).unwrap();
            } else {
                std::fs::write(zone_dir.join("io.toml"), "").unwrap();
            }

            if let Some(anatomy) = graph.anatomy_configs.get(&zone.name) {
                let anatomy_file = zone.anatomy.file_name().unwrap_or_default();
                std::fs::write(zone_dir.join(anatomy_file), toml::to_string(anatomy).unwrap()).unwrap();
            } else {
                let default_anatomy = "[[layer]]\nname = \"Main\"\nheight_pct = 1.0\ndensity = 0.1\ncomposition = { \"Default_Neuron\" = 1.0 }\n";
                std::fs::write(zone_dir.join("anatomy.toml"), default_anatomy).unwrap();
            }

            let bp_path = zone_dir.join("blueprints.toml");
            if !bp_path.exists() {
                let default_bp = r#"[[neuron_type]]
name = "Default_Neuron"
is_inhibitory = false
threshold = 30000
rest_potential = 10000
leak_rate = 1500
refractory_period = 15
synapse_refractory_period = 15
signal_propagation_length = 20
homeostasis_penalty = 10000
homeostasis_decay = 20
slot_decay_ltm = 96
slot_decay_wm = 190
ltm_slot_count = 103
initial_synapse_weight = 74
gsop_potentiation = 150
gsop_depression = 60
"#;
                std::fs::write(bp_path, default_bp).unwrap();
            }

            let shard_path = zone_dir.join("shard.toml");
            if !shard_path.exists() {
                let default_shard = format!(
                    "zone_id = \"{}\"\n\n[world_offset]\nx = 0\ny = 0\nz = 0\n\n[dimensions]\nw = 32\nd = 32\nh = 32\n\n[neighbors]\n\n[settings]\nnight_interval_ticks = 100000\nsave_checkpoints_interval_ticks = 250000\n",
                    zone.name
                );
                std::fs::write(shard_path, default_shard).unwrap();
            }
        }
        info!("[Pipeline] 💾 Project '{}' saved to source directory.", proj_name);
    }

    for _ in bake_events.read() {
        let Some(proj_name) = &graph.active_project else { continue; };
        info!("[Pipeline] 🔥 Spawning Genesis Baker for '{}'...", proj_name);

        let brain_path = format!("Genesis-Models/{}/brain.toml", proj_name);

        let status = std::process::Command::new("cargo")
            .arg("run").arg("--release").arg("-p").arg("genesis-baker")
            .arg("--bin").arg("baker").arg("--")
            .arg("--brain").arg(&brain_path)
            .arg("--clean")
            .arg("--yes")
            .status();

        match status {
            Ok(s) if s.success() => {
                info!("[Pipeline] ✅ Bake complete! .axic archive updated.");
                load_events.send(LoadGraphEvent { project_name: proj_name.clone() });
            },
            _ => error!("❌ [Pipeline] Bake failed! Check compiler logs."),
        }
    }
}
