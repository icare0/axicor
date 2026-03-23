use bevy::prelude::*;
use crate::layout::tree::SplitDirection;
use crate::theme::*;

#[derive(Component)]
pub struct Resizer {
    pub direction: SplitDirection,
    pub first_child: Entity,
}

#[derive(Resource, Default)]
pub struct DragState {
    pub active_resizer: Option<Entity>,
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
    interaction_query: Query<(Entity, &Interaction, &Resizer)>,
    mut style_query: Query<&mut Style>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return; };

    if mouse_button.just_pressed(MouseButton::Left) {
        for (entity, interaction, _) in interaction_query.iter() {
            if *interaction == Interaction::Pressed {
                drag_state.active_resizer = Some(entity);
                break;
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        drag_state.active_resizer = None;
    }

    if let Some(resizer_entity) = drag_state.active_resizer {
        if let Ok((_, _, resizer)) = interaction_query.get(resizer_entity) {
            if let Ok(mut style) = style_query.get_mut(resizer.first_child) {
                let (pos, total) = match resizer.direction {
                    SplitDirection::Horizontal => (cursor_pos.x, window.width()),
                    SplitDirection::Vertical => (cursor_pos.y, window.height()),
                };
                
                let ratio = (pos / total).clamp(0.05, 0.95);
                style.flex_basis = Val::Percent(ratio * 100.0);
            }
        }
    }
}
