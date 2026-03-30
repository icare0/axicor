use bevy::prelude::*;
use std::fs;
use std::path::Path;
use crate::domain::{ProjectModel, ProjectNode, ProjectNodeType, ProjectStatus};

pub fn fs_scanner_system(
    mut cache: ResMut<crate::domain::ProjectFsCache>,
) {
    let models_dir = Path::new("Genesis-Models");
    if !models_dir.exists() { return; }

    let mut new_projects = Vec::new();

    if let Ok(entries) = fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') { continue; }

                let mut project = ProjectModel {
                    name: format!("{} (Source)", name),
                    status: ProjectStatus::Ready,
                    root_nodes: Vec::new(),
                    is_bundle: false,
                };

                let sim_path = entry.path().join("simulation.toml");
                if sim_path.exists() {
                    let mut sim_node = ProjectNode {
                        id: extract_id(&sim_path, "model_id_v1").unwrap_or_else(|| name.clone()),
                        name: "simulation.toml".to_string(),
                        path: sim_path.clone(),
                        node_type: ProjectNodeType::Simulation,
                        children: Vec::new(),
                    };

                    // Сканируем департаменты внутри деда
                    scan_departments(&entry.path(), &mut sim_node.children);
                    project.root_nodes.push(sim_node);
                }

                new_projects.push(project);
            }
        }
    }

    cache.projects = new_projects;
}

fn scan_departments(model_path: &Path, children: &mut Vec<ProjectNode>) {
    let sim_path = model_path.join("simulation.toml");
    let Ok(content) = fs::read_to_string(&sim_path) else { return };
    
    // Парсим деда, чтобы найти батю
    if let Ok(toml_val) = content.parse::<toml::Value>() {
        if let Some(depts) = toml_val.get("department").and_then(|v| v.as_array()) {
            for d in depts {
                let name = d.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let config = d.get("config").and_then(|v| v.as_str()).unwrap_or("");
                let id = d.get("depart_id_v1").and_then(|v| v.get("id")).and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.to_string());

                let brain_path = model_path.join(config);
                let mut dept_node = ProjectNode {
                    id: id.clone(),
                    name: format!("{}.toml", name),
                    path: brain_path.clone(),
                    node_type: ProjectNodeType::Brain,
                    children: Vec::new(),
                };

                // Сканируем шарды внутри бати
                if brain_path.exists() {
                    scan_shards(model_path, &brain_path, &mut dept_node.children);
                }
                children.push(dept_node);
            }
        }
    }
}

fn scan_shards(model_path: &Path, brain_path: &Path, children: &mut Vec<ProjectNode>) {
    let Ok(content) = fs::read_to_string(brain_path) else { return };
    
    if let Ok(toml_val) = content.parse::<toml::Value>() {
        if let Some(zones) = toml_val.get("zone").and_then(|v| v.as_array()) {
            for z in zones {
                let name = z.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let id = z.get("shard_id_v1").and_then(|v| v.get("id")).and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.to_string());

                // Шард - это папка, где лежит shard.toml
                let shard_toml_path = model_path.join(brain_path.file_name().unwrap().to_string_lossy().replace(".toml", ""))
                    .join(name).join("shard.toml");

                children.push(ProjectNode {
                    id,
                    name: name.to_string(),
                    path: shard_toml_path,
                    node_type: ProjectNodeType::Shard,
                    children: Vec::new(),
                });
            }
        }
    } else {
        error!("❌ [Scanner] Failed to parse TOML at {:?}. Check for syntax errors (e.g. missing newlines between blocks).", brain_path);
    }
}

fn extract_id(path: &Path, field: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    if let Ok(val) = content.parse::<toml::Value>() {
        val.get(field).and_then(|v| v.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string())
    } else { None }
}
