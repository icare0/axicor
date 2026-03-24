use bevy::prelude::*;
use crate::layout::tree::{SplitDirection, WorkspaceTree, ComputedRect};
use crate::layout::systems::{PanelButton, PanelAction};
use crate::theme::*;

#[derive(Component)]
pub struct Resizer {
    pub direction: SplitDirection,
    pub split_id: u32,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub active_resizer: Option<ResizerInfo>,
}

pub struct ResizerInfo {
    pub split_id: u32,
    pub direction: SplitDirection,
    pub parent_rect: ComputedRect,
}

pub fn border_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<Resizer>, Changed<Interaction>),
    >,
) {
    for (interaction, mut color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Hovered | Interaction::Pressed => {
                *color = color_border().into();
            }
            Interaction::None => {
                *color = Color::NONE.into();
            }
        }
    }
}

pub fn border_drag_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<DragState>,
    mut tree: ResMut<WorkspaceTree>,
    interaction_query: Query<(&Interaction, &Resizer)>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return; };

    // Start drag
    if mouse_button.just_pressed(MouseButton::Left) {
        for (interaction, resizer) in interaction_query.iter() {
            if *interaction == Interaction::Pressed {
                let window_size = Vec2::new(window.width(), window.height());
                
                fn find_split_rect(node: &crate::layout::tree::AreaNode, target_id: u32, pos: Vec2, size: Vec2, gap: f32) -> Option<ComputedRect> {
                    match node {
                        crate::layout::tree::AreaNode::Split { id, direction, ratio, children } => {
                            if *id == target_id {
                                return Some(ComputedRect { position: pos, size });
                            }
                            let w1 = (size.x - gap) * ratio;
                            let h1 = (size.y - gap) * ratio;
                            find_split_rect(&children.0, target_id, pos, match direction {
                                SplitDirection::Horizontal => Vec2::new(w1, size.y),
                                SplitDirection::Vertical => Vec2::new(size.x, h1),
                            }, gap).or_else(|| {
                                find_split_rect(&children.1, target_id, match direction {
                                    SplitDirection::Horizontal => pos + Vec2::new(w1 + gap, 0.0),
                                    SplitDirection::Vertical => pos + Vec2::new(0.0, h1 + gap),
                                }, match direction {
                                    SplitDirection::Horizontal => Vec2::new(size.x - w1 - gap, size.y),
                                    SplitDirection::Vertical => Vec2::new(size.x, size.y - h1 - gap),
                                }, gap)
                            })
                        }
                        _ => None,
                    }
                }

                if let Some(parent_rect) = find_split_rect(&tree.root, resizer.split_id, Vec2::ZERO, window_size, tree.gap) {
                    drag_state.active_resizer = Some(ResizerInfo {
                        split_id: resizer.split_id,
                        direction: resizer.direction,
                        parent_rect,
                    });
                    break;
                }
            }
        }
    }

    // Stop drag
    if mouse_button.just_released(MouseButton::Left) {
        drag_state.active_resizer = None;
    }

    // Dragging
    if let Some(ref info) = drag_state.active_resizer {
        let rect = &info.parent_rect;
        
        let new_ratio = match info.direction {
            SplitDirection::Horizontal => {
                let available_w = rect.size.x - tree.gap;
                ((cursor_pos.x - rect.position.x) / available_w).clamp(0.05, 0.95)
            }
            SplitDirection::Vertical => {
                let available_h = rect.size.y - tree.gap;
                ((cursor_pos.y - rect.position.y) / available_h).clamp(0.05, 0.95)
            }
        };

        tree.set_ratio(info.split_id, new_ratio);
    }
}

pub fn panel_control_system(
    mut tree: ResMut<WorkspaceTree>,
    interaction_query: Query<(&Interaction, &PanelButton), Changed<Interaction>>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button.action {
                PanelAction::SplitVertical => {
                    tree.split_node(button.id, SplitDirection::Horizontal);
                }
                PanelAction::SplitHorizontal => {
                    tree.split_node(button.id, SplitDirection::Vertical);
                }
                PanelAction::Close => {
                    tree.close_node(button.id);
                }
            }
        }
    }
}
