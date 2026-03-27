use bevy::prelude::*;
use bevy_egui::EguiContexts;
use layout_api::{AllocatedPanes, PluginInput, PluginWindow};

pub fn route_plugin_inputs_system(
    mut contexts: EguiContexts,
    allocated: Res<AllocatedPanes>,
    mut query: Query<(&PluginWindow, &mut PluginInput)>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    let pointer     = ctx.input(|i| i.pointer.clone());
    let scroll      = ctx.input(|i| i.raw_scroll_delta.y);
    let pointer_pos = pointer.interact_pos();

    for (window, mut input) in query.iter_mut() {
        let Some(rect) = allocated.rects.get(&window.plugin_id) else { continue };

        let is_hovered = pointer_pos.is_some_and(|p| rect.contains(p));

        // Кнопки: если hovered — прямой статус; если нет — только если уже были зажаты
        // (drag-out: зажал внутри, вытянул наружу — кнопка остаётся активной)
        input.is_primary_pressed   = pointer.primary_down()   && (is_hovered || input.is_primary_pressed);
        input.is_secondary_pressed = pointer.secondary_down() && (is_hovered || input.is_secondary_pressed);
        input.is_middle_pressed    = pointer.middle_down()    && (is_hovered || input.is_middle_pressed);

        if is_hovered {
            let pos = pointer_pos.unwrap(); // safe: is_hovered гарантирует Some
            input.local_cursor = Vec2::new(pos.x - rect.min.x, pos.y - rect.min.y);
            input.cursor_delta = Vec2::new(pointer.delta().x, pointer.delta().y);
            input.scroll_delta = scroll;
        } else {
            input.cursor_delta = Vec2::ZERO;
            input.scroll_delta = 0.0;
        }
    }
}