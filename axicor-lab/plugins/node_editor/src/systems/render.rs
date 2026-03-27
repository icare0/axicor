use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::AllocatedPanes;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation, SaveProjectEvent, BakeProjectEvent};
use crate::ui::render_editor_ui;

pub fn render_node_editor_system(
    mut contexts: EguiContexts,
    allocated: Res<AllocatedPanes>,
    graph: Res<BrainTopologyGraph>,
    mut ui_state: ResMut<NodeGraphUiState>,
    mut mut_events: EventWriter<TopologyMutation>,
    mut save_events: EventWriter<SaveProjectEvent>,
    mut bake_events: EventWriter<BakeProjectEvent>,
) {
    let Some(rect) = allocated.rects.get("axicor.node_editor") else { return; };
    let Some(ctx) = contexts.try_ctx_mut() else { return; };

    egui::Area::new("NodeEditorPortal".into())
        .fixed_pos(rect.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_clip_rect(*rect);
            ui.allocate_ui_at_rect(*rect, |ui| {
                render_editor_ui(
                    ui,
                    *rect,
                    &graph,
                    &mut ui_state,
                    |ev| { mut_events.send(ev); },
                    || { save_events.send(SaveProjectEvent); },
                    || { bake_events.send(BakeProjectEvent); },
                );
            });
        });
}
