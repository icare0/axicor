use std::collections::HashSet;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;
use crate::domain::ProjectSession;

pub struct ValidationReport {
    pub pruned_ports: usize,
    pub deduped_ports: usize,
}

/// Стандартные порты — неуязвимы к pruning
const PROTECTED_PORTS: &[&str] = &["in", "out"];

/// Точка входа: валидирует холодные io.toml на двух уровнях.
pub fn validate_project(
    base_dir: &Path,
    sessions: &std::collections::HashMap<std::path::PathBuf, ProjectSession>,
) -> ValidationReport {
    let mut report = ValidationReport { pruned_ports: 0, deduped_ports: 0 };

    bevy::log::info!("🔍 [Validator] Scanning {} sessions, base_dir={}", sessions.len(), base_dir.display());

    for (session_path, session) in sessions {
        let file_name = session_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        bevy::log::info!("🔍 [Validator] Session: {} | zones: {:?} | connections: {}", 
            file_name, session.zones, session.connections.len());

        if file_name == "simulation.toml" {
            // Model-level: department io.toml
            validate_model_level(base_dir, session, &mut report);
        } else if file_name.ends_with(".toml")
            && !file_name.starts_with('.')
            && file_name != "manifest.toml"
            && file_name != "shard.toml"
            && file_name != "io.toml"
            && file_name != "blueprints.toml"
            && file_name != "anatomy.toml"
        {
            // Department-level: shard io.toml
            let dept_name = file_name.replace(".toml", "");
            validate_department_level(base_dir, &dept_name, session, &mut report);
        }
    }

    report
}

/// Model-level: department io.toml файлы (`{dept_name}/io.toml`)
/// На model-level connections теперь хранят и `output_matrix` и `input_matrix`.
/// Поэтому на model-level:
///   - Пользовательские output-порты: prune если не в output_matrix ни одного connection
///   - Пользовательские input-порты: prune если не в input_matrix ни одного connection
///   - Стандартные "in"/"out": неуязвимы
fn validate_model_level(
    base_dir: &Path,
    session: &ProjectSession,
    report: &mut ValidationReport,
) {
    for zone_name in &session.zones {
        let io_path = base_dir.join(zone_name).join("io.toml");

        bevy::log::info!("🔍 [Validator/Model] Zone '{}' → checking {}", zone_name, io_path.display());

        if !io_path.exists() {
            bevy::log::info!("🔍 [Validator/Model] Not found, skipping.");
            continue;
        }

        // Собираем используемые порты из connections
        let mut used_outputs: HashSet<String> = HashSet::new();
        let mut used_inputs: HashSet<String> = HashSet::new();
        
        for &p in PROTECTED_PORTS { 
            used_outputs.insert(p.to_string()); 
            used_inputs.insert(p.to_string());
        }

        for (from_zone, from_port, to_zone, to_port) in &session.connections {
            if from_zone == zone_name {
                used_outputs.insert(from_port.clone());
            }
            if to_zone == zone_name {
                used_inputs.insert(to_port.clone());
            }
        }

        let Ok(content) = fs::read_to_string(&io_path) else { continue };
        let Ok(mut doc) = content.parse::<DocumentMut>() else { continue };

        let mut changed = false;
        changed |= sanitize_port_table(&mut doc, "input", zone_name, &used_inputs, report);
        changed |= sanitize_port_table(&mut doc, "output", zone_name, &used_outputs, report);

        if changed {
            if let Err(e) = fs::write(&io_path, doc.to_string()) {
                bevy::log::error!("❌ [Validator] Failed to write {}: {}", io_path.display(), e);
            } else {
                bevy::log::info!("🧹 [Validator] Sanitized dept io: {}", io_path.display());
            }
        }
    }
}

/// Department-level: shard io.toml файлы (`{dept}/{zone}/io.toml`)
fn validate_department_level(
    base_dir: &Path,
    dept_name: &str,
    session: &ProjectSession,
    report: &mut ValidationReport,
) {
    let dept_dir = base_dir.join(dept_name);
    if !dept_dir.exists() { return; }

    for zone_name in &session.zones {
        let io_path = dept_dir.join(zone_name).join("io.toml");
        if !io_path.exists() { continue; }

        let mut used_ports: HashSet<String> = HashSet::new();
        for &p in PROTECTED_PORTS { used_ports.insert(p.to_string()); }

        for (from_zone, from_port, to_zone, to_port) in &session.connections {
            if from_zone == zone_name { used_ports.insert(from_port.clone()); }
            if to_zone == zone_name   { used_ports.insert(to_port.clone()); }
        }

        let Ok(content) = fs::read_to_string(&io_path) else { continue };
        let Ok(mut doc) = content.parse::<DocumentMut>() else { continue };

        let mut changed = false;
        changed |= sanitize_port_table(&mut doc, "input", zone_name, &used_ports, report);
        changed |= sanitize_port_table(&mut doc, "output", zone_name, &used_ports, report);

        if changed {
            if let Err(e) = fs::write(&io_path, doc.to_string()) {
                bevy::log::error!("❌ [Validator] Failed to write {}: {}", io_path.display(), e);
            } else {
                bevy::log::info!("🧹 [Validator] Sanitized shard io: {}", io_path.display());
            }
        }
    }
}

/// Полная санитизация: dedup + orphan pruning (Hierarchical Matrix -> Pins)
fn sanitize_port_table(
    doc: &mut DocumentMut,
    array_key: &str,
    _target_zone: &str,
    used_ports: &HashSet<String>,
    report: &mut ValidationReport,
) -> bool {
    let mut matrices_to_remove = Vec::new();
    let mut changed = false;

    if let Some(matrices) = doc.get_mut(array_key).and_then(|i| i.as_array_of_tables_mut()) {
        for (m_idx, matrix) in matrices.iter_mut().enumerate() {
            let mut pins_to_remove = Vec::new();
            if let Some(pins) = matrix.get_mut("pin").and_then(|p| p.as_array_of_tables_mut()) {
                for (p_idx, pin) in pins.iter().enumerate() {
                    if let Some(name) = pin.get("name").and_then(|v| v.as_str()) {
                        let name_str = name.to_string();
                        if PROTECTED_PORTS.contains(&name_str.as_str()) {
                            continue;
                        }
                        if !used_ports.contains(&name_str) {
                            pins_to_remove.push(p_idx);
                        }
                    } else {
                        pins_to_remove.push(p_idx);
                    }
                }

                if !pins_to_remove.is_empty() {
                    changed = true;
                    report.pruned_ports += pins_to_remove.len();
                    for &idx in pins_to_remove.iter().rev() {
                        pins.remove(idx);
                    }
                }

                if pins.is_empty() {
                    matrices_to_remove.push(m_idx);
                }
            } else {
                // Матрица без пинов — нелегальна
                matrices_to_remove.push(m_idx);
            }
        }

        if !matrices_to_remove.is_empty() {
            changed = true;
            for &idx in matrices_to_remove.iter().rev() {
                matrices.remove(idx);
            }
        }
    }

    changed
}
