use bevy::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProjectStatus { Ready, Stale, Error }

#[derive(Clone, Debug)]
pub struct ProjectModel {
    pub name: String,
    pub status: ProjectStatus,
    pub dna_files: Vec<String>,
    pub shards: Vec<String>,
    pub is_bundle: bool,
}

#[derive(Resource, Default, Debug)]
pub struct ProjectFsCache {
    pub projects: Vec<ProjectModel>,
}
