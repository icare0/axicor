use bevy_egui::egui::{self, Color32};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, EditorLevel};

pub fn draw_department_crumb(
    ui: &mut egui::Ui,
    graph: &BrainTopologyGraph,
    state: &NodeGraphUiState,
    mut send_open: impl FnMut(std::path::PathBuf),
) {
    let Some(active_proj) = graph.active_project.as_ref() else { return };
    
    let dept_label = if let Some(path) = &graph.active_path {
        let path_str = path.to_string_lossy();
        if path_str.contains("simulation.toml") {
            "Select Dept".to_string()
        } else {
            let mut p = path.clone();
            // Если мы внутри шарда (shard.toml, io.toml и т.д.)
            if path_str.contains("shard.toml") || path_str.contains("blueprints.toml") || path_str.contains("io.toml") || path_str.contains("anatomy.toml") {
                p.pop(); // к папке Shard_M
                p.pop(); // к папке Zone_N
                p.file_name().map_or("brain".to_string(), |n| n.to_string_lossy().into_owned())
            } else {
                // Если мы в файле самого департамента (Zone_N.toml)
                path.file_name().map_or("brain".to_string(), |n| n.to_string_lossy().replace(".toml", ""))
            }
        }
    } else {
        "Select Dept".to_string()
    };

    let is_active = state.level == EditorLevel::Department;
    let color = if is_active { Color32::WHITE } else { Color32::GRAY };

    if ui.button(egui::RichText::new(&dept_label).strong().color(color)).clicked() && dept_label != "Select Dept" {
        let path = std::path::PathBuf::from("Genesis-Models").join(&active_proj).join(format!("{}.toml", dept_label));
        send_open(path);
    }
}
