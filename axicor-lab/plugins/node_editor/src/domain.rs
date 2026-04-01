use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct NodeGraphUiState {
    pub pan: bevy_egui::egui::Vec2,
    pub zoom: f32,
    pub level: EditorLevel,
    pub node_positions: HashMap<String, bevy_egui::egui::Pos2>,
    pub selected_node_id: Option<String>, // Паспорт выбранной ноды
    pub dragging_pin: Option<(String, String, bevy_egui::egui::Pos2, bool)>, 
    pub pending_connection: Option<(String, String, String, String)>,
    pub new_node_buffer: String,
    
    pub model_search: String,
    pub dept_search: String,
    pub zone_search: String,

    pub renaming_zone: Option<String>,
    pub renaming_port: Option<(String, bool, String)>, // (zone, is_input, old_port_name)
    pub rename_buffer: String,
    pub show_clear_modal: bool,
    pub show_inputs_panel: bool,
    pub show_outputs_panel: bool,
    pub show_uv_panel: bool,
    pub shard_rtt: Option<bevy::prelude::Handle<bevy::prelude::Image>>,
    pub cad_viewport_size: bevy_egui::egui::Vec2,
    pub cad_viewport_rect: Option<bevy_egui::egui::Rect>,
    pub pending_3d_drop: Option<(String, String, bevy_egui::egui::Pos2, bevy_egui::egui::Pos2, bool)>,
    pub dragging_over_3d: Option<bevy_egui::egui::Pos2>, 
    pub active_3d_hover: Option<(bevy_egui::egui::Pos2, u32)>, 
}

impl Default for NodeGraphUiState {
    fn default() -> Self {
        Self {
            pan: bevy_egui::egui::Vec2::ZERO,
            zoom: 1.0,
            level: EditorLevel::Model,
            node_positions: HashMap::new(),
            selected_node_id: None,
            dragging_pin: None,
            pending_connection: None,
            new_node_buffer: String::new(),
            model_search: String::new(),
            dept_search: String::new(),
            zone_search: String::new(),
            renaming_zone: None,
            renaming_port: None,
            rename_buffer: String::new(),
            show_clear_modal: false,
            show_inputs_panel: false,
            show_outputs_panel: false,
            show_uv_panel: false,
            shard_rtt: None,
            cad_viewport_size: bevy_egui::egui::Vec2::ZERO,
            cad_viewport_rect: None,
            pending_3d_drop: None,
            dragging_over_3d: None,
            active_3d_hover: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorLevel {
    Model,
    Department,
    Zone(String),
}

#[derive(Resource, Default, Debug)]
pub struct BrainTopologyGraph {
    pub active_project: Option<String>,
    pub active_path:    Option<std::path::PathBuf>,
    pub sessions:       HashMap<std::path::PathBuf, ProjectSession>,
}

#[derive(Debug, Default, Clone)]
pub struct ProjectSession {
    pub father_id: String,
    pub zones: Vec<String>,
    pub zone_ids: HashMap<String, String>, 
    pub env_rx_nodes: Vec<String>,
    pub env_tx_nodes: Vec<String>,
    pub connections: Vec<(String, String, String, String)>, 
    pub node_inputs: HashMap<String, Vec<String>>,
    pub node_outputs: HashMap<String, Vec<String>>,
    pub layout_cache: HashMap<String, (f32, f32)>, 
    pub shard_anatomies: HashMap<String, ShardAnatomy>,
    pub voxel_size_um: f32,
    pub is_dirty: bool,
}

#[derive(Event, Debug, Clone)]
pub struct LoadGraphEvent {
    pub project_name: String,
}

#[derive(Debug, Clone)]
pub enum RenameTarget {
    Shard { old_name: String, new_name: String, id: String },
    IoPin { zone: String, is_input: bool, old_name: String, new_name: String },
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    Zone { name: String, id: String },
    Connection { from: String, from_port: String, to: String, to_port: String },
    IoPin { zone: String, is_input: bool, name: String },
    Layer { zone: String, name: String },
}

#[derive(Debug, Clone)]
pub enum CreateTarget {
    Zone { name: String, pos: bevy_egui::egui::Pos2 },
    EnvRx { name: String, pos: bevy_egui::egui::Pos2 },
    EnvTx { name: String, pos: bevy_egui::egui::Pos2 },
    Connection { from: String, from_port: String, to: String, to_port: String, voxel_z: Option<u32> },
    IoPin { zone: String, is_input: bool, name: String },
    Layer { zone: String, name: String, height_pct: f32 },
}

#[derive(Event, Debug, Clone)]
pub enum TopologyMutation {
    Create(CreateTarget, Option<std::path::PathBuf>),
    Delete(DeleteTarget, Option<std::path::PathBuf>),
    Rename(RenameTarget, Option<std::path::PathBuf>),
}

#[derive(Event, Clone, Debug)]
pub struct SaveProjectEvent;

#[derive(Event, Clone, Debug)]
pub struct CompileGraphEvent; 

#[derive(Event, Clone, Debug)]
pub struct BakeProjectEvent;

#[derive(Debug, Clone)]
pub enum NodeSignal {
    None,
    Dragged(bevy::math::Vec2),
    RightClicked, // Заменили DeleteClicked на вызов контекстного меню
    PortClicked { port_name: String, is_input: bool },
    PortDragStarted { port_name: String, is_input: bool },
    PortDropped { port_name: String, is_input: bool },
}

#[derive(Component)]
pub struct ShardCadEntity;

#[derive(Component)]
pub struct CadCameraState {
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, // Вращение вокруг Y
    pub beta: f32,  // Вращение вверх/вниз
}

impl Default for CadCameraState {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 60.0,
            alpha: std::f32::consts::PI / 4.0,
            beta: 0.5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShardLayer {
    pub name: String,
    pub height_pct: f32,
}

#[derive(Debug, Clone)]
pub struct ShardAnatomy {
    pub w: f32,
    pub d: f32,
    pub h: f32,
    pub layers: Vec<ShardLayer>,
}

