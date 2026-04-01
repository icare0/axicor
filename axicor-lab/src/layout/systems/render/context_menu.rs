use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{OpenContextMenuEvent, ContextMenuActionTriggeredEvent, MenuAction};
use crate::layout::ui::context_menu::ContextMenuState;

/// Система отрисовки контекстного меню на стороне Window Manager.
/// Плагины посылают OpenContextMenuEvent со своими MenuAction,
/// WM рендерит и роутит триггеры обратно через ContextMenuActionTriggeredEvent.
pub fn render_context_menu_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ContextMenuState>,
    mut open_events: EventReader<OpenContextMenuEvent>,
    mut trigger_writer: EventWriter<ContextMenuActionTriggeredEvent>,
) {
    // 1. Читаем новые события открытия
    let mut just_opened = false;
    for ev in open_events.read() {
        state.open = true;
        state.target_window = Some(ev.target_window);
        state.position = ev.position;
        state.actions = ev.actions.clone();

        // WM добавляет свои глобальные инвариантные действия
        state.actions.push(MenuAction {
            action_id: "wm.create_file".into(),
            label: "Create File (Global)".into(),
        });
        just_opened = true;
    }

    if !state.open {
        return;
    }

    let ctx = contexts.ctx_mut();
    let menu_id = egui::Id::new("wm_context_menu");

    // 2. Рендерим через egui::Area (Strict Tile Isolation — egui::Window запрещён)
    let actions_snapshot: Vec<_> = state.actions.iter().cloned().collect();
    let mut clicked_action: Option<String> = None;

    let area_resp = egui::Area::new(menu_id)
        .fixed_pos(state.position)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::menu(&ctx.style())
                .show(ui, |ui| {
                    for action in &actions_snapshot {
                        if ui.button(&action.label).clicked() {
                            clicked_action = Some(action.action_id.clone());
                        }
                    }
                });
        });

    // Обработка сигнала клика (вне borrow scope)
    if let Some(action_id) = clicked_action {
        if let Some(target) = state.target_window {
            trigger_writer.send(ContextMenuActionTriggeredEvent {
                target_window: target,
                action_id,
            });
        }
        state.open = false;
    }

    // 3. Закрытие при клике мимо (пропускаем кадр открытия)
    if !just_opened && ctx.input(|i| i.pointer.any_click()) {
        let menu_rect = area_resp.response.rect;
        if !menu_rect.contains(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
            state.open = false;
        }
    }
}
