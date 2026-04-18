use crate::domain::{NodeGraphUiState, ProjectSession};
use std::path::Path;

///      (Layout).
///  AST- (TOML)     DOD-  .
pub fn flush_session_to_disk(
    base_path: &Path,
    session: &ProjectSession,
    ui_state: Option<&NodeGraphUiState>,
    _is_tmp: bool,
) -> Result<(), String> {
    let toml_fname = base_path
        .file_name()
        .ok_or("Invalid path")?
        .to_string_lossy();
    let parent_dir = base_path.parent().unwrap_or(Path::new("."));

    if let Some(state) = ui_state {
        let cold_layout_path =
            parent_dir.join(format!("{}.layout.toml", toml_fname.replace(".toml", "")));
        //        crate::layout (   axicor-lab).
        //    Layout-       API.
        let layout_path = layout_api::resolve_sandbox_path(&cold_layout_path);
        if let Some(p) = layout_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        let mut layout_toml = String::from("[nodes]\n");
        let mut has_layout = false;

        for (zone_name, pos) in &state.node_positions {
            if session.zones.contains(zone_name) {
                let id = session
                    .zone_ids
                    .get(zone_name)
                    .cloned()
                    .unwrap_or_else(|| zone_name.clone());
                layout_toml.push_str(&format!(
                    "[\"nodes\".\"{}\"]\nx = {:.1}\ny = {:.1}\n\n",
                    id, pos.x, pos.y
                ));
                has_layout = true;
            }
        }

        if has_layout {
            std::fs::write(&layout_path, layout_toml).map_err(|e| format!("FS Error: {}", e))?;
        }
    }

    Ok(())
}

///  RAM-  ()   io.toml.
/// [DOD FIX]     Overlay FS,    .
pub fn sync_io_ports_from_disk(base_path: &Path, session: &mut crate::domain::ProjectSession) {
    let project_dir = base_path.parent().unwrap_or(std::path::Path::new("."));
    let path_str = base_path.to_string_lossy();
    let is_sim = path_str.ends_with("simulation.toml");
    let is_zone_level = path_str.ends_with("shard.toml")
        || path_str.ends_with("io.toml")
        || path_str.ends_with("blueprints.toml")
        || path_str.ends_with("anatomy.toml");
    let dept_name = base_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(".toml", "");

    for zone_name in &session.zones {
        let io_path = if is_sim {
            project_dir.join(zone_name).join("io.toml")
        } else if is_zone_level {
            // [DOD FIX]  - project_dir
            project_dir.join("io.toml")
        } else {
            project_dir.join(&dept_name).join(zone_name).join("io.toml")
        };

        if let Ok(content) = layout_api::overlay_read_to_string(&io_path) {
            if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                let mut inputs = Vec::new();
                let mut outputs = Vec::new();

                if let Some(arr) = doc.get("input").and_then(|i| i.as_array_of_tables()) {
                    for matrix in arr.iter() {
                        if let Some(pins) = matrix.get("pin").and_then(|p| p.as_array_of_tables()) {
                            for pin in pins.iter() {
                                if let Some(name) = pin.get("name").and_then(|v| v.as_str()) {
                                    inputs.push(name.to_string());
                                }
                            }
                        }
                    }
                }

                if let Some(arr) = doc.get("output").and_then(|i| i.as_array_of_tables()) {
                    for matrix in arr.iter() {
                        if let Some(pins) = matrix.get("pin").and_then(|p| p.as_array_of_tables()) {
                            for pin in pins.iter() {
                                if let Some(name) = pin.get("name").and_then(|v| v.as_str()) {
                                    outputs.push(name.to_string());
                                }
                            }
                        }
                    }
                }

                // [DOD FIX] Merge:   "in"/"out"   .
                //    ,    .
                //   ,      RAM (  loader).
                if !inputs.is_empty() {
                    if !inputs.contains(&"in".to_string()) {
                        inputs.insert(0, "in".to_string());
                    }
                    session.node_inputs.insert(zone_name.clone(), inputs);
                }
                //    RAM      loader   defaults

                if !outputs.is_empty() {
                    if !outputs.contains(&"out".to_string()) {
                        outputs.insert(0, "out".to_string());
                    }
                    session.node_outputs.insert(zone_name.clone(), outputs);
                }
            }
        }
    }
}
