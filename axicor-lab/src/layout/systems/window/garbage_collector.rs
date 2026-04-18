use crate::layout::domain::WorkspaceState;
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use egui_tiles::{SimplificationOptions, Tile};
use layout_api::TopologyCache;

const MIN_PANE_SIZE: f32 = 100.0;

pub fn window_garbage_collector_system(
    mut contexts: EguiContexts,
    mut workspace: ResMut<WorkspaceState>,
    topology: Res<TopologyCache>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else {
        return;
    };
    if !ctx.input(|i| i.pointer.any_released()) {
        return;
    }

    let active_ws = workspace.active_workspace.clone();
    let Some(tree) = workspace.trees.get_mut(&active_ws) else {
        return;
    };

    let pane_count = tree
        .tiles
        .iter()
        .filter(|(_, t)| matches!(t, Tile::Pane(_)))
        .count();

    if pane_count <= 1 {
        return;
    }

    let mut to_remove = Vec::with_capacity(topology.tiles.len());
    for (&id, r) in topology.tiles.iter() {
        if r.width() <= MIN_PANE_SIZE || r.height() <= MIN_PANE_SIZE {
            to_remove.push(id);
        }
    }

    if to_remove.is_empty() {
        return;
    }

    for id in to_remove {
        tree.tiles.remove(id);
    }
    tree.simplify(&SimplificationOptions {
        all_panes_must_have_tabs: false,
        ..default()
    });
}
