use bevy::prelude::*;
use layout_api::ContextMenuActionTriggeredEvent;
use std::path::Path;
use toml_edit::{value, ArrayOfTables, DocumentMut, InlineTable, Item, Table};

///      WM (  )
pub fn wm_file_ops_context_menu_system(
    mut trigger_events: EventReader<ContextMenuActionTriggeredEvent>,
) {
    for ev in trigger_events.read() {
        if !ev.action_id.starts_with("wm.") {
            continue;
        }

        match ev.action_id.as_str() {
            "wm.create_file" => {
                info!(
                    "WM: Executing 'Create File' for window {:?}",
                    ev.target_window
                );
                //    VFS (axicor-core)
            }
            "wm.delete_file" => {
                info!(
                    "WM: Executing 'Delete File' for window {:?}",
                    ev.target_window
                );
            }
            _ => warn!("Unknown WM action: {}", ev.action_id),
        }
    }
}

///  TOML     .
/// [Sandbox]  Overlay FS  layout_api.
pub fn load_document(path: &Path) -> Result<DocumentMut, String> {
    let content =
        layout_api::overlay_read_to_string(path).map_err(|e| format!("FS Error: {}", e))?;
    content
        .parse::<DocumentMut>()
        .map_err(|e| format!("Parse Error: {}", e))
}

///   TOML    .
pub fn save_document(path: &Path, doc: &DocumentMut) -> Result<(), String> {
    let sandbox_path = layout_api::resolve_sandbox_path(path);
    if let Some(parent) = sandbox_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&sandbox_path, doc.to_string()).map_err(|e| format!("FS Error: {}", e))
}

///     `ArrayOfTables` (, `[[department]]`)
///   target_id    id_table_name.
pub fn remove_array_of_tables_item(
    doc: &mut DocumentMut,
    array_name: &str,
    id_table_name: &str,
    target_id: &str,
) -> bool {
    let mut index_to_remove = None;

    if let Some(arr) = doc
        .get_mut(array_name)
        .and_then(|item| item.as_array_of_tables_mut())
    {
        for (i, table) in arr.iter().enumerate() {
            //   id.      { id = "..." },
            //
            if let Some(id_val) = table
                .get(id_table_name)
                .and_then(|t| t.get("id"))
                .and_then(|v| v.as_str())
            {
                if id_val == target_id {
                    index_to_remove = Some(i);
                    break;
                }
            }
        }

        if let Some(i) = index_to_remove {
            arr.remove(i);
            return true;
        }
    }

    false
}

///   I/O   `io.toml`    Lineage ID (Matrix -> Pin).
pub fn add_io_record(
    doc: &mut DocumentMut,
    section: &str,
    port_name: &str,
    _io_id: &str,
    zone_name: &str,
    width: u32,
    height: u32,
    voxel_z: Option<u32>,
) {
    // 1. Generate Matrix ID: {zone_suffix}_{uuid8}
    let z_suffix = if zone_name.len() >= 4 {
        &zone_name[zone_name.len() - 4..]
    } else {
        zone_name
    };
    let m_uuid_full = axicor_core::config::sys::SystemMeta::generate()
        .id
        .replace("-", "");
    let m_uuid = &m_uuid_full[..8];
    let matrix_id = format!("{}_{}", z_suffix, m_uuid);

    let mut matrix = Table::new();
    let mut m_inline_id = InlineTable::new();
    m_inline_id.insert("id", matrix_id.clone().into());
    matrix.insert(
        "matrix_id_v1",
        Item::Value(toml_edit::Value::InlineTable(m_inline_id)),
    );
    matrix.insert("name", value(format!("{}_matrix", port_name)));

    if section == "input" {
        if let Some(z) = voxel_z {
            matrix.insert("entry_z", value(z as i64));
        } else {
            matrix.insert("entry_z", value("top"));
        }
    } else {
        matrix.insert("entry_z", value("bottom"));
    }

    // 2. Generate Pin ID: {matrix_uuid_suffix}_{uuid4}
    let m_suffix = &m_uuid[m_uuid.len() - 4..];
    let p_uuid_full = axicor_core::config::sys::SystemMeta::generate()
        .id
        .replace("-", "");
    let p_uuid = &p_uuid_full[..4];
    let pin_id = format!("{}_{}", m_suffix, p_uuid);

    let mut pin = Table::new();
    let mut p_inline_id = InlineTable::new();
    p_inline_id.insert("id", pin_id.into());
    pin.insert(
        "pin_id_v1",
        Item::Value(toml_edit::Value::InlineTable(p_inline_id)),
    );

    pin.insert("name", value(port_name));
    pin.insert("width", value(width as i64));
    pin.insert("height", value(height as i64));
    pin.insert("local_u", value(0.0));
    pin.insert("local_v", value(0.0));
    pin.insert("u_width", value(1.0));
    pin.insert("v_height", value(1.0));
    pin.insert("target_type", value("All"));
    pin.insert("stride", value(1i64));

    let mut pins_array = ArrayOfTables::new();
    pins_array.push(pin);
    matrix.insert("pin", Item::ArrayOfTables(pins_array));

    if !doc.contains_key(section) {
        doc.insert(section, Item::ArrayOfTables(ArrayOfTables::new()));
    }
    if let Some(arr) = doc
        .get_mut(section)
        .and_then(|i| i.as_array_of_tables_mut())
    {
        arr.push(matrix);
    }
}

///   I/O   io.toml   .
/// [DOD FIX]  ,        .
pub fn remove_io_record_by_name(doc: &mut DocumentMut, section: &str, target_name: &str) -> bool {
    let mut matrix_to_remove = None;
    let mut pin_removed = false;

    if let Some(matrices) = doc
        .get_mut(section)
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for (m_idx, matrix) in matrices.iter_mut().enumerate() {
            let mut pin_to_remove = None;
            if let Some(pins) = matrix
                .get_mut("pin")
                .and_then(|p| p.as_array_of_tables_mut())
            {
                for (p_idx, pin) in pins.iter().enumerate() {
                    if let Some(name_val) = pin.get("name").and_then(|v| v.as_str()) {
                        if name_val == target_name {
                            pin_to_remove = Some(p_idx);
                            break;
                        }
                    }
                }
                if let Some(idx) = pin_to_remove {
                    pins.remove(idx);
                    pin_removed = true;
                    //
                    if pins.is_empty() {
                        matrix_to_remove = Some(m_idx);
                    }
                }
            }
            if pin_removed {
                break;
            }
        }

        if let Some(idx) = matrix_to_remove {
            matrices.remove(idx);
        }
    }
    pin_removed
}

///       .
pub fn remove_connection_record(
    doc: &mut DocumentMut,
    from: &str,
    to: &str,
    out_matrix: &str,
    in_matrix: &str,
) -> bool {
    let mut index_to_remove = None;
    if let Some(arr) = doc
        .get_mut("connection")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for (i, table) in arr.iter().enumerate() {
            let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let m = table
                .get("output_matrix")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let im = table
                .get("input_matrix")
                .and_then(|v| v.as_str())
                .unwrap_or("in");

            if f == from && t == to && m == out_matrix && im == in_matrix {
                index_to_remove = Some(i);
                break;
            }
        }
        if let Some(i) = index_to_remove {
            arr.remove(i);
            return true;
        }
    }
    false
}

///  Z-   io.toml
pub fn update_io_input_z(doc: &mut DocumentMut, target_name: &str, voxel_z: u32) -> bool {
    if let Some(arr) = doc
        .get_mut("input")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for table in arr.iter_mut() {
            if table.get("name").and_then(|v| v.as_str()) == Some(target_name) {
                table.insert("entry_z", value(voxel_z as i64));
                return true;
            }
        }
    }
    false
}

///  Z-   brain.toml ( simulation.toml)
pub fn update_connection_z(
    doc: &mut DocumentMut,
    from: &str,
    from_port: &str,
    to: &str,
    voxel_z: u32,
) -> bool {
    if let Some(arr) = doc
        .get_mut("connection")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for table in arr.iter_mut() {
            let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let m = table
                .get("output_matrix")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if f == from && t == to && m == from_port {
                table.insert("entry_z", value(voxel_z as i64));
                return true;
            }
        }
    }
    false
}

///   
pub fn add_anatomy_layer_record(doc: &mut DocumentMut, name: &str, height_pct: f32) {
    if !doc.contains_key("layer") {
        doc.insert("layer", Item::ArrayOfTables(ArrayOfTables::new()));
    }

    if let Some(arr) = doc
        .get_mut("layer")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for table in arr.iter_mut() {
            let current_val = table
                .get("height_pct")
                .and_then(|v| {
                    Some(
                        v.as_float()
                            .unwrap_or_else(|| v.as_integer().unwrap_or(1) as f64)
                            as f32,
                    )
                })
                .unwrap_or(1.0);
            let new_val = current_val * (1.0 - height_pct);
            table.insert("height_pct", value(new_val as f64));
        }

        let mut new_table = Table::new();
        new_table.insert("name", value(name));
        new_table.insert("height_pct", value(height_pct as f64));
        new_table.insert("density", value(0.1));
        new_table.insert(
            "composition",
            Item::Value(toml_edit::Value::InlineTable(InlineTable::new())),
        );

        arr.push(new_table);
    }
}

///   
pub fn remove_anatomy_layer_record(doc: &mut DocumentMut, target_name: &str) -> bool {
    let mut index_to_remove = None;
    let mut removed_pct = 0.0;

    if let Some(arr) = doc
        .get_mut("layer")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for (i, table) in arr.iter().enumerate() {
            if let Some(name_val) = table.get("name").and_then(|v| v.as_str()) {
                if name_val == target_name {
                    index_to_remove = Some(i);
                    removed_pct = table
                        .get("height_pct")
                        .and_then(|v| {
                            Some(
                                v.as_float()
                                    .unwrap_or_else(|| v.as_integer().unwrap_or(1) as f64)
                                    as f32,
                            )
                        })
                        .unwrap_or(1.0);
                    break;
                }
            }
        }

        if let Some(i) = index_to_remove {
            arr.remove(i);

            if removed_pct < 1.0 {
                let divisor = 1.0 - removed_pct;
                for table in arr.iter_mut() {
                    let current_val = table
                        .get("height_pct")
                        .and_then(|v| {
                            Some(
                                v.as_float()
                                    .unwrap_or_else(|| v.as_integer().unwrap_or(1) as f64)
                                    as f32,
                            )
                        })
                        .unwrap_or(1.0);
                    let new_val = current_val / divisor;
                    table.insert("height_pct", value(new_val as f64));
                }
            }
            return true;
        }
    }
    false
}

pub fn find_path_by_id(
    cache: &layout_api::ProjectFsCache,
    target_id: &str,
) -> Option<std::path::PathBuf> {
    fn search(nodes: &[layout_api::ProjectNode], target_id: &str) -> Option<std::path::PathBuf> {
        for node in nodes {
            if node.id == target_id {
                return Some(node.path.clone());
            }
            if let Some(p) = search(&node.children, target_id) {
                return Some(p);
            }
        }
        None
    }
    for proj in &cache.projects {
        if let Some(p) = search(&proj.root_nodes, target_id) {
            return Some(p);
        }
    }
    None
}
