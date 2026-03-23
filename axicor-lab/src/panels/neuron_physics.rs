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
            padding: UiRect::all(Val::Px(10.0)),
            row_gap: Val::Px(5.0),
            ..default()
        },
        ..default()
    }).with_children(|container| {
        container.spawn(TextBundle::from_section(
            "Neuron Physics",
            TextStyle {
                font_size: 18.0,
                color: color_text_main(),
                ..default()
            },
        ).with_style(Style {
            margin: UiRect::bottom(Val::Px(10.0)),
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
                padding: UiRect::vertical(Val::Px(10.0)),
                ..default()
            },
            ..default()
        }).with_children(|footer| {
            spawn_button(footer, "Apply Changes");
        });
    });
}
