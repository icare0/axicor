use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum PanelType {
    ProjectExplorer,
    ShardCanvas,
    ConnectomeViewer,
    NeuronPhysics,
    SynapsePhysics,
    Analytics,
}

impl PanelType {
    pub fn name(&self) -> &str {
        match self {
            PanelType::ProjectExplorer => "Project Explorer",
            PanelType::ShardCanvas => "Shard Canvas",
            PanelType::ConnectomeViewer => "Connectome Viewer",
            PanelType::NeuronPhysics => "Neuron Physics",
            PanelType::SynapsePhysics => "Synapse Physics",
            PanelType::Analytics => "Simulation Control / Analytics",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComputedRect {
    pub position: Vec2,
    pub size: Vec2,
}

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub panels: Vec<(u32, ComputedRect)>,
    pub resizers: Vec<(u32, ComputedRect, SplitDirection)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AreaNode {
    Split {
        id: u32,
        direction: SplitDirection,
        ratio: f32,
        children: (Box<AreaNode>, Box<AreaNode>),
    },
    Leaf {
        id: u32,
        panel_type: PanelType,
    },
}

#[derive(Resource, Clone)]
pub struct WorkspaceTree {
    pub root: AreaNode,
    pub next_id: u32,
    pub gap: f32,
}

impl WorkspaceTree {
    pub fn new() -> Self {
        WorkspaceTree {
            root: AreaNode::Leaf { id: 0, panel_type: PanelType::ShardCanvas },
            next_id: 1,
            gap: 4.0,
        }
    }

    pub fn compute_layout(&self, total_size: Vec2) -> LayoutResult {
        let mut result = LayoutResult { panels: Vec::new(), resizers: Vec::new() };
        self.compute_recursive(&self.root, Vec2::ZERO, total_size, &mut result);
        result
    }

    fn compute_recursive(&self, node: &AreaNode, pos: Vec2, size: Vec2, result: &mut LayoutResult) {
        match node {
            AreaNode::Leaf { id, .. } => {
                result.panels.push((*id, ComputedRect { position: pos, size }));
            }
            AreaNode::Split { id, direction, ratio, children } => {
                match direction {
                    SplitDirection::Horizontal => {
                        let w1 = (size.x - self.gap) * ratio;
                        let w2 = size.x - w1 - self.gap;
                        
                        result.resizers.push((*id, ComputedRect {
                            position: pos + Vec2::new(w1, 0.0),
                            size: Vec2::new(self.gap, size.y),
                        }, *direction));

                        self.compute_recursive(&children.0, pos, Vec2::new(w1, size.y), result);
                        self.compute_recursive(&children.1, pos + Vec2::new(w1 + self.gap, 0.0), Vec2::new(w2, size.y), result);
                    }
                    SplitDirection::Vertical => {
                        let h1 = (size.y - self.gap) * ratio;
                        let h2 = size.y - h1 - self.gap;

                        result.resizers.push((*id, ComputedRect {
                            position: pos + Vec2::new(0.0, h1),
                            size: Vec2::new(size.x, self.gap),
                        }, *direction));

                        self.compute_recursive(&children.0, pos, Vec2::new(size.x, h1), result);
                        self.compute_recursive(&children.1, pos + Vec2::new(0.0, h1 + self.gap), Vec2::new(size.x, h2), result);
                    }
                }
            }
        }
    }

    pub fn split_node(&mut self, target_id: u32, direction: SplitDirection) {
        fn find_and_split(node: &mut AreaNode, target_id: u32, direction: SplitDirection, next_id: &mut u32) -> bool {
            match node {
                AreaNode::Leaf { id, panel_type } if *id == target_id => {
                    let old_type = *panel_type;
                    let split_id = *next_id; *next_id += 1;
                    let id1 = *next_id; *next_id += 1;
                    let id2 = *next_id; *next_id += 1;
                    *node = AreaNode::Split {
                        id: split_id,
                        direction,
                        ratio: 0.5,
                        children: (
                            Box::new(AreaNode::Leaf { id: id1, panel_type: old_type }),
                            Box::new(AreaNode::Leaf { id: id2, panel_type: old_type }),
                        ),
                    };
                    true
                }
                AreaNode::Split { children, .. } => {
                    find_and_split(&mut children.0, target_id, direction, next_id) ||
                    find_and_split(&mut children.1, target_id, direction, next_id)
                }
                _ => false,
            }
        }
        find_and_split(&mut self.root, target_id, direction, &mut self.next_id);
    }

    pub fn close_node(&mut self, target_id: u32) {
        fn find_and_close(node: &mut AreaNode, target_id: u32) -> Option<AreaNode> {
            match node {
                AreaNode::Split { children, .. } => {
                    match (&*children.0, &*children.1) {
                        (AreaNode::Leaf { id, .. }, _) if *id == target_id => return Some(*children.1.clone()),
                        (_, AreaNode::Leaf { id, .. }) if *id == target_id => return Some(*children.0.clone()),
                        _ => {
                            if let Some(new) = find_and_close(&mut children.0, target_id) {
                                children.0 = Box::new(new);
                                return None;
                            }
                            if let Some(new) = find_and_close(&mut children.1, target_id) {
                                children.1 = Box::new(new);
                                return None;
                            }
                        }
                    }
                }
                _ => {}
            }
            None
        }
        if let Some(new_root) = find_and_close(&mut self.root, target_id) {
            self.root = new_root;
        }
    }

    pub fn set_ratio(&mut self, split_id: u32, new_ratio: f32) {
        fn find_and_set(node: &mut AreaNode, split_id: u32, new_ratio: f32) -> bool {
            match node {
                AreaNode::Split { id, ratio, children, .. } => {
                    if *id == split_id {
                        *ratio = new_ratio;
                        true
                    } else {
                        find_and_set(&mut children.0, split_id, new_ratio) ||
                        find_and_set(&mut children.1, split_id, new_ratio)
                    }
                }
                _ => false,
            }
        }
        find_and_set(&mut self.root, split_id, new_ratio);
    }
}
