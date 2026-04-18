//  Bake    .

use crate::domain::{BakeProjectEvent, BrainTopologyGraph, EditorLevel, NodeGraphUiState};
use bevy::prelude::*;
use tracing::{error, info};

pub fn bake_project_system(
    mut bake_ev: EventReader<BakeProjectEvent>,
    graph: Res<BrainTopologyGraph>,
    query: Query<&NodeGraphUiState>,
) {
    if bake_ev.read().next().is_none() {
        return;
    }

    let Some(active_path) = &graph.active_path else {
        return;
    };

    // DOD FIX: ,
    let mut args = vec![
        "run".to_string(),
        "--release".to_string(),
        "-p".to_string(),
        "axicor-baker".to_string(),
        "--bin".to_string(),
        "axicor-baker".to_string(),
        "--".to_string(),
        "--yes".to_string(),
    ];
    if let Some(ui) = query.iter().next() {
        if ui.level == EditorLevel::Model {
            info!(
                " [Node Editor] Orchestrating GLOBAL Model Bake: {:?}",
                active_path
            );
            args.push("--model".to_string());
            args.push(active_path.to_str().unwrap().to_string());
        } else {
            info!(" [Node Editor] Baking Department: {:?}", active_path);
            args.push("--brain".to_string());
            args.push(active_path.to_str().unwrap().to_string());
        }
    } else {
        args.push("--brain".to_string());
        args.push(active_path.to_str().unwrap().to_string());
    }

    std::thread::spawn(move || {
        let status = std::process::Command::new("cargo")
            .args(&args)
            .spawn()
            .expect("Failed to spawn axicor-baker")
            .wait()
            .unwrap();

        if status.success() {
            info!("[OK] [Node Editor] Baking finished successfully!");
        } else {
            error!("[ERROR] [Node Editor] Baking failed.");
        }
    });
}
