use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::TopologyMutation;
use genesis_core::config::sys::SystemMeta;
use crate::layout::systems::wm_file_ops::{load_document, save_document};
use toml_edit::{value, ArrayOfTables, InlineTable, Item, Table};

pub fn create_department_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
    mut ui_states: Query<&mut node_editor::domain::NodeGraphUiState>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };
    if !active_path.to_string_lossy().contains("simulation.toml") { return; }

    for ev in events.read() {
        if let TopologyMutation::AddZone { name, pos } = ev {
            info!("[Orchestrator] Starting clean birth of Department: {}", name);

            let project_dir = active_path.parent().unwrap_or(Path::new("."));

            let mut doc = match load_document(&active_path) {
                Ok(d) => d,
                Err(e) => {
                    error!("[Orchestrator] Could not load AST for simulation.toml: {}", e);
                    continue;
                }
            };

            let sim_id = doc.get("model_id_v1")
                .and_then(|i| i.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("0000")
                .to_string();

            let sim_suffix = if sim_id.len() >= 4 { &sim_id[sim_id.len()-4..] } else { "0000" };

            let dept_uuid = SystemMeta::generate().id;
            let dept_suffix = if dept_uuid.len() >= 4 { &dept_uuid[dept_uuid.len()-4..] } else { "0000" };
            let structured_dept_id = format!("{}_{}", sim_suffix, dept_suffix);

            let mut dept_table = Table::new();
            dept_table.insert("name", value(name.clone()));
            dept_table.insert("config", value(format!("{}.toml", name)));

            let mut inline_id = InlineTable::new();
            inline_id.insert("id", structured_dept_id.clone().into());
            dept_table.insert("depart_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));

            if !doc.contains_key("department") {
                doc.insert("department", Item::ArrayOfTables(ArrayOfTables::new()));
            }
            if let Some(arr) = doc.get_mut("department").and_then(|i| i.as_array_of_tables_mut()) {
                arr.push(dept_table);
            }

            if let Err(e) = save_document(&active_path, &doc) {
                error!("[Orchestrator] Failed to save simulation.toml: {}", e);
                continue;
            }

            let brain_path = project_dir.join(format!("{}.toml", name));
            let brain_toml = format!(
r#"[depart_id_v1]
id = "{}"

[simulation]
config = "simulation.toml"
"#, structured_dept_id);

            let _ = fs::write(&brain_path, brain_toml);

            info!("[Orchestrator] Department {} successfully born and registered.", name);

            if let Some(session) = graph.sessions.get_mut(&active_path) {
                session.zones.push(name.clone());
                session.zone_ids.insert(name.clone(), structured_dept_id.clone());
                session.node_inputs.insert(name.clone(), vec!["in".to_string()]);
                session.node_outputs.insert(name.clone(), vec!["out".to_string()]);
                session.layout_cache.insert(structured_dept_id, (pos.x, pos.y));
                session.is_dirty = true;
            }
            for mut ui in ui_states.iter_mut() {
                ui.node_positions.insert(name.clone(), *pos);
            }
        }
    }
}
