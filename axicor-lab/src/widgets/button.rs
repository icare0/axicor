use bevy::prelude::*;
use crate::theme::*;

#[derive(Component)]
pub struct AxicorButton;

pub fn spawn_button(
    parent: &mut ChildBuilder,
    label: &str,
) -> Entity {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(120.0),
                height: Val::Px(32.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            border_color: color_border().into(),
            background_color: color_bg_panel().into(),
            ..default()
        },
        AxicorButton,
    )).with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font_size: 14.0,
                color: color_text_main(),
                ..default()
            },
        ));
    }).id()
}

pub fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<AxicorButton>),
    >,
) {
    for (interaction, mut bg_color, mut border_color) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = color_accent().into();
                *border_color = color_accent().into();
            }
            Interaction::Hovered => {
                *bg_color = color_border().into();
                *border_color = color_accent().into();
            }
            Interaction::None => {
                *bg_color = color_bg_panel().into();
                *border_color = color_border().into();
            }
        }
    }
}
