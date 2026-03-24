use bevy::prelude::*;
use crate::layout::tree::{AreaNode, WorkspaceTree, PanelType};
use crate::layout::interaction::Resizer;
use crate::panels::neuron_physics::populate_neuron_physics_panel;
use crate::theme::*;
use crate::viewport::mask::CornerMaskMaterial;

#[derive(Component)]
pub struct ViewportContainer;

#[derive(Component)]
pub struct WorkspaceRoot;

#[derive(Component, Clone, Copy)]
pub enum PanelAction {
    SplitVertical,
    SplitHorizontal,
    Close,
}

#[derive(Component)]
pub struct PanelButton {
    pub id: u32,
    pub action: PanelAction,
}

#[derive(Component)]
pub struct PanelContainer {
    pub id: u32,
}

#[derive(Component)]
pub struct ResizerContainer {
    pub split_id: u32,
}

pub fn setup_layout(
    mut commands: Commands, 
    tree: Res<WorkspaceTree>, 
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CornerMaskMaterial>>,
) {
    commands.spawn(Camera2dBundle {
        camera: Camera {
            order: 1,
            ..default()
        },
        ..default()
    });

    spawn_workspace(&mut commands, &tree, &asset_server, &mut materials);
}

pub fn spawn_workspace(
    commands: &mut Commands, 
    tree: &WorkspaceTree, 
    asset_server: &AssetServer,
    materials: &mut Assets<CornerMaskMaterial>,
) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: color_bg_root().into(),
            ..default()
        },
        WorkspaceRoot,
    )).with_children(|parent| {
        spawn_nodes_recursive(parent, &tree.root, asset_server, materials);
    });
}

fn spawn_nodes_recursive(
    parent: &mut ChildBuilder, 
    node: &AreaNode, 
    asset_server: &AssetServer,
    materials: &mut Assets<CornerMaskMaterial>,
) {
    match node {
        AreaNode::Split { id, direction, children, .. } => {
            spawn_nodes_recursive(parent, &children.0, asset_server, materials);
            
            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    background_color: Color::NONE.into(),
                    ..default()
                },
                Interaction::default(),
                Resizer {
                    direction: *direction,
                    split_id: *id,
                },
                ResizerContainer { split_id: *id },
            ));

            spawn_nodes_recursive(parent, &children.1, asset_server, materials);
        }
        AreaNode::Leaf { id, panel_type } => {
            let is_viewport = *panel_type == PanelType::ShardCanvas;
            let is_neuron_physics = *panel_type == PanelType::NeuronPhysics;
            let name = panel_type.name();
            let node_id = *id;

            parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    background_color: Color::NONE.into(),
                    ..default()
                },
                PanelContainer { id: node_id },
            )).with_children(|outer| {
                outer.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    background_color: color_bg_panel().into(),
                    ..default()
                }).with_children(|inner| {
                    // Header Area (Integrated Zone)
                    inner.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(PANEL_HEADER_HEIGHT),
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(10.0)),
                            justify_content: JustifyContent::SpaceBetween,
                            border: UiRect::bottom(Val::Px(1.0)),
                            ..default()
                        },
                        border_color: color_border().into(),
                        background_color: Color::NONE.into(),
                        ..default()
                    }).with_children(|header| {
                        header.spawn(TextBundle::from_section(
                            name.to_uppercase(),
                            TextStyle {
                                font_size: TEXT_SIZE_HEADER,
                                color: color_text_dim(),
                                ..default()
                            },
                        ));

                        header.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(4.0),
                                ..default()
                            },
                            ..default()
                        }).with_children(|btns| {
                            spawn_control_button(btns, "|", node_id, PanelAction::SplitVertical);
                            spawn_control_button(btns, "-", node_id, PanelAction::SplitHorizontal);
                            spawn_control_button(btns, "X", node_id, PanelAction::Close);
                        });
                    });

                    // Content Area
                    let mut content = inner.spawn(NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(10.0)),
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        background_color: Color::NONE.into(),
                        ..default()
                    });

                    if is_viewport {
                        content.insert(ViewportContainer);
                    }

                    content.with_children(|content_parent| {
                        if is_neuron_physics {
                            populate_neuron_physics_panel(content_parent);
                        } else if !is_viewport {
                            content_parent.spawn(TextBundle::from_section(
                                name,
                                TextStyle {
                                    font_size: TEXT_SIZE_BODY,
                                    color: color_text_main(),
                                    ..default()
                                },
                            ));
                        }
                    });
                });

                spawn_panel_masks(outer, materials);
            });
        }
    }
}

fn spawn_panel_masks(parent: &mut ChildBuilder, materials: &mut Assets<CornerMaskMaterial>) {
    let radius = PANEL_BORDER_RADIUS;
    let color = color_bg_root();

    // Corners
    let positions = [
        (UiRect::new(Val::Px(0.), Val::Auto, Val::Px(0.), Val::Auto), Vec2::new(1., 1.)), // TL
        (UiRect::new(Val::Auto, Val::Px(0.), Val::Px(0.), Val::Auto), Vec2::new(0., 1.)), // TR
        (UiRect::new(Val::Px(0.), Val::Auto, Val::Auto, Val::Px(0.)), Vec2::new(1., 0.)), // BL
        (UiRect::new(Val::Auto, Val::Px(0.), Val::Auto, Val::Px(0.)), Vec2::new(0., 0.)), // BR
    ];

    for (style_rect, pivot) in positions {
        parent.spawn(MaterialNodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: style_rect.left,
                right: style_rect.right,
                top: style_rect.top,
                bottom: style_rect.bottom,
                width: Val::Px(radius),
                height: Val::Px(radius),
                ..default()
            },
            z_index: ZIndex::Local(10),
            material: materials.add(CornerMaskMaterial { color, pivot }),
            ..default()
        });
    }
}

fn spawn_control_button(parent: &mut ChildBuilder, label: &str, id: u32, action: PanelAction) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(18.0),
                height: Val::Px(18.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::NONE.into(),
            ..default()
        },
        PanelButton { id, action },
    )).with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size: 12.0, color: color_text_dim(), ..default() },
        ));
    });
}

pub fn sync_workspace_rects(
    tree: Res<WorkspaceTree>,
    windows: Query<&Window>,
    mut panel_query: Query<(&PanelContainer, &mut Style), Without<ResizerContainer>>,
    mut resizer_query: Query<(&ResizerContainer, &mut Style), Without<PanelContainer>>,
) {
    let window = windows.single();
    let total_size = Vec2::new(window.width(), window.height());
    let layout = tree.compute_layout(total_size);

    for (id, rect) in layout.panels {
        for (panel, mut style) in panel_query.iter_mut() {
            if panel.id == id {
                style.left = Val::Px(rect.position.x);
                style.top = Val::Px(rect.position.y);
                style.width = Val::Px(rect.size.x);
                style.height = Val::Px(rect.size.y);
            }
        }
    }

    for (split_id, rect, _dir) in layout.resizers {
        for (resizer, mut style) in resizer_query.iter_mut() {
            if resizer.split_id == split_id {
                style.left = Val::Px(rect.position.x);
                style.top = Val::Px(rect.position.y);
                style.width = Val::Px(rect.size.x);
                style.height = Val::Px(rect.size.y);
            }
        }
    }
}

pub fn workspace_rebuild_system(
    mut commands: Commands,
    tree: Res<WorkspaceTree>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<CornerMaskMaterial>>,
    root_query: Query<Entity, With<WorkspaceRoot>>,
) {
    if tree.is_changed() {
        for entity in root_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        spawn_workspace(&mut commands, &tree, &asset_server, &mut materials);
    }
}
