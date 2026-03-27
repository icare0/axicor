use bevy::prelude::*;
use crate::layout::domain::{SaveDefaultLayoutEvent, WorkspaceState};

const LAYOUT_FILE: &str = "config/default_layout.ron";

pub fn save_layout_system(
    mut events: EventReader<SaveDefaultLayoutEvent>,
    workspace: Res<WorkspaceState>, 
) {
    for _ in events.read() {
        if let Err(e) = std::fs::create_dir_all("config") {
            eprintln!("[WM] ❌ Failed to create config directory: {}", e);
            continue;
        }
        
        let serialized = ron::ser::to_string_pretty(
            &workspace.tree, 
            ron::ser::PrettyConfig::default()
        ).expect("FATAL: Failed to serialize window topology");
        
        if let Err(e) = std::fs::write(LAYOUT_FILE, serialized) {
            eprintln!("[WM] ❌ Failed to write layout to disk: {}", e);
        } else {
            println!("[WM] 💾 Default layout saved to {}", LAYOUT_FILE);
        }
    }
}
