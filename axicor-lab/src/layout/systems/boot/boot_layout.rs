use bevy::prelude::*;
use std::fs;
use crate::layout::domain::{WorkspaceState, Pane, SavedLayout};
use layout_api::{DOMAIN_VIEWPORT, DOMAIN_EXPLORER, DOMAIN_NODE_ED};

const LAYOUT_FILE: &str = "config/default_layout.ron";

pub fn boot_layout_system(mut commands: Commands) {
    info!("[WM] Booting Layout Manager");

    // Пытаемся загрузить пользовательский лейаут
    if let Ok(content) = fs::read_to_string(LAYOUT_FILE) {
        if let Ok(saved) = ron::from_str::<SavedLayout>(&content) {
            info!("[WM] Loaded custom layout from {}", LAYOUT_FILE);
            commands.insert_resource(WorkspaceState {
                active_workspace: saved.active_workspace,
                workspace_order: saved.workspace_order,
                trees: saved.trees,
                renaming_workspace: None,
                rename_buffer: String::new(),
            });
            return;
        } else {
            warn!("[WM] Failed to parse {}. Falling back to factory default.", LAYOUT_FILE);
        }
    }

    // Фолбэк на дефолт (только если файла нет)
    info!("[WM] No custom layout found. Booting factory layout.");
    let mut trees = bevy::utils::HashMap::new();
    trees.insert("Main".to_string(), build_default_tree());

    commands.insert_resource(WorkspaceState {
        active_workspace: "Main".to_string(),
        workspace_order: vec!["Main".to_string()],
        trees,
        renaming_workspace: None,
        rename_buffer: String::new(),
    });
}

fn build_default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();
    let pe = tiles.insert_pane(pane(DOMAIN_EXPLORER,  "Explorer"));
    let vp = tiles.insert_pane(pane(DOMAIN_VIEWPORT,  "Connectome"));
    let ne = tiles.insert_pane(pane(DOMAIN_NODE_ED,   "Topology Editor"));

    let right = tiles.insert_vertical_tile(vec![vp, ne]);
    let root  = tiles.insert_horizontal_tile(vec![pe, right]);

    egui_tiles::Tree::new("axicor_main", root, tiles)
}

#[inline]
fn pane(plugin_id: &str, title: &str) -> Pane {
    Pane { plugin_id: plugin_id.into(), title: title.into() }
}
