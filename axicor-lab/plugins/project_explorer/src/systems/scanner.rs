use bevy::prelude::*;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::domain::{ProjectModel, ProjectNode, ProjectNodeType, ProjectStatus, GitStatus};

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
                    let mut sim_node_cold = ProjectNode {
                        id: extract_id(&sim_path, "model_id_v1").unwrap_or_else(|| name.clone()),
                        name: "simulation.toml".to_string(),
                        path: sim_path.clone(),
                        node_type: ProjectNodeType::Simulation,
                        git_status: GitStatus::Unmodified,
                        children: scan_departments(&entry.path(), false),
                    };

                    // Для Hot дерева всегда сканируем с использованием Overlay FS.
                    // Если изменений нет, overlay вернет те же сырые файлы, и AST совпадет (Unmodified).
                    let sim_node_hot = ProjectNode {
                        id: extract_id_overlay(&sim_path, "model_id_v1").unwrap_or_else(|| name.clone()),
                        name: "simulation.toml".to_string(),
                        path: sim_path.clone(),
                        node_type: ProjectNodeType::Simulation,
                        git_status: GitStatus::Unmodified,
                        children: scan_departments(&entry.path(), true),
                    };
                    
                    sim_node_cold.children = merge_nodes(sim_node_cold.children, sim_node_hot.children);
                    project.root_nodes.push(sim_node_cold);
                }

                new_projects.push(project);
            }
        }
    }

    cache.projects = new_projects;
}

fn scan_departments(model_path: &Path, is_hot: bool) -> Vec<ProjectNode> {
    let mut children = Vec::new();
    let sim_path = model_path.join("simulation.toml");
    
    // Если Hot — читаем через Overlay FS (fallback на cold). Иначе — строго cold.
    let content = if is_hot {
        layout_api::overlay_read_to_string(&sim_path).unwrap_or_default()
    } else {
        fs::read_to_string(&sim_path).unwrap_or_default()
    };
    if content.is_empty() { return children; }
    
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
                    git_status: GitStatus::Unmodified, // will be resolved in merge
                    children: Vec::new(),
                };

                // Сканируем шарды внутри бати
                // Для Hot проверки достаточно использовать overlay_read_to_string внутри scan_shards
                if is_hot || brain_path.exists() {
                    dept_node.children = scan_shards(model_path, &brain_path, is_hot);
                }
                children.push(dept_node);
            }
        }
    }
    children
}

fn scan_shards(model_path: &Path, brain_path: &Path, is_hot: bool) -> Vec<ProjectNode> {
    let mut children = Vec::new();
    
    let content = if is_hot {
        layout_api::overlay_read_to_string(brain_path).unwrap_or_default()
    } else {
        fs::read_to_string(brain_path).unwrap_or_default()
    };
    if content.is_empty() { return children; }
    
    if let Ok(toml_val) = content.parse::<toml::Value>() {
        if let Some(zones) = toml_val.get("zone").and_then(|v| v.as_array()) {
            for z in zones {
                let name = z.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let id = z.get("shard_id_v1").and_then(|v| v.get("id")).and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| name.to_string());

                let mut shard_files = Vec::new();
                let base_shard_dir = model_path.join(brain_path.file_name().unwrap().to_string_lossy().replace(".toml", "")).join(name);

                for file_name in &["shard.toml", "anatomy.toml", "blueprints.toml", "io.toml"] {
                    shard_files.push(ProjectNode {
                        id: format!("{}_{}", id, file_name),
                        name: file_name.to_string(),
                        path: base_shard_dir.join(file_name),
                        node_type: ProjectNodeType::File,
                        git_status: GitStatus::Unmodified,
                        children: Vec::new(),
                    });
                }

                // Шард - это папка, где лежит shard.toml
                let shard_toml_path = base_shard_dir.join("shard.toml");

                children.push(ProjectNode {
                    id,
                    name: name.to_string(),
                    path: shard_toml_path,
                    node_type: ProjectNodeType::Shard,
                    git_status: GitStatus::Unmodified,
                    children: shard_files,
                });
            }
        }
    } else {
        error!("❌ [Scanner] Failed to parse TOML at {:?}. Check for syntax errors.", brain_path);
    }
    children
}

fn extract_id(path: &Path, field: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    if let Ok(val) = content.parse::<toml::Value>() {
        val.get(field).and_then(|v| v.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string())
    } else { None }
}

fn extract_id_overlay(path: &Path, field: &str) -> Option<String> {
    let content = layout_api::overlay_read_to_string(path).ok()?;
    if let Ok(val) = content.parse::<toml::Value>() {
        val.get(field).and_then(|v| v.get("id")).and_then(|v| v.as_str()).map(|s| s.to_string())
    } else { None }
}

fn merge_nodes(cold: Vec<ProjectNode>, hot: Vec<ProjectNode>) -> Vec<ProjectNode> {
    let mut merged = Vec::new();
    let mut hot_map: HashMap<String, ProjectNode> = hot.into_iter().map(|n| (n.id.clone(), n)).collect();

    for mut c_node in cold {
        if let Some(h_node) = hot_map.remove(&c_node.id) {
            // Присутствует и там и там -> Unmodified, мержим детей
            c_node.git_status = GitStatus::Unmodified;
            c_node.children = merge_nodes(c_node.children, h_node.children);
            merged.push(c_node);
        } else {
            // Есть в Cold, но нет в Hot -> удален в Sandbox!
            c_node.git_status = GitStatus::Deleted;
            c_node.children = mark_all_deleted(c_node.children);
            merged.push(c_node);
        }
    }

    // Все что осталось в Hot, но чего не было в Cold -> Added!
    for (_, mut h_node) in hot_map {
        h_node.git_status = GitStatus::Added;
        h_node.children = mark_all_added(h_node.children);
        merged.push(h_node);
    }

    // Сортировка для порядка: сначала добавленные, затем обычные, затем удаленные (внутри групп по алфавиту)
    merged.sort_by(|a, b| {
        let type_ord = |status: &GitStatus| match status {
            GitStatus::Added => 0,
            GitStatus::Unmodified => 1,
            GitStatus::Deleted => 2,
        };
        type_ord(&a.git_status).cmp(&type_ord(&b.git_status)).then(a.name.cmp(&b.name))
    });

    merged
}

fn mark_all_deleted(nodes: Vec<ProjectNode>) -> Vec<ProjectNode> {
    nodes.into_iter().map(|mut n| {
        n.git_status = GitStatus::Deleted;
        n.children = mark_all_deleted(n.children);
        n
    }).collect()
}

fn mark_all_added(nodes: Vec<ProjectNode>) -> Vec<ProjectNode> {
    nodes.into_iter().map(|mut n| {
        n.git_status = GitStatus::Added;
        n.children = mark_all_added(n.children);
        n
    }).collect()
}
