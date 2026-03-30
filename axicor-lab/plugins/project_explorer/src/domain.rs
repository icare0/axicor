use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProjectStatus { Ready, Stale, Error }

#[derive(Clone, Debug, PartialEq)]
pub enum ProjectNodeType {
    Simulation, // "Дедушка": Законы мира, глобальный конфиг
    Brain,      // "Отец": Макро-топология, связи зон
    Shard,      // "Внук": Локальная архитектура зоны
    File,       // Прочие файлы
}

#[derive(Debug, Clone)]
pub struct ProjectNode {
    pub id: String, // NEW: Паспорт из TOML
    pub name: String,
    pub path: PathBuf,
    pub node_type: ProjectNodeType,
    pub children: Vec<ProjectNode>,
}


#[derive(Clone, Debug)]
pub struct ProjectModel {
    pub name: String,
    pub status: ProjectStatus,
    pub root_nodes: Vec<ProjectNode>, 
    pub is_bundle: bool,
}

#[derive(Resource, Default, Debug)]
pub struct ProjectFsCache {
    pub projects: Vec<ProjectModel>,
}

#[derive(Component, Default, Debug)]
pub struct ProjectExplorerState {
    pub active_file: Option<PathBuf>,
}
