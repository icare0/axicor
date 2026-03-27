use bevy::prelude::*;
use crate::layout::domain::{WorkspaceState, Pane};
use layout_api::{DOMAIN_VIEWPORT, DOMAIN_EXPLORER, DOMAIN_NODE_ED};

const LAYOUT_FILE: &str = "config/default_layout.ron";

pub fn boot_layout_system(mut commands: Commands) {
    let tree = match std::fs::read_to_string(LAYOUT_FILE) {
        Ok(data) => {
            info!("[WM] Loading workspace layout from {}", LAYOUT_FILE);
            ron::from_str(&data).unwrap_or_else(|e| {
                warn!("[WM] Corrupted layout file: {}. Booting fallback.", e);
                build_fallback_tree()
            })
        }
        Err(_) => {
            info!("[WM] No layout found. Booting fallback.");
            build_fallback_tree()
        }
    };
    commands.insert_resource(WorkspaceState { tree });
}

fn build_fallback_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let pe = tiles.insert_pane(pane(DOMAIN_EXPLORER,  "Explorer"));
    let vp = tiles.insert_pane(pane(DOMAIN_VIEWPORT,  "Connectome"));
    let ne = tiles.insert_pane(pane(DOMAIN_NODE_ED,   "Topology Editor"));

    let right = tiles.insert_vertical_tile(vec![vp, ne]);
    let root  = tiles.insert_horizontal_tile(vec![pe, right]);

    egui_tiles::Tree::new("axicor_workspace", root, tiles)
}

#[inline]
fn pane(plugin_id: &str, title: &str) -> Pane {
    Pane { plugin_id: plugin_id.into(), title: title.into() }
}