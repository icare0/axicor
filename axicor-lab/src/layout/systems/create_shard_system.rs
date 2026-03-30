use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::TopologyMutation;
use genesis_core::config::sys::SystemMeta;
use crate::layout::systems::wm_file_ops::{load_document, save_document};
use toml_edit::{value, ArrayOfTables, InlineTable, Item, Table};

pub fn create_shard_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
    mut ui_states: Query<&mut node_editor::domain::NodeGraphUiState>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };

    for ev in events.read() {
        if let TopologyMutation::AddZone { name, pos } = ev {
            let path_str = active_path.to_string_lossy();
            if path_str.contains("simulation.toml") || path_str.contains("shard.toml") { continue; }
            if !path_str.ends_with(".toml") { continue; }

            info!("[Orchestrator] Starting clean birth of Shard: {} inside {:?}", name, active_path);

            let project_dir = active_path.parent().unwrap_or(Path::new("."));
            let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");

            let mut doc = match load_document(&active_path) {
                Ok(d) => d,
                Err(e) => {
                    error!("[Orchestrator] Could not load AST for {:?}: {}", active_path, e);
                    continue;
                }
            };

            let dept_id = doc.get("depart_id_v1")
                .and_then(|i| i.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("0000")
                .to_string();

            let sim_path = project_dir.join("simulation.toml");
            let model_id = if let Ok(sim_doc) = load_document(&sim_path) {
                sim_doc.get("model_id_v1")
                    .and_then(|i| i.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else { "unknown".to_string() };

            let shard_uuid = SystemMeta::generate().id;
            let shard_suffix = if shard_uuid.len() >= 8 { &shard_uuid[shard_uuid.len()-8..] } else { "00000000" };
            let structured_shard_id = format!("{}_{}", dept_id, shard_suffix);

            let mut zone_table = Table::new();
            zone_table.insert("name", value(name.clone()));

            let mut inline_id = InlineTable::new();
            inline_id.insert("id", structured_shard_id.clone().into());
            zone_table.insert("shard_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));

            zone_table.insert("blueprints", value(format!("{}/{}/blueprints.toml", dept_name, name)));
            zone_table.insert("anatomy", value(format!("{}/{}/anatomy.toml", dept_name, name)));
            zone_table.insert("io", value(format!("{}/{}/io.toml", dept_name, name)));
            zone_table.insert("baked_dir", value(format!("baked/{}/", name)));

            if !doc.contains_key("zone") {
                doc.insert("zone", Item::ArrayOfTables(ArrayOfTables::new()));
            }
            if let Some(arr) = doc.get_mut("zone").and_then(|i| i.as_array_of_tables_mut()) {
                arr.push(zone_table);
            }

            if let Err(e) = save_document(&active_path, &doc) {
                error!("[Orchestrator] Failed to save AST after shard creation: {}", e);
                continue;
            }

            let shard_dir = project_dir.join(&dept_name).join(name);
            let _ = fs::create_dir_all(&shard_dir);

            let _ = fs::write(shard_dir.join("anatomy.toml"), "[[layer]]\nname = \"Main\"\nheight_pct = 1.0\ndensity = 0.1\ncomposition = {}\n");
            let _ = fs::write(shard_dir.join("blueprints.toml"), "[[neuron_type]]\nname = \"Default\"\nthreshold = 20000\nrest_potential = 0\nleak_rate = 100\nhomeostasis_penalty = 1000\nhomeostasis_decay = 10\nrefractory_period = 10\nsynapse_refractory_period = 10\nsignal_propagation_length = 5\n");
            let _ = fs::write(shard_dir.join("io.toml"), format!("[shard_id_v1]\nid = \"{}-IO\"\n\n[[input]]\nname = \"in\"\nwidth = 32\nheight = 32\n\n[[output]]\nname = \"out\"\nwidth = 32\nheight = 32\n", structured_shard_id));

            let shard_toml = format!(r#"model_id_v1 = {{ id = "{}" }}
depart_id_v1 = {{ id = "{}" }}
shard_id_v1 = {{ id = "{}" }}

zone_id = "{}"

[dimensions]
w = 32
d = 32
h = 32

[world_offset]
x = 0
y = 0
z = 0

[settings]
night_interval_ticks = 10000
prune_threshold = 15
max_sprouts = 16
ghost_capacity = 20000
"#, model_id, dept_id, structured_shard_id, structured_shard_id);

            let _ = fs::write(shard_dir.join("shard.toml"), shard_toml);

            info!("[Orchestrator] Shard {} successfully born with full lineage.", name);

            if let Some(session) = graph.sessions.get_mut(&active_path) {
                session.zones.push(name.clone());
                session.zone_ids.insert(name.clone(), structured_shard_id.clone());
                session.node_inputs.insert(name.clone(), vec!["in".to_string()]);
                session.node_outputs.insert(name.clone(), vec!["out".to_string()]);
                session.layout_cache.insert(structured_shard_id, (pos.x, pos.y));
                session.is_dirty = true;
            }
            for mut ui in ui_states.iter_mut() {
                ui.node_positions.insert(name.clone(), *pos);
            }
        }
    }
}
