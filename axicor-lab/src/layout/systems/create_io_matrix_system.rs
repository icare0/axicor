use bevy::prelude::*;
use node_editor::domain::TopologyMutation;
use std::path::Path;
use crate::layout::systems::wm_file_ops::{load_document, save_document, add_io_record};

pub fn create_io_matrix_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<node_editor::domain::BrainTopologyGraph>,
) {
    let Some(active_path) = graph.active_path.clone() else { return };

    for ev in events.read() {
        if let TopologyMutation::AddIoMatrix { zone, is_input, name } = ev {
            let path_str = active_path.to_string_lossy();
            let is_sim = path_str.contains("simulation.toml");
            let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
            let project_dir = active_path.parent().unwrap_or(Path::new("."));

            let io_path = if is_sim {
                project_dir.join(zone).join("io.toml")
            } else {
                project_dir.join(&dept_name).join(zone).join("io.toml")
            };

            if let Ok(mut doc) = load_document(&io_path) {
                let section = if *is_input { "input" } else { "output" };
                // Локальный Lineage ID (baker перезапишет при необходимости)
                let io_id = format!("{}_{}", zone, name);
                
                // Дефолтный сайз для новой матрицы 32x32
                add_io_record(&mut doc, section, name, &io_id, 32, 32);
                
                if let Err(e) = save_document(&io_path, &doc) {
                    error!("❌ [Orchestrator] Failed to write I/O AST: {}", e);
                } else {
                    info!("✅ [Orchestrator] I/O Matrix {} added to {:?}", name, io_path);
                }
            }
        }
    }
}
