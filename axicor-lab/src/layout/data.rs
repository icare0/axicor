use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{TileId, LinearDir, Tree};
use std::collections::HashMap;

// --- Components ---

#[derive(Component, Debug, Default, Clone)]
pub struct PluginInput {
    pub local_cursor: Vec2,
    pub is_pressed: bool,
}

#[derive(Component, Debug, Clone)]
pub struct PluginGeometry {
    pub size: Vec2,
}

#[derive(Component)]
pub struct PluginWindow {
    pub texture: Handle<Image>,
}

// --- Resources & Enums ---

#[derive(Resource)]
pub struct WorkspaceTree {
    pub tree: Tree<Entity>,
}

impl Default for WorkspaceTree {
    fn default() -> Self {
        Self {
            tree: Tree::empty("workspace"),
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum DragIntent {
    #[default]
    None,
    Split { axis: LinearDir, fraction: f32, insert_before: bool },
    Merge { victim: TileId },
}

#[derive(Resource, Default)]
pub struct WindowDragState {
    pub is_dragging: bool,
    pub source_tile: Option<TileId>,
    pub start_pos: Option<egui::Pos2>,
    pub drag_axis: Option<LinearDir>,
    pub drag_normal: Option<f32>,
    pub intent: DragIntent,
}

#[derive(Resource, Default)]
pub struct TopologyCache {
    pub rects: HashMap<TileId, egui::Rect>,
}

pub enum TreeCommand {
    Split { 
        target: TileId, 
        axis: LinearDir,
        fraction: f32,
        insert_before: bool,
    },
    Merge {
        survivor: TileId,
        victim: TileId,
    },
}

#[derive(Resource, Default)]
pub struct TreeCommands {
    pub queue: Vec<TreeCommand>,
}

pub struct PaneData {
    pub texture_id: egui::TextureId,
}
