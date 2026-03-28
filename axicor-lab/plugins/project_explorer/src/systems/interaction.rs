use bevy::prelude::*;
use std::fs;
use std::path::PathBuf;
use layout_api::CreateNewModelEvent;

pub fn create_new_model_system(
    mut events: EventReader<CreateNewModelEvent>,
) {
    for ev in events.read() {
        let base_dir = PathBuf::from("Genesis-Models").join(&ev.model_name);
        
        // 1. Создаем папку проекта
        if let Err(e) = fs::create_dir_all(&base_dir) {
            eprintln!("Failed to create model directory: {}", e);
            continue;
        }

        // 2. Генерируем минимальный brain.toml (Пустой граф)
        let brain_toml = r#"[simulation]
config = "simulation.toml"

# Zones and connections will be added via Node Editor
"#;
        if let Err(e) = fs::write(base_dir.join("brain.toml"), brain_toml) {
            eprintln!("Failed to write brain.toml: {}", e);
        }

        // 3. Генерируем базовый simulation.toml (Законы физики)
        let sim_toml = r#"[world]
width_um = 1000
depth_um = 1000
height_um = 1000

[simulation]
tick_duration_us = 100
total_ticks = 0
master_seed = "GENESIS"
voxel_size_um = 25.0
signal_speed_m_s = 0.5
sync_batch_ticks = 20
segment_length_voxels = 2
axon_growth_max_steps = 500
"#;
        if let Err(e) = fs::write(base_dir.join("simulation.toml"), sim_toml) {
            eprintln!("Failed to write simulation.toml: {}", e);
        }

        info!("✅ New model created at {:?}", base_dir);
    }
}

pub fn sync_smart_focus_system(
    mut events: EventReader<layout_api::OpenFileEvent>,
    mut query: Query<&mut crate::domain::ProjectExplorerState>,
) {
    for ev in events.read() {
        // Когда любой плагин (включая Node Editor) открывает файл, Эксплорер берет его в фокус
        for mut state in query.iter_mut() {
            state.active_file = Some(ev.path.clone());
        }
    }
}
