use bevy::prelude::*;
use crate::domain::{ProjectFsCache, ProjectModel, ProjectStatus};

pub fn fs_scanner_system(
    time: Res<Time>,
    mut cache: ResMut<ProjectFsCache>,
    mut timer: Local<f32>,
) {
    *timer += time.delta_seconds();
    if *timer < 1.0 { return; }
    *timer = 0.0;

    let Ok(entries) = std::fs::read_dir("Genesis-Models") else { return; };
    let mut projects = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_stem().expect("Invalid project path").to_string_lossy().to_string();

        if path.is_file() {
            if path.extension().is_some_and(|ext| ext == "axic") {
                let mut dna_files = Vec::new();
                let mut shards = Vec::new();

                if let Some(archive) = genesis_core::vfs::AxicArchive::open(&path) {
                    for file_path in archive.toc.keys() {
                        if file_path == "brain.toml" {
                            dna_files.push(file_path.clone());
                        }
                        if file_path.starts_with("baked/") && file_path.ends_with("/shard.pos") {
                            let parts: Vec<&str> = file_path.split('/').collect();
                            // DOD FIX: parts="baked", parts[1]="ZoneName", parts[2]="shard.pos"
                            if parts.len() >= 3 {
                                shards.push(parts[1].to_string());
                            }
                        }
                    }

                    if !dna_files.is_empty() {
                        shards.sort_unstable();
                        shards.dedup();

                        projects.push(ProjectModel {
                            name,
                            status: ProjectStatus::Ready,
                            dna_files,
                            shards,
                            is_bundle: true,
                        });
                    }
                }
            }
        } else if path.is_dir() {
            // DOD FIX: Игнорируем временные папки SRAM распаковки ноды
            if name.ends_with(".axic.mem") || name == "baked" {
                continue;
            }

            if path.join("brain.toml").exists() {
                // Динамически сканируем все TOML файлы в папке
                let mut dna_files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let file_path = entry.path();
                        if file_path.is_file() && file_path.extension().map_or(false, |e| e == "toml") {
                            if let Some(fname) = file_path.file_name() {
                                dna_files.push(fname.to_string_lossy().into_owned());
                            }
                        }
                    }
                }
                dna_files.sort();

                projects.push(ProjectModel {
                    // DOD FIX: Добавляем суффикс, чтобы избежать ID Collision в egui
                    // с одноименными скомпилированными .axic архивами!
                    name: format!("{} (Source)", name),
                    status: ProjectStatus::Ready,
                    dna_files,
                    shards: Vec::new(), 
                    is_bundle: false,
                });
            }
        }
    }
    
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    cache.projects = projects;
}
