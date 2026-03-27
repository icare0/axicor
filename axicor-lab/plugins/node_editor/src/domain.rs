use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_egui::egui;

use genesis_core::config::brain::BrainConfig;
use genesis_core::config::anatomy::AnatomyConfig;
use genesis_core::config::io::IoConfig;
use std::path::PathBuf;

#[derive(Resource, Default, Debug)]
pub struct BrainTopologyGraph {
    pub active_project: Option<String>,
    pub active_path: Option<PathBuf>,
    pub config: Option<BrainConfig>,
    pub io_configs: HashMap<String, IoConfig>,
    pub anatomy_configs: HashMap<String, AnatomyConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorLevel {
    Model,
    Zone(String),
    Shard { zone_name: String, shard_name: String },
}

#[derive(Resource, Debug)]
pub struct NodeGraphUiState {
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub node_positions: HashMap<String, egui::Pos2>,
    pub visual_groups: HashMap<String, Vec<String>>,
    pub level: EditorLevel,
    pub active_wire: Option<String>,
}

impl Default for NodeGraphUiState {
    fn default() -> Self {
        Self {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            node_positions: HashMap::new(),
            visual_groups: HashMap::new(),
            level: EditorLevel::Zone("Main Network".to_string()),
            active_wire: None,
        }
    }
}

// === ИНТЕНТЫ (События) ===
#[derive(Event, Debug, Clone)]
pub enum TopologyMutation {
    AddZone { name: String, pos: egui::Pos2 },
    AddConnection { from: String, to: String, out_matrix: String },
}

#[derive(Event, Debug, Clone)]
pub struct SaveProjectEvent;

#[derive(Event, Debug, Clone)]
pub struct BakeProjectEvent;

#[derive(Event, Debug, Clone)]
pub struct LoadGraphEvent {
    pub project_name: String,
}
