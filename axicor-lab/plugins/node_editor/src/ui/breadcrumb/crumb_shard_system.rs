use bevy_egui::egui::{self, Color32};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, EditorLevel};

pub fn draw_shard_crumb(
    ui: &mut egui::Ui,
    graph: &BrainTopologyGraph,
    state: &NodeGraphUiState,
    mut send_open: impl FnMut(std::path::PathBuf),
) {
    let Some(active_proj) = graph.active_project.as_ref() else { return };
    
    // Определяем имя Шарда
    let shard_label = match &state.level {
        EditorLevel::Zone(name) => name.clone(),
        _ => {
            if let Some(path) = &graph.active_path {
                let path_str = path.to_string_lossy();
                if path_str.contains("shard.toml") || path_str.contains("blueprints.toml") || path_str.contains("io.toml") || path_str.contains("anatomy.toml") {
                    let mut p = path.clone();
                    if path_str.contains(".toml") { p.pop(); } // к папке Shard_M
                    p.file_name().map_or("Select Shard".to_string(), |n| n.to_string_lossy().into_owned())
                } else { "Select Shard".to_string() }
            } else { "Select Shard".to_string() }
        }
    };

    // Определяем имя Департамента (для построения пути)
    let dept_name = if let Some(path) = &graph.active_path {
        let path_str = path.to_string_lossy();
        let mut p = path.clone();
        if path_str.contains("shard.toml") || path_str.contains("blueprints.toml") || path_str.contains("io.toml") || path_str.contains("anatomy.toml") {
            p.pop(); // к папке Shard_M
            p.pop(); // к папке Zone_N
            p.file_name().map_or("brain".to_string(), |n| n.to_string_lossy().into_owned())
        } else {
            path.file_name().map_or("brain".to_string(), |n| n.to_string_lossy().replace(".toml", ""))
        }
    } else {
        "brain".to_string()
    };

    let is_active = matches!(state.level, EditorLevel::Zone(_));
    let color = if is_active { Color32::WHITE } else { Color32::GRAY };

    if ui.button(egui::RichText::new(&shard_label).strong().color(color)).clicked() && shard_label != "Select Shard" {
        let path = std::path::PathBuf::from("Genesis-Models")
            .join(&active_proj)
            .join(&dept_name)
            .join(&shard_label)
            .join("shard.toml");
        send_open(path);
    }
}
