use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::{PluginWindow, base_domain, DOMAIN_NODE_ED};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation, SaveProjectEvent, BakeProjectEvent};
use crate::ui::render_editor_ui;

pub fn render_node_editor_system(
    mut contexts: EguiContexts,
    window_query: Query<(&PluginWindow, Entity)>,
    mut ui_states: Query<&mut NodeGraphUiState>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut mut_events: EventWriter<TopologyMutation>,
    mut save_events: EventWriter<SaveProjectEvent>,
    mut bake_events: EventWriter<BakeProjectEvent>,
    mut open_file_events: ParamSet<(
        EventReader<layout_api::OpenFileEvent>,
        EventWriter<layout_api::OpenFileEvent>,
    )>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    let mut new_level = None;
    let mut opened_model_path = None;

    for ev in open_file_events.p0().read() {
        if let Some(file_name) = ev.path.file_name().and_then(|n| n.to_str()) {
            if file_name == "simulation.toml" {
                new_level = Some(crate::domain::EditorLevel::Model);
                opened_model_path = Some(ev.path.clone());
            } else if file_name.ends_with(".toml") && file_name != "simulation.toml" && file_name != "manifest.toml" {
                // Если открыли любой другой TOML (например, visual_cortex.toml) - это уровень департамента
                new_level = Some(crate::domain::EditorLevel::Department);
            }
        }
    }

    for (window, entity) in window_query.iter() {
        if !window.is_visible { continue; }
        if base_domain(&window.plugin_id) != DOMAIN_NODE_ED { continue; }
        let Ok(mut ui_state) = ui_states.get_mut(entity) else { continue };

        if let Some(lvl) = &new_level {
            ui_state.level = lvl.clone();
            
            if *lvl == crate::domain::EditorLevel::Model {
                graph.zones.clear();
                graph.connections.clear(); // Связи пока очищаем, роутинг сделаем позже
                
                if let Some(path) = &opened_model_path {
                    if let Some(proj_dir) = path.parent() {
                        let proj_name = proj_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
                        graph.project_name = Some(proj_name);

                        // DOD FIX: O(1) Сканирование директории на лету. 
                        // Все *.toml файлы (кроме системных) становятся нодами-департаментами!
                        if let Ok(entries) = std::fs::read_dir(proj_dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.is_file() {
                                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                        if name.ends_with(".toml") && name != "simulation.toml" && name != "manifest.toml" {
                                            // Добавляем департамент (например, "vision")
                                            graph.zones.push(name.replace(".toml", ""));
                                        }
                                    }
                                }
                            }
                        }
                        graph.zones.sort(); // Стабильный рендер
                    }
                }
            }
        }

        egui::Area::new(egui::Id::new(&window.plugin_id))
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                ui.set_min_size(window.rect.size());

                render_editor_ui(
                    ui,
                    window.rect,
                    &mut graph,
                    &mut ui_state,
                    |ev| { mut_events.send(ev); },
                    ||   { save_events.send(SaveProjectEvent); },
                    ||   { bake_events.send(BakeProjectEvent); },
                    |path| { open_file_events.p1().send(layout_api::OpenFileEvent { path }); },
                );
            });
    }
}