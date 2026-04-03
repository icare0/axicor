use bevy::prelude::*;
use crate::layout::domain::{SaveDefaultLayoutEvent, WorkspaceState, SavedLayout};

const LAYOUT_FILE: &str = "config/default_layout.ron";

pub fn save_layout_system(
    mut events: EventReader<SaveDefaultLayoutEvent>,
    workspace: Res<WorkspaceState>,
) {
    for _ in events.read() {
        if let Err(e) = std::fs::create_dir_all("config") {
            error!("[WM] ❌ Failed to create config directory: {}", e);
            continue;
        }

        let layout = SavedLayout {
            active_workspace: workspace.active_workspace.clone(),
            workspace_order: workspace.workspace_order.clone(),
            trees: workspace.trees.clone(),
        };

        let serialized = ron::ser::to_string_pretty(
            &layout,
            ron::ser::PrettyConfig::default()
        ).expect("FATAL: Failed to serialize window topology");

        if let Err(e) = std::fs::write(LAYOUT_FILE, serialized) {
            error!("[WM] ❌ Failed to write layout to disk: {}", e);
        } else {
            info!("[WM] 💾 Default layout saved to {}", LAYOUT_FILE);
        }
    }
}
