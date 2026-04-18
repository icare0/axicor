use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

// ============================================================================
// 0. layout_api constants

// ============================================================================
// 1. Interactions (Intents & Events)
// ============================================================================

#[derive(Event, Debug, Clone)]
pub struct SaveDefaultLayoutEvent;

// DOD FIX: Native OS window control (WM -> OS)
#[derive(Event, Debug, Clone)]
pub enum OsWindowCommand {
    Drag,
    Minimize,
    Maximize,
}

// ============================================================================
// 2. Topology
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pane {
    pub plugin_id: String,
    pub title: String,
}

#[derive(Resource)]
pub struct WorkspaceState {
    pub active_workspace: String,
    pub workspace_order: Vec<String>,
    pub trees: bevy::utils::HashMap<String, egui_tiles::Tree<Pane>>,
    pub renaming_workspace: Option<String>,
    pub rename_buffer: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SavedLayout {
    pub active_workspace: String,
    pub workspace_order: Vec<String>,
    pub trees: bevy::utils::HashMap<String, egui_tiles::Tree<Pane>>,
}

// 3. State Machine for Drag-and-Drop
#[derive(Resource, Default)]
pub struct WindowDragState {
    pub is_dragging: bool,
    pub source_tile: Option<egui_tiles::TileId>,
    pub drop_zone: Option<egui::Rect>,
    pub intent: layout_api::DragIntent, // Keep using layout-api types for events if needed, or move them here
    pub start_pos: Option<egui::Pos2>,
    pub drag_axis: Option<egui_tiles::LinearDir>,
    pub drag_normal: Option<f32>,
}

/// Command queue for egui_tiles (handled by systems::window)
#[derive(Resource, Default)]
pub struct TreeCommands {
    pub queue: Vec<layout_api::TreeCommand>,
}
