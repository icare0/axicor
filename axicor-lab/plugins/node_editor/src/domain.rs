use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ShardPopout {
    Io,
    Blueprints,
}

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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorLevel {
    Model,
    Department,
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
    pub shard_blueprints: HashMap<String, ShardBlueprint>,
    pub shard_io: HashMap<String, ShardIoData>, // [DOD FIX] Кэш данных I/O матриц
    pub voxel_size_um: f32,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NeuronType {
    pub name: String,
    #[serde(default)] pub is_inhibitory: bool,
    #[serde(default)] pub threshold: i32,
    #[serde(default)] pub rest_potential: i32,
    #[serde(default)] pub leak_rate: i32,
    #[serde(default)] pub refractory_period: u8,
    #[serde(default)] pub synapse_refractory_period: u8,
    #[serde(default)] pub signal_propagation_length: u8,
    #[serde(default)] pub spontaneous_firing_period_ticks: u32,
    #[serde(default)] pub homeostasis_penalty: i32,
    #[serde(default)] pub homeostasis_decay: u16,
    #[serde(default)] pub adaptive_leak_max: i32,
    #[serde(default)] pub adaptive_leak_gain: u16,
    #[serde(default)] pub adaptive_mode: u8,
    #[serde(default)] pub gsop_potentiation: u16,
    #[serde(default)] pub gsop_depression: u16,
    #[serde(default)] pub initial_synapse_weight: u16,
    #[serde(default)] pub d1_affinity: u8,
    #[serde(default)] pub d2_affinity: u8,
    #[serde(default)] pub slot_decay_ltm: u8,
    #[serde(default)] pub slot_decay_wm: u8,
    #[serde(default)] pub ltm_slot_count: u8,
    #[serde(default)] pub steering_fov_deg: f32,
    #[serde(default)] pub steering_radius_um: f32,
    #[serde(default)] pub steering_weight_inertia: f32,
    #[serde(default)] pub steering_weight_sensor: f32,
    #[serde(default)] pub steering_weight_jitter: f32,
    #[serde(default)] pub growth_vertical_bias: f32,
    #[serde(default)] pub type_affinity: f32,
    #[serde(default)] pub sprouting_weight_distance: f32,
    #[serde(default)] pub sprouting_weight_power: f32,
    #[serde(default)] pub sprouting_weight_explore: f32,
    #[serde(default)] pub inertia_curve: [u8; 16],
    #[serde(default)] pub dendrite_whitelist: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShardBlueprint {
    pub neuron_type: Vec<NeuronType>,
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
    UpdateBlueprint {
        zone: String,
        context_path: Option<std::path::PathBuf>,
    },
    UpdateIo { zone: String, context_path: Option<std::path::PathBuf> }, // [DOD FIX] Интент сохранения матриц
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
    RightClicked, 
    PortClicked { port_name: String, is_input: bool },
    PortDragStarted { port_name: String, is_input: bool },
    PortDropped { port_name: String, is_input: bool },
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

pub fn default_entry_z() -> String { "top".to_string() }
pub fn default_target_type() -> String { "All".to_string() }
pub fn default_stride() -> u32 { 1 }
pub fn default_uv_rect() -> [f32; 4] { [0.0, 0.0, 1.0, 1.0] }
pub fn default_width() -> u32 { 32 }

pub use genesis_core::config::io::{IoConfig as ShardIoData, IoMatrix, IoPin, SysId};
