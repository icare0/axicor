use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use egui_tiles::{LinearDir, Tile};

use crate::layout::domain::{WorkspaceState, WindowDragState, TreeCommands};
use layout_api::{DragIntent, TreeCommand, WindowDragRequest, DragSource, TopologyCache};

// --- Пороги ---
const AXIS_LOCK_DIST_SQ: f32 = 100.0;  // 10px
const SPLIT_THRESHOLD:   f32 = 100.0;
const MERGE_THRESHOLD:   f32 = 50.0;
const SPLIT_MIN_SIZE:    f32 = 200.0;
const MERGE_HIT_EXPAND:  f32 = 10.0;
const ALIGN_EPS:         f32 = 10.0;
const FALLBACK_PLUGIN:   &str = "axicor.viewport_3d";

pub fn evaluate_drag_intents_system(
    mut contexts: EguiContexts,
    mut drag_state: ResMut<WindowDragState>,
    drag_request: Res<WindowDragRequest>,
    topology: Res<TopologyCache>,
    mut commands_queue: ResMut<TreeCommands>,
    workspace: Option<Res<WorkspaceState>>,
) {
    // --- Завершение драга ---
    if !drag_request.active {
        if drag_state.is_dragging {
            flush_drag_command(&drag_state, &mut commands_queue);
        }
        *drag_state = WindowDragState::default();
        return;
    }

    // --- Начало драга ---
    if !drag_state.is_dragging {
        let Some(src_tile) = drag_request.target_tile else { return };
        drag_state.is_dragging = true;
        drag_state.source_tile = Some(src_tile);
        drag_state.start_pos   = Some(drag_request.start_pos);
        return;
    }

    // --- Активный драг ---
    let Some(workspace) = workspace else { return };
    let ctx = contexts.ctx_mut();

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        *drag_state = WindowDragState::default();
        return;
    }

    let (Some(src_tile), Some(start_pos)) = (drag_state.source_tile, drag_state.start_pos) else { return };
    let Some(&src_rect) = topology.tiles.get(&src_tile) else { return };

    let current_pos = drag_request.current_pos;
    let delta = current_pos - start_pos;

    // Фиксация оси после преодоления мёртвой зоны
    if drag_state.drag_axis.is_none() && delta.length_sq() > AXIS_LOCK_DIST_SQ {
        let horizontal = delta.x.abs() > delta.y.abs();
        drag_state.drag_axis   = Some(if horizontal { LinearDir::Horizontal } else { LinearDir::Vertical });
        drag_state.drag_normal = Some(if horizontal { delta.x.signum() } else { delta.y.signum() });
    }

    let (Some(axis), Some(normal)) = (drag_state.drag_axis, drag_state.drag_normal) else { return };

    drag_state.intent = match drag_request.source {
        DragSource::Header => {
            compute_swap_intent(current_pos, src_tile, &topology)
        }
        DragSource::EdgeTrigger => {
            if src_rect.contains(current_pos) {
                compute_split_intent(delta, axis, normal, current_pos, src_rect, src_tile, &workspace)
            } else {
                compute_merge_intent(delta, axis, current_pos, src_rect, src_tile, &topology)
            }
        }
    };
}

// ---------------------------------------------------------------------------

fn flush_drag_command(drag_state: &WindowDragState, queue: &mut TreeCommands) {
    if matches!(drag_state.intent, DragIntent::None) { return; }
    let src = match drag_state.source_tile {
        Some(t) => t,
        None => return,
    };
    let cmd = match &drag_state.intent {
        DragIntent::Split { axis, fraction, insert_before, plugin_id } =>
            TreeCommand::Split { target: src, axis: *axis, fraction: *fraction,
                                 insert_before: *insert_before, plugin_id: plugin_id.clone() },
        DragIntent::Merge { victim } =>
            TreeCommand::Merge { survivor: src, victim: *victim },
        DragIntent::Swap { victim } =>
            TreeCommand::SwapPanes { src, dst: *victim },
        _ => return,
    };
    queue.queue.push(cmd);
}

fn axis_component(v: egui::Vec2, axis: LinearDir) -> f32 {
    if axis == LinearDir::Horizontal { v.x } else { v.y }
}

fn axis_size(rect: egui::Rect, axis: LinearDir) -> f32 {
    if axis == LinearDir::Horizontal { rect.width() } else { rect.height() }
}

fn compute_split_intent(
    delta: egui::Vec2,
    axis: LinearDir,
    normal: f32,
    current_pos: egui::Pos2,
    src_rect: egui::Rect,
    src_tile: egui_tiles::TileId,
    workspace: &WorkspaceState,
) -> DragIntent {
    let size = axis_size(src_rect, axis);
    if axis_component(delta, axis).abs() < SPLIT_THRESHOLD || size <= SPLIT_MIN_SIZE {
        return DragIntent::None;
    }

    let raw_fraction = if axis == LinearDir::Horizontal {
        (current_pos.x - src_rect.min.x) / src_rect.width()
    } else {
        (current_pos.y - src_rect.min.y) / src_rect.height()
    };
    let min_f = SPLIT_THRESHOLD / size;
    let fraction = raw_fraction.clamp(min_f, 1.0 - min_f);

    let plugin_id = workspace.tree.tiles.get(src_tile)
        .and_then(|t| if let Tile::Pane(p) = t { Some(p.plugin_id.clone()) } else { None })
        .unwrap_or_else(|| FALLBACK_PLUGIN.to_string());

    DragIntent::Split { axis, fraction, insert_before: normal > 0.0, plugin_id }
}

fn compute_merge_intent(
    delta: egui::Vec2,
    axis: LinearDir,
    current_pos: egui::Pos2,
    src_rect: egui::Rect,
    src_tile: egui_tiles::TileId,
    topology: &TopologyCache,
) -> DragIntent {
    if axis_component(delta, axis).abs() < MERGE_THRESHOLD {
        return DragIntent::None;
    }

    let victim = topology.tiles.iter().find(|(&id, r)| {
        if id == src_tile { return false; }
        if !r.expand(MERGE_HIT_EXPAND).contains(current_pos) { return false; }
        // Жертва должна быть выровнена по перпендикулярной оси
        if axis == LinearDir::Horizontal {
            (src_rect.min.y - r.min.y).abs() < ALIGN_EPS &&
            (src_rect.max.y - r.max.y).abs() < ALIGN_EPS
        } else {
            (src_rect.min.x - r.min.x).abs() < ALIGN_EPS &&
            (src_rect.max.x - r.max.x).abs() < ALIGN_EPS
        }
    });

    match victim {
        Some((&victim_id, _)) => DragIntent::Merge { victim: victim_id },
        None => DragIntent::None,
    }
}

fn compute_swap_intent(
    current_pos: egui::Pos2,
    src_tile: egui_tiles::TileId,
    topology: &TopologyCache,
) -> DragIntent {
    let victim = topology.tiles.iter().find(|(&id, r)| {
        id != src_tile && r.contains(current_pos)
    });
    match victim {
        Some((&victim_id, _)) => DragIntent::Swap { victim: victim_id },
        None => DragIntent::None,
    }
}