use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct NodeGraphUiState {
    pub pan: bevy_egui::egui::Vec2,
    pub zoom: f32,
    pub level: EditorLevel,
    pub node_positions: HashMap<String, bevy_egui::egui::Pos2>,
}

impl Default for NodeGraphUiState {
    fn default() -> Self {
        Self {
            pan: bevy_egui::egui::Vec2::ZERO,
            zoom: 1.0,
            level: EditorLevel::Model,
            node_positions: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorLevel {
    Model,
    Zone(String),
    Shard { zone_name: String, shard_name: String },
}

// DOD: Кэш макро-топологии (только имена зон и связей для рендера)
#[derive(Resource, Default, Debug)]
pub struct BrainTopologyGraph {
    pub project_name: Option<String>,
    pub zones: Vec<String>,
    pub connections: Vec<(String, String)>, // (From, To)
    
    // Совместимость с текущим ui.rs и pipeline.rs
    pub active_project: Option<String>,
    pub config: Option<genesis_core::config::brain::BrainConfig>,
    pub io_configs: HashMap<String, genesis_core::config::io::IoConfig>,
    pub anatomy_configs: HashMap<String, genesis_core::config::anatomy::AnatomyConfig>,
}

#[derive(Event, Clone, Debug)]
pub struct LoadGraphEvent {
    pub project_name: String,
}

#[derive(Event, Debug, Clone)]
pub enum TopologyMutation {
    AddZone { name: String, pos: bevy_egui::egui::Pos2 },
    AddConnection { from: String, to: String },
}

#[derive(Event, Clone, Debug)]
pub struct SaveProjectEvent;

#[derive(Event, Clone, Debug)]
pub struct BakeProjectEvent;
