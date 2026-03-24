use bevy::prelude::*;
use crate::widgets::slider::spawn_slider;
use crate::widgets::button::spawn_button;
use crate::theme::*;

pub fn populate_neuron_physics_panel(parent: &mut ChildBuilder) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            ..default()
        },
        ..default()
    }).with_children(|container| {
        container.spawn(TextBundle::from_section(
            "Neuron Parameters",
            TextStyle {
                font_size: TEXT_SIZE_TITLE,
                color: color_accent(),
                ..default()
            },
        ).with_style(Style {
            margin: UiRect::bottom(Val::Px(12.0)),
            ..default()
        }));

        spawn_slider(container, "V_rest", -90.0, -50.0, -70.0, "mV");
        spawn_slider(container, "V_th", -60.0, -30.0, -50.0, "mV");
        spawn_slider(container, "tau_m", 1.0, 50.0, 20.0, "ms");
        spawn_slider(container, "g_L", 0.01, 1.0, 0.1, "nS");

        // Spacer
        container.spawn(NodeBundle {
            style: Style {
                flex_grow: 1.0,
                ..default()
            },
            ..default()
        });

        // Apply Button
        container.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                padding: UiRect::top(Val::Px(16.0)),
                ..default()
            },
            ..default()
        }).with_children(|footer| {
            spawn_button(footer, "Apply Changes");
        });
    });
}
