use bevy::prelude::*;
use layout_api::ContextMenuActionTriggeredEvent;
use toml_edit::{DocumentMut, value, ArrayOfTables, Item, Table, InlineTable};
use std::path::Path;

/// Глобальная система обработки файловых операций WM (Строгая зона ответственности)
pub fn wm_file_ops_context_menu_system(
    mut trigger_events: EventReader<ContextMenuActionTriggeredEvent>,
) {
    for ev in trigger_events.read() {
        if !ev.action_id.starts_with("wm.") {
            continue;
        }

        match ev.action_id.as_str() {
            "wm.create_file" => {
                info!("WM: Executing 'Create File' for window {:?}", ev.target_window);
                // Здесь будет вызов VFS (genesis-core)
            }
            "wm.delete_file" => {
                info!("WM: Executing 'Delete File' for window {:?}", ev.target_window);
            }
            _ => warn!("Unknown WM action: {}", ev.action_id),
        }
    }
}

/// Загружает TOML документ с сохранением оригинального форматирования и комментариев
pub fn load_document(path: &Path) -> Result<DocumentMut, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("FS Error: {}", e))?;
    content.parse::<DocumentMut>()
        .map_err(|e| format!("Parse Error: {}", e))
}

/// Сохраняет мутированный TOML документ обратно на диск
pub fn save_document(path: &Path, doc: &DocumentMut) -> Result<(), String> {
    std::fs::write(path, doc.to_string())
        .map_err(|e| format!("FS Error: {}", e))
}

/// Семантическое удаление элемента из `ArrayOfTables` (например, `[[department]]`)
/// Ищет совпадение target_id внутри вложенной таблицы id_table_name.
pub fn remove_array_of_tables_item(
    doc: &mut DocumentMut,
    array_name: &str,
    id_table_name: &str,
    target_id: &str,
) -> bool {
    let mut index_to_remove = None;
    
    if let Some(arr) = doc.get_mut(array_name).and_then(|item| item.as_array_of_tables_mut()) {
        for (i, table) in arr.iter().enumerate() {
            // Пытаемся извлечь id. Работает как для инлайн таблиц { id = "..." }, 
            // так и для стандартных блоков
            if let Some(id_val) = table.get(id_table_name)
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

/// Семантическое добавление I/O записи в `io.toml` шарда с поддержкой Lineage ID.
pub fn add_io_record(doc: &mut DocumentMut, section: &str, name: &str, io_id: &str, width: u32, height: u32) {
    let mut table = Table::new();
    
    let mut inline_id = InlineTable::new();
    inline_id.insert("id", io_id.into());
    table.insert("io_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));

    table.insert("name", value(name));
    table.insert("width", value(width as i64));
    table.insert("height", value(height as i64));

    if !doc.contains_key(section) {
        doc.insert(section, Item::ArrayOfTables(ArrayOfTables::new()));
    }
    if let Some(arr) = doc.get_mut(section).and_then(|i| i.as_array_of_tables_mut()) {
        arr.push(table);
    }
}

/// Семантическое удаление I/O записи из io.toml по имени порта.
pub fn remove_io_record_by_name(doc: &mut DocumentMut, section: &str, target_name: &str) -> bool {
    let mut index_to_remove = None;
    if let Some(arr) = doc.get_mut(section).and_then(|i| i.as_array_of_tables_mut()) {
        for (i, table) in arr.iter().enumerate() {
            if let Some(name_val) = table.get("name").and_then(|v| v.as_str()) {
                if name_val == target_name {
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

/// Семантическое удаление межшардовой связи из родительского конфига.
pub fn remove_connection_record(doc: &mut DocumentMut, from: &str, to: &str, out_matrix: &str) -> bool {
    let mut index_to_remove = None;
    if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) {
        for (i, table) in arr.iter().enumerate() {
            let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let m = table.get("output_matrix").and_then(|v| v.as_str()).unwrap_or("");

            if f == from && t == to && m == out_matrix {
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
