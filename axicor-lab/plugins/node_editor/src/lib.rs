use bevy::prelude::*;
use std::path::PathBuf;
use layout_api::LoadGraphEvent;
use bevy::utils::HashMap;
use bevy_egui::egui;

// Используем нативные конфиги ядра
use genesis_core::config::brain::BrainConfig;
use genesis_core::config::io::IoConfig;

#[derive(Resource, Default, Debug)]
pub struct BrainTopologyGraph {
    pub active_project: Option<String>,
    pub active_path: Option<PathBuf>,
    pub config: Option<BrainConfig>,
    pub io_configs: HashMap<String, IoConfig>, // Кэш пинов для каждой зоны
}

// Состояние исключительно для отрисовки (Dumb View)
#[derive(Resource, Debug)]
pub struct NodeGraphUiState {
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub node_positions: HashMap<String, egui::Pos2>, // Кэш позиций нод
}

impl Default for NodeGraphUiState {
    fn default() -> Self {
        Self {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
            node_positions: HashMap::new(),
        }
    }
}

pub struct NodeEditorPlugin;

impl Plugin for NodeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrainTopologyGraph>()
           .init_resource::<NodeGraphUiState>()
           .add_event::<LoadGraphEvent>()
           .add_systems(Update, sync_topology_graph_system);
    }
}

// Фоновая система чтения (Zero-Lock)
fn sync_topology_graph_system(
    mut events: EventReader<LoadGraphEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_state: ResMut<NodeGraphUiState>,
) {
    for ev in events.read() {
        let path = PathBuf::from("Genesis-Models").join(&ev.project_name).join("brain.toml");
        
        match genesis_core::config::brain::parse_brain_config(&path) {
            Ok(config) => {
                println!("[NodeEditor] Loaded brain topology for: {}", ev.project_name);
                
                let mut x = 100.0;
                let mut io_configs = HashMap::new();
                
                // Получаем базовую директорию проекта для поиска io.toml
                let project_dir = path.parent().unwrap_or(std::path::Path::new("."));

                for zone in &config.zones {
                    // DOD: Читаем io.toml для генерации пинов
                    let io_path = project_dir.join(&zone.io);
                    if let Ok(io_cfg) = IoConfig::load(&io_path) {
                        io_configs.insert(zone.name.clone(), io_cfg);
                    }

                    ui_state.node_positions.entry(zone.name.clone()).or_insert_with(|| {
                        let pos = egui::Pos2::new(x, 150.0);
                        x += 300.0; 
                        pos
                    });
                }

                graph.active_project = Some(ev.project_name.clone());
                graph.active_path = Some(path);
                graph.config = Some(config);
                graph.io_configs = io_configs; // Сохраняем в кэш
            }
            Err(e) => {
                eprintln!("[NodeEditor] Failed to load brain.toml: {}", e);
                graph.config = None;
                graph.io_configs.clear();
            }
        }
    }
}
