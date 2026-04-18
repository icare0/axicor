use crate::domain::{BrainTopologyGraph, CompileGraphEvent, NodeGraphUiState};
use bevy::prelude::*;
use std::fs;
use std::path::Path;

use super::validate;

pub fn compile_project_system(
    mut events: EventReader<CompileGraphEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
    ui_state_query: Query<&NodeGraphUiState>,
) {
    for _ev in events.read() {
        let Some(active_proj) = graph.active_project.clone() else {
            continue;
        };

        let base_dir = Path::new("Axicor-Models").join(&active_proj);
        let sandbox_dir = base_dir.join(".Sandbox");
        let autosave_dir = sandbox_dir.join(".tmp.autosave");
        let last_backup_dir = sandbox_dir.join(".tmp.last_backup");
        let old_backup_dir = sandbox_dir.join(".tmp.old_backup");

        info!(
            " [Compile] Starting transactional commit for '{}'...",
            active_proj
        );

        // [DOD FIX]
        if let Some(active_path) = graph.active_path.clone() {
            if let Some(session) = graph.sessions.get(&active_path) {
                let ui_state = ui_state_query.iter().next();
                if let Err(e) =
                    super::utils::flush_session_to_disk(&active_path, session, ui_state, true)
                {
                    error!("[ERROR] [Compile] Failed to flush layout to sandbox: {}", e);
                } else {
                    info!("[OK] [Compile] Visual layout auto-flushed to sandbox");
                }
            }
        }

        // 3.0   ( )
        if autosave_dir.exists() {
            let _ = fs::remove_dir_all(&old_backup_dir);

            if last_backup_dir.exists() {
                let _ = fs::rename(&last_backup_dir, &old_backup_dir);
            }

            // 3.2-3.3
            if let Err(e) = copy_dir_recursive(&autosave_dir, &base_dir) {
                error!(
                    "[ERROR] [Compile] Failed to apply sandbox to pure files: {}",
                    e
                );
                continue;
            }

            if let Err(e) = fs::rename(&autosave_dir, &last_backup_dir) {
                error!(
                    "[ERROR] [Compile] Failed to rotate autosave to last_backup: {}",
                    e
                );
                continue;
            }

            info!("[OK] [Compile] Sandbox applied to cold files. Backups rotated.");
        } else {
            info!(" [Compile] Sandbox is empty. Proceeding to validation only.");
        }

        // [DOD FIX] Post-Commit Validation:   io.toml  orphan-
        let report = validate::validate_project(&base_dir, &graph.sessions);
        if report.pruned_ports > 0 || report.deduped_ports > 0 {
            info!(
                " [Compile] Sanitized io.toml: {} orphan ports pruned, {} duplicates removed",
                report.pruned_ports, report.deduped_ports
            );
        }

        //    RAM
        for (_, session) in graph.sessions.iter_mut() {
            session.is_dirty = false;
        }

        info!("[OK] [Compile] Compilation complete for '{}'.", active_proj);
    }
}

///    (Overlay Apply)
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());

        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(&entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
