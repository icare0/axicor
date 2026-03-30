use std::fs;
use std::path::Path;
use crate::domain::{NodeGraphUiState, ProjectSession};
use toml_edit::{value, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

/// Централизованная утилита сериализации ProjectSession и NodeGraphUiState на диск.
/// Семантическое редактирование AST с сохранением форматирования и комментариев.
pub fn flush_session_to_disk(
    base_path: &Path,
    session: &ProjectSession,
    ui_state: Option<&NodeGraphUiState>,
    is_tmp: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let toml_fname = base_path.file_name().ok_or("Invalid path")?.to_string_lossy();
    let parent_dir = base_path.parent().unwrap_or(Path::new("."));

    let config_path = if is_tmp {
        parent_dir.join(format!("{}.tmp.toml", toml_fname.replace(".toml", "")))
    } else {
        base_path.to_path_buf()
    };

    let original_content = fs::read_to_string(base_path).unwrap_or_default();
    let mut doc = original_content.parse::<DocumentMut>().unwrap_or_default();

    let path_str = base_path.to_string_lossy();
    if path_str.contains("simulation.toml") {
        update_simulation_toml(&mut doc, session);
    } else {
        update_department_toml(&mut doc, session);
    }

    if is_tmp {
        fs::write(&config_path, doc.to_string())?;
    } else {
        // При компиляции перезаписываем основной файл
        fs::write(&config_path, doc.to_string())?;
    }

    if let Some(state) = ui_state {
        let layout_path = parent_dir.join(format!(".{}.layout.tmp.toml", toml_fname));
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
            fs::write(layout_path, layout_toml)?;
        }
    }

    Ok(())
}

fn update_simulation_toml(_doc: &mut DocumentMut, _session: &ProjectSession) {
    // Департаменты мутируются строго через оркестратор WM (create_department_system).
    // Node Editor только читает их. Мы не перетираем массив, чтобы не уничтожить конфиги.
}

fn update_department_toml(doc: &mut DocumentMut, session: &ProjectSession) {
    // 1. Мягкий Merge зон (обновляем ID, не трогая пути blueprints/anatomy)
    if !doc.contains_key("zone") {
        doc.insert("zone", Item::ArrayOfTables(ArrayOfTables::new()));
    }
    
    if let Some(zone_arr) = doc.get_mut("zone").and_then(|i| i.as_array_of_tables_mut()) {
        for name in &session.zones {
            let id = session.zone_ids.get(name).cloned().unwrap_or_default();
            
            let mut found = false;
            for table in zone_arr.iter_mut() {
                if let Some(n) = table.get("name").and_then(|v| v.as_str()) {
                    if n == name {
                        let mut inline_id = InlineTable::new();
                        inline_id.insert("id", id.clone().into());
                        table.insert("shard_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
                        found = true;
                        break;
                    }
                }
            }
            
            if !found {
                let mut new_zone = Table::new();
                new_zone.insert("name", value(name.clone()));
                let mut inline_id = InlineTable::new();
                inline_id.insert("id", id.clone().into());
                new_zone.insert("shard_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
                zone_arr.push(new_zone);
            }
        }
    }

    // 2. Генерация связей (Node Editor полностью контролирует связи)
    let mut new_conns = ArrayOfTables::new();
    for (from, from_port, to, _to_port) in &session.connections {
        let mut conn_table = Table::new();
        conn_table.insert("from", value(from.clone()));
        conn_table.insert("to", value(to.clone()));
        conn_table.insert("output_matrix", value(from_port.clone()));
        new_conns.push(conn_table);
    }
    doc.insert("connection", Item::ArrayOfTables(new_conns));
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
