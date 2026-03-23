use bevy::prelude::*;
use crate::theme::*;

#[derive(Component)]
pub struct AxicorSlider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub fill_node: Entity,
    pub text_node: Entity,
    pub unit: String,
}

#[derive(Resource, Default)]
pub struct ActiveSlider(pub Option<Entity>);

pub fn spawn_slider(
    parent: &mut ChildBuilder,
    label: &str,
    min: f32,
    max: f32,
    default_val: f32,
    unit: &str,
) {
    let mut fill_node_id = Entity::PLACEHOLDER;
    let mut text_node_id = Entity::PLACEHOLDER;
    let mut track_id = Entity::PLACEHOLDER;

    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Px(40.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(10.0),
            padding: UiRect::horizontal(Val::Px(10.0)),
            ..default()
        },
        ..default()
    }).with_children(|row| {
        // Label
        row.spawn(TextBundle::from_section(
            format!("{}:", label),
            TextStyle {
                font_size: 14.0,
                color: color_text_dim(),
                ..default()
            },
        ).with_style(Style {
            width: Val::Px(70.0),
            ..default()
        }));

        // Track
        track_id = row.spawn((
            NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    height: Val::Px(6.0),
                    ..default()
                },
                background_color: color_border().into(),
                ..default()
            },
            Interaction::default(),
        )).with_children(|track| {
            // Fill
            let ratio = (default_val - min) / (max - min);
            fill_node_id = track.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(ratio * 100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: color_accent().into(),
                ..default()
            }).id();
        }).id();

        // Value Text
        text_node_id = row.spawn(TextBundle::from_section(
            format!("{:.1}{}", default_val, unit),
            TextStyle {
                font_size: 14.0,
                color: color_text_main(),
                ..default()
            },
        ).with_style(Style {
            width: Val::Px(80.0),
            justify_content: JustifyContent::FlexEnd,
            ..default()
        })).id();
    });

    let unit_string = unit.to_string();
    parent.add_command(move |world: &mut World| {
        world.entity_mut(track_id).insert(AxicorSlider {
            value: default_val,
            min,
            max,
            fill_node: fill_node_id,
            text_node: text_node_id,
            unit: unit_string,
        });
    });
}

pub fn slider_drag_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut active_slider: ResMut<ActiveSlider>,
    windows: Query<&Window>,
    mut interaction_query: Query<(Entity, &Interaction, &GlobalTransform, &Node, &mut AxicorSlider)>,
    mut style_query: Query<&mut Style>,
    mut text_query: Query<&mut Text>,
) {
    let window = windows.single();
    let Some(cursor_pos) = window.cursor_position() else { return; };

    if mouse_button.just_pressed(MouseButton::Left) {
        for (entity, interaction, _, _, _) in interaction_query.iter() {
            if *interaction == Interaction::Pressed {
                active_slider.0 = Some(entity);
                break;
            }
        }
    }

    if mouse_button.just_released(MouseButton::Left) {
        active_slider.0 = None;
    }

    if let Some(active_entity) = active_slider.0 {
        if let Ok((_, _, gt, node, mut slider)) = interaction_query.get_mut(active_entity) {
            let rect_center = gt.translation().truncate();
            let rect_size = node.size();
            
            // Bevy's screen-space coordinate system can be tricky.
            // For simple case, let's use window width/height.
            let ratio = ((cursor_pos.x - (window.width() / 2.0 + rect_center.x - rect_size.x / 2.0)) / rect_size.x).clamp(0.0, 1.0);
            slider.value = slider.min + ratio * (slider.max - slider.min);
            
            // Update fill
            if let Ok(mut style) = style_query.get_mut(slider.fill_node) {
                style.width = Val::Percent(ratio * 100.0);
            }
            
            // Update text
            if let Ok(mut text) = text_query.get_mut(slider.text_node) {
                text.sections[0].value = format!("{:.1}{}", slider.value, slider.unit);
            }
        }
    }
}
