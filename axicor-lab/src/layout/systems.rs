use bevy::prelude::*;
use crate::layout::tree::{AreaNode, SplitDirection};
use crate::layout::interaction::Resizer;
use crate::panels::neuron_physics::populate_neuron_physics_panel;
use crate::theme::*;

#[derive(Component)]
pub struct ViewportContainer;

pub fn setup_layout(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 1,
            ..default()
        },
        ..default()
    });

    let root_node = AreaNode::initial_layout();
    
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        background_color: color_bg_root().into(),
        ..default()
    }).with_children(|parent| {
        spawn_area_node(parent, &root_node, &asset_server);
    });
}

fn spawn_area_node(parent: &mut ChildBuilder, node: &AreaNode, asset_server: &AssetServer) {
    match node {
        AreaNode::Split { direction, ratio, children } => {
            let flex_direction = match direction {
                SplitDirection::Horizontal => FlexDirection::Row,
                SplitDirection::Vertical => FlexDirection::Column,
            };

            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction,
                    ..default()
                },
                ..default()
            }).with_children(|split_parent| {
                // First child
                let first_child = split_parent.spawn(NodeBundle {
                    style: Style {
                        flex_basis: Val::Percent(ratio * 100.0),
                        flex_grow: 0.0,
                        ..default()
                    },
                    ..default()
                }).with_children(|child_parent| {
                    spawn_area_node(child_parent, &children.0, asset_server);
                }).id();

                // Interactive Resizer (The "Gap")
                let (w, h) = match direction {
                    SplitDirection::Horizontal => (Val::Px(4.0), Val::Percent(100.0)),
                    SplitDirection::Vertical => (Val::Percent(100.0), Val::Px(4.0)),
                };
                
                split_parent.spawn((
                    NodeBundle {
                        style: Style {
                            width: w,
                            height: h,
                            ..default()
                        },
                        background_color: Color::NONE.into(),
                        ..default()
                    },
                    Interaction::default(),
                    Resizer {
                        direction: *direction,
                        first_child,
                    },
                ));

                // Second child
                split_parent.spawn(NodeBundle {
                    style: Style {
                        flex_basis: Val::Percent((1.0 - ratio) * 100.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                    ..default()
                }).with_children(|child_parent| {
                    spawn_area_node(child_parent, &children.1, asset_server);
                });
            });
        }
        AreaNode::Leaf { name } => {
            let is_viewport = name == "Shard Canvas";
            let is_neuron_physics = name == "Neuron Physics";
            
            let mut entity = parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: if is_viewport { Color::NONE.into() } else { color_bg_panel().into() },
                ..default()
            });

            if is_viewport {
                entity.insert(ViewportContainer);
            }

            entity.with_children(|leaf_parent| {
                if is_neuron_physics {
                    populate_neuron_physics_panel(leaf_parent);
                } else {
                    leaf_parent.spawn(TextBundle::from_section(
                        name,
                        TextStyle {
                            font_size: 16.0,
                            color: color_text_main(),
                            ..default()
                        },
                    ));
                }
            });
        }
    }
}
