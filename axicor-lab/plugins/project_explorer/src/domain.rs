use bevy::prelude::*;
pub use layout_api::{ProjectStatus, GitStatus, ProjectNodeType, ProjectNode, ProjectModel, ProjectFsCache};
use std::path::PathBuf;

#[derive(Component, Default, Debug)]
pub struct ProjectExplorerState {
    pub active_file: Option<PathBuf>,
}
