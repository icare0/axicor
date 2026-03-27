use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

// ============================================================================
// §0. ГЛОБАЛЬНЫЕ КОНСТАНТЫ ПЕРЕНЕСЕНЫ В layout_api

// ============================================================================
// §1. ИНТЕНТЫ И СОБЫТИЯ (Intents & Events)
// ============================================================================

#[derive(Event, Debug, Clone)]
pub struct SaveDefaultLayoutEvent;

// DOD FIX: Комплексный интент для управления системным окном (WM -> OS)
#[derive(Event, Debug, Clone)]
pub enum OsWindowCommand {
    Drag,
    Minimize,
    Maximize,
}

// ============================================================================
// §2. ОСНОВНЫЕ СТРУКТУРЫ ОКОН (Topology)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pane {
    pub plugin_id: String,
    pub title: String,
}

#[derive(Resource)]
pub struct WorkspaceState {
    pub tree: egui_tiles::Tree<Pane>,
}


// §3. ПАМЯТЬ ОКОННОГО МЕНЕДЖЕРА ПЕРЕНЕСЕНА В layout_api

/// Внутреннее состояние State Machine для Drag-and-Drop
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

/// Очередь команд на мутацию дерева egui_tiles (разрешается в systems::window)
#[derive(Resource, Default)]
pub struct TreeCommands {
    pub queue: Vec<layout_api::TreeCommand>,
}
