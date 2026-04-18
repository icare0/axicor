use bevy::prelude::*;
use std::fs;
use std::path::PathBuf;
use layout_api::CreateNewModelEvent;
use axicor_core::config::sys::SystemMeta;

/// -:    
pub fn create_model_system(
    mut events: EventReader<CreateNewModelEvent>,
) {
    for ev in events.read() {
        let base_dir = PathBuf::from("Axicor-Models").join(&ev.model_name);
        let sim_path = base_dir.join("simulation.toml");

        if let Err(e) = fs::create_dir_all(&base_dir) {
            error!("[ERROR] [Orchestrator] Failed to create model directory: {}", e);
            continue;
        }

        let model_uuid = SystemMeta::generate().id;

        let sim_toml = format!(r#"[model_id_v1]
id = "{}"

[world]
width_um = 1000
depth_um = 1000
height_um = 1000

[sim_v_1]
tick_duration_us = 100
total_ticks = 0
master_seed = "AXICOR"
voxel_size_um = 25.0
signal_speed_m_s = 0.5
sync_batch_ticks = 20
segment_length_voxels = 2
axon_growth_max_steps = 500
"#, model_uuid);

        let _ = fs::write(&sim_path, sim_toml);
        info!(" [Orchestrator] Model {} initialized. Further nodes must be added manually.", ev.model_name);
    }
}
