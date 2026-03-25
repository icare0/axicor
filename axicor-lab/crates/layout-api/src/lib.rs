use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{TileId, LinearDir, Tree};
use std::collections::HashMap;

// --- Components ---

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PluginDomain {
    #[default]
    Viewport3D,
    ProjectExplorer,
    NodeEditor,
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PluginInput {
    pub local_cursor: Vec2,
    pub cursor_delta: Vec2,
    pub scroll_delta: f32,
    pub is_primary_pressed: bool,
    pub is_secondary_pressed: bool, // For rotation (RMB)
    pub is_middle_pressed: bool,    // For panning (MMB)
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ViewportCamera {
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, // Rotation around Y
    pub beta: f32,  // Rotation up/down
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 40.0, // Дистанция, охватывающая весь шард
            alpha: std::f32::consts::PI / 4.0, // Изометрия по умолчанию
            beta: 0.5,
        }
    }
}

#[derive(Component, Debug, Default, Clone)]
pub struct PluginGeometry {
    pub size: Vec2,
}

#[derive(Component)]
pub struct PluginWindow {
    pub domain: PluginDomain,
    pub texture: Option<Handle<Image>>, 
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectStatus {
    #[default]
    Unknown,
    Ready,
    NeedsBake,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectModel {
    pub name: String,
    pub status: ProjectStatus,
    pub dna_files: Vec<String>,
    pub shards: Vec<String>,
}

#[derive(Resource, Default)]
pub struct WindowDragRequest {
    pub should_drag: bool,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ProjectFsCache {
    pub projects: Vec<ProjectModel>,
}

// --- Events ---

#[derive(Event, Clone)]
pub struct ZoneSelectedEvent {
    pub project_name: String,
    pub shard_name: String,
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
    Split { axis: LinearDir, fraction: f32, insert_before: bool, domain: PluginDomain },
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
        domain: PluginDomain,
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
    pub domain: PluginDomain,
    pub texture_id: Option<egui::TextureId>,
}

#[derive(Event, Clone)]
pub struct LoadGraphEvent {
    pub project_name: String,
}
