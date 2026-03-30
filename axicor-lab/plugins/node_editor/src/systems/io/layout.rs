use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState};

pub fn autosave_layout_system(
    time: Res<Time>,
    mut timer: Local<f32>,
    graph: Res<BrainTopologyGraph>,
    query: Query<&NodeGraphUiState>,
) {
    *timer += time.delta_seconds();
    if *timer >= 60.0 {
        *timer = 0.0;
        
        let Some(active_path) = &graph.active_path else { return };
        let Some(session) = graph.sessions.get(active_path) else { return };
        
        if session.zones.is_empty() { return; }

        let proj_dir = active_path.parent().unwrap();
        let toml_fname = active_path.file_name().unwrap_or_default().to_string_lossy();
        
        let mut layout_toml = String::from("[nodes]\n");
        let mut has_data = false;
        
        if let Some(state) = query.iter().next() {
            let is_state_valid = state.node_positions.keys().any(|k| session.zones.contains(k));
            
            if !state.node_positions.is_empty() && is_state_valid {
                has_data = true;
                for (zone, pos) in &state.node_positions {
                    if session.zones.contains(zone) {
                        let id = session.zone_ids.get(zone).cloned().unwrap_or_else(|| zone.clone());
                        layout_toml.push_str(&format!("[\"nodes\".\"{}\"]\n", id));
                        layout_toml.push_str(&format!("x = {:.1}\n", pos.x));
                        layout_toml.push_str(&format!("y = {:.1}\n\n", pos.y));
                    }
                }
            }
        }
        
        if has_data {
            let _ = std::fs::write(proj_dir.join(format!(".{}.layout.tmp.toml", toml_fname)), &layout_toml);
        }
    }
}
