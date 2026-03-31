use std::path::Path;
use crate::domain::{NodeGraphUiState, ProjectSession};

/// Сохранение только визуального состояния редактора (Layout).
/// Мутация AST-деревьев (TOML) отсюда вырезана в пользу DOD-роутеров оконного менеджера.
pub fn flush_session_to_disk(
    base_path: &Path,
    session: &ProjectSession,
    ui_state: Option<&NodeGraphUiState>,
    _is_tmp: bool
) -> Result<(), String> {
    let toml_fname = base_path.file_name().ok_or("Invalid path")?.to_string_lossy();
    let parent_dir = base_path.parent().unwrap_or(Path::new("."));

    if let Some(state) = ui_state {
        let cold_layout_path = parent_dir.join(format!("{}.layout.toml", toml_fname.replace(".toml", "")));
        // В плагине мы не можем достучаться до crate::layout (это приватный модуль axicor-lab).
        // Поэтому для сохранения Layout-файлов мы теперь используем официальный резолвер из API.
        let layout_path = layout_api::resolve_sandbox_path(&cold_layout_path);
        if let Some(p) = layout_path.parent() { let _ = std::fs::create_dir_all(p); }
        
        let mut layout_toml = String::from("[nodes]\n");
        let mut has_layout = false;

        for (zone_name, pos) in &state.node_positions {
            if session.zones.contains(zone_name) {
                let id = session.zone_ids.get(zone_name).cloned().unwrap_or_else(|| zone_name.clone());
                layout_toml.push_str(&format!("[\"nodes\".\"{}\"]\nx = {:.1}\ny = {:.1}\n\n", id, pos.x, pos.y));
                has_layout = true;
            }
        }

        if has_layout {
            std::fs::write(&layout_path, layout_toml).map_err(|e| format!("FS Error: {}", e))?;
        }
    }

    Ok(())
}

/// Синхронизирует RAM-кэш портов (пинов) с файлами io.toml.
/// [DOD FIX] Использует абстрактный парсинг поверх Overlay FS, игнорируя строгие схемы бекенда.
pub fn sync_io_ports_from_disk(base_path: &Path, session: &mut crate::domain::ProjectSession) {
    let project_dir = base_path.parent().unwrap_or(Path::new("."));
    let path_str = base_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = base_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");

    for zone_name in &session.zones {
        let io_path = if is_sim {
            project_dir.join(zone_name).join("io.toml")
        } else {
            project_dir.join(&dept_name).join(zone_name).join("io.toml")
        };

        if let Ok(content) = layout_api::overlay_read_to_string(&io_path) {
            if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
                let mut inputs = Vec::new();
                let mut outputs = Vec::new();

                if let Some(arr) = doc.get("input").and_then(|i| i.as_array_of_tables()) {
                    for t in arr.iter() {
                        if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                            inputs.push(name.to_string());
                        }
                    }
                }

                if let Some(arr) = doc.get("output").and_then(|i| i.as_array_of_tables()) {
                    for t in arr.iter() {
                        if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                            outputs.push(name.to_string());
                        }
                    }
                }

                if !inputs.is_empty() {
                    session.node_inputs.insert(zone_name.clone(), inputs);
                }
                if !outputs.is_empty() {
                    session.node_outputs.insert(zone_name.clone(), outputs);
                }
            }
        }
    }
}
