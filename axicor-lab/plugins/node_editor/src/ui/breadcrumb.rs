// ui/breadcrumb.rs
use bevy_egui::egui::{self, Color32};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState};

pub fn draw_breadcrumbs(
    ui: &mut egui::Ui,
    graph: &BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    send_open: &mut impl FnMut(std::path::PathBuf),
) {
    let is_lvl_0 = state.level == crate::domain::EditorLevel::Model;
    let is_lvl_1 = state.level == crate::domain::EditorLevel::Department;
    
    let c_model = if is_lvl_0 { Color32::WHITE } else { Color32::GRAY };
    let c_dept  = if is_lvl_1 { Color32::WHITE } else { Color32::GRAY };
    let c_zone  = if !is_lvl_0 && !is_lvl_1 { Color32::WHITE } else { Color32::GRAY };

    let model_name = graph.project_name.clone().unwrap_or_else(|| "Select Model".to_string());
    
    // --- КРОШКА 1: MODELS ---
    if let Some(new_m) = draw_searchable_breadcrumb(ui, &model_name, &mut state.model_search, c_model, || {
        let mut models = Vec::new();
        if let Ok(entries) = std::fs::read_dir("Genesis-Models") {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().replace(".axic", "").replace(" (Source)", "");
                models.push(name);
            }
        }
        models.sort(); models.dedup();
        (models, vec![])
    }) {
        send_open(std::path::PathBuf::from("Genesis-Models").join(&new_m).join("simulation.toml"));
    }

    ui.label(egui::RichText::new("›").color(Color32::DARK_GRAY));

    // --- КРОШКА 2: DEPARTMENTS (Fixed Logic) ---
    let dept_name = if is_lvl_0 { "Select Dept".to_string() } else { "brain".to_string() };
    if let Some(new_d) = draw_searchable_breadcrumb(ui, &dept_name, &mut state.dept_search, c_dept, || {
        let mut local = Vec::new();
        let mut global = Vec::new();
        if let Ok(models) = std::fs::read_dir("Genesis-Models") {
            for m in models.flatten() {
                let m_name = m.file_name().to_string_lossy().replace(".axic", "").replace(" (Source)", "");
                if let Ok(entries) = std::fs::read_dir(m.path()) {
                    for e in entries.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.ends_with(".toml") && name != "simulation.toml" && name != "manifest.toml" {
                            let d = name.replace(".toml", "");
                            if m_name == model_name { local.push(d); }
                            else { global.push(format!("{}/{}", m_name, d)); }
                        }
                    }
                }
            }
        }
        (local, global)
    }) {
        let parts: Vec<&str> = new_d.split('/').collect();
        let path = if parts.len() == 2 {
            std::path::PathBuf::from("Genesis-Models").join(parts[0]).join(format!("{}.toml", parts[1]))
        } else {
            std::path::PathBuf::from("Genesis-Models").join(&model_name).join(format!("{}.toml", new_d))
        };
        send_open(path);
    }

    ui.label(egui::RichText::new("›").color(Color32::DARK_GRAY));

    // --- КРОШКА 3: ZONES (Restored) ---
    let zone_name = if is_lvl_0 || is_lvl_1 { "Select Zone".to_string() } else { "Zone".to_string() };
    if let Some(new_z) = draw_searchable_breadcrumb(ui, &zone_name, &mut state.zone_search, c_zone, || {
        let mut local = Vec::new();
        let global = Vec::new();
        
        // Сканируем текущий департамент (brain) для получения списка зон
        let dept_path = std::path::PathBuf::from("Genesis-Models").join(&model_name).join("brain");
        if let Ok(entries) = std::fs::read_dir(dept_path) {
            for e in entries.flatten() {
                if e.path().is_dir() {
                    local.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        (local, global)
    }) {
        let path = std::path::PathBuf::from("Genesis-Models").join(&model_name).join("brain").join(&new_z).join("shard.toml");
        send_open(path);
    }
}

pub fn draw_searchable_breadcrumb(
    ui: &mut egui::Ui,
    name: &str,
    search: &mut String,
    color: Color32,
    fetch: impl FnOnce() -> (Vec<String>, Vec<String>),
) -> Option<String> {
    let mut selected = None;
    ui.menu_button(egui::RichText::new(name).strong().color(color), |ui| {
        let resp = ui.text_edit_singleline(search);
        resp.request_focus();
        ui.separator();
        let (local, global) = fetch();
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for item in local {
                if item.to_lowercase().contains(&search.to_lowercase()) {
                    if ui.button(egui::RichText::new(&item).color(Color32::LIGHT_BLUE)).clicked() {
                        selected = Some(item); ui.close_menu();
                    }
                }
            }
            if !global.is_empty() {
                ui.separator();
                for item in global {
                    if item.to_lowercase().contains(&search.to_lowercase()) {
                        if ui.button(&item).clicked() { selected = Some(item); ui.close_menu(); }
                    }
                }
            }
        });
    });
    selected
}
