// Кнопка Bake запускает процесс сборки модели.

use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, BakeProjectEvent, EditorLevel, NodeGraphUiState};

pub fn bake_project_system(
    mut bake_ev: EventReader<BakeProjectEvent>,
    graph: Res<BrainTopologyGraph>,
    query: Query<&NodeGraphUiState>,
) {
    if bake_ev.read().next().is_none() { return; }

    let Some(active_path) = &graph.active_path else { return };
    
    // DOD FIX: Определяем, запекаем ли мы всю модель или конкретный департамент
    let mut args = vec!["run".to_string(), "--release".to_string(), "-p".to_string(), "genesis-baker".to_string(), "--bin".to_string(), "baker".to_string(), "--".to_string()];
    
    if let Some(ui) = query.iter().next() {
        if ui.level == EditorLevel::Model {
            println!("🔥 [Node Editor] Orchestrating GLOBAL Model Bake: {:?}", active_path);
            args.push("--model".to_string());
            args.push(active_path.to_str().unwrap().to_string());
        } else {
            println!("🔥 [Node Editor] Baking Department: {:?}", active_path);
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
            .expect("Failed to spawn genesis-baker")
            .wait()
            .unwrap();

        if status.success() {
            println!("✅ [Node Editor] Baking finished successfully!");
        } else {
            eprintln!("❌ [Node Editor] Baking failed.");
        }
    });
}
