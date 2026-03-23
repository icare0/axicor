use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AreaNode {
    Split {
        direction: SplitDirection,
        ratio: f32,
        children: (Box<AreaNode>, Box<AreaNode>),
    },
    Leaf {
        name: String,
    },
}

impl AreaNode {
    pub fn initial_layout() -> Self {
        // Root: Split(Horizontal, 0.85):
        //     Left: Existing 5-panel layout
        //     Right: Leaf "Simulation Control / Analytics"
        
        let core_layout = AreaNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.2,
            children: (
                Box::new(AreaNode::Leaf { name: "Project Explorer".to_string() }),
                Box::new(AreaNode::Split {
                    direction: SplitDirection::Vertical,
                    ratio: 0.5,
                    children: (
                        Box::new(AreaNode::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.3,
                            children: (
                                Box::new(AreaNode::Leaf { name: "Shard Canvas".to_string() }),
                                Box::new(AreaNode::Leaf { name: "Connectome Viewer".to_string() }),
                            ),
                        }),
                        Box::new(AreaNode::Split {
                            direction: SplitDirection::Horizontal,
                            ratio: 0.5,
                            children: (
                                Box::new(AreaNode::Leaf { name: "Neuron Physics".to_string() }),
                                Box::new(AreaNode::Leaf { name: "Synapse Physics".to_string() }),
                            ),
                        }),
                    ),
                }),
            ),
        };

        AreaNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.85,
            children: (
                Box::new(core_layout),
                Box::new(AreaNode::Leaf { name: "Simulation Control / Analytics".to_string() }),
            ),
        }
    }
}
