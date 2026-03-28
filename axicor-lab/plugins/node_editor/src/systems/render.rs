use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::{PluginWindow, base_domain, DOMAIN_NODE_ED};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation, SaveProjectEvent, CompileGraphEvent, BakeProjectEvent};
use crate::ui::render_editor_ui;

pub fn render_node_editor_system(
    mut contexts: EguiContexts,
    window_query: Query<(&PluginWindow, Entity)>,
    mut ui_states: Query<&mut NodeGraphUiState>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut mut_events: EventWriter<TopologyMutation>,
    mut save_events: EventWriter<SaveProjectEvent>,
    mut compile_events: EventWriter<CompileGraphEvent>,
    mut bake_events: EventWriter<BakeProjectEvent>,
    mut open_file_events: EventWriter<layout_api::OpenFileEvent>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (window, entity) in window_query.iter() {
        if !window.is_visible { continue; }
        if base_domain(&window.plugin_id) != DOMAIN_NODE_ED { continue; }
        let Ok(mut ui_state) = ui_states.get_mut(entity) else { continue };

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
                    ||   { compile_events.send(CompileGraphEvent); },
                    ||   { bake_events.send(BakeProjectEvent); },
                    |path| { open_file_events.send(layout_api::OpenFileEvent { path }); },
                );
            });
    }
}