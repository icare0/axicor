use bevy::prelude::*;
use std::path::Path;
use layout_api::{ProjectFsCache, ProjectModel, ProjectStatus};

pub struct ProjectExplorerPlugin;

impl Plugin for ProjectExplorerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectFsCache>()
           .add_systems(Update, fs_scanner_system);
    }
}

fn fs_scanner_system(
    mut cache: ResMut<ProjectFsCache>,
    mut timer: Local<f32>,
    time: Res<Time>,
) {
    *timer += time.delta_seconds();
    if *timer < 1.0 { return; }
    *timer = 0.0;

    let mut projects = Vec::new();
    let base_path = Path::new("Genesis-Models");

    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                let brain_toml = path.join("brain.toml");
                if brain_toml.exists() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let dna_files = vec!["brain.toml".to_string()];
                    let mut shards = Vec::new();
                    let mut status = ProjectStatus::NeedsBake;

                    // Scan compiled shards
                    let baked_dir = path.join("baked");
                    if baked_dir.exists() {
                        status = ProjectStatus::Ready;
                        if let Ok(baked_entries) = std::fs::read_dir(&baked_dir) {
                            for b_entry in baked_entries.filter_map(Result::ok) {
                                if b_entry.path().is_dir() {
                                    shards.push(b_entry.file_name().to_string_lossy().into_owned());
                                }
                            }
                        }
                    }

                    projects.push(ProjectModel { name, status, dna_files, shards });
                }
            }
        }
    }
    
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    cache.projects = projects;
}
