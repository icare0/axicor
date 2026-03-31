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
        // В плагине мы не можем достучаться до crate::layout (это приватный модуль axicor-lab).
        // Поэтому для сохранения Layout-файлов мы просто используем старый путь .tmp, 
        // так как они и так являются временными метаданными редактора.
        let layout_path = parent_dir.join(format!(".{}.layout.tmp.toml", toml_fname.replace(".toml", "")));
        
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

use genesis_core::config::io::IoConfig;

/// Синхронизирует RAM-кэш портов (пинов) с реальными файлами io.toml на диске при загрузке графа.
/// Гарантирует актуальность пинов Node Editor после холодного старта.
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

        if let Ok(io_config) = IoConfig::load(&io_path) {
            let inputs: Vec<String> = io_config.inputs.into_iter().map(|i| i.name).collect();
            let outputs: Vec<String> = io_config.outputs.into_iter().map(|o| o.name).collect();
            session.node_inputs.insert(zone_name.clone(), inputs);
            session.node_outputs.insert(zone_name.clone(), outputs);
        }
    }
}
