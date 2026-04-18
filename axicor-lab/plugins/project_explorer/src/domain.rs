use bevy::prelude::*;
pub use layout_api::{
    GitStatus, ProjectFsCache, ProjectModel, ProjectNode, ProjectNodeType, ProjectStatus,
};
use std::path::PathBuf;

#[derive(Component, Default, Debug)]
pub struct ProjectExplorerState {
    pub active_file: Option<PathBuf>,
}
