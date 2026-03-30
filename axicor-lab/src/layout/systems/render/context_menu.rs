use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{OpenContextMenuEvent, ContextMenuActionTriggeredEvent, MenuAction};
use crate::layout::domain::ContextMenuState;

/// Система отрисовки контекстного меню на стороне Window Manager
pub fn render_context_menu_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ContextMenuState>,
    mut open_events: EventReader<OpenContextMenuEvent>,
    mut trigger_writer: EventWriter<ContextMenuActionTriggeredEvent>,
) {
    // 1. Читаем новые события открытия
    for ev in open_events.read() {
        state.open = true;
        state.target_window = Some(ev.target_window);
        state.position = ev.position;
        state.actions = ev.actions.clone();

        // 2. WM добавляет свои глобальные инвариантные действия
        state.actions.push(MenuAction { 
            action_id: "wm.create_file".into(), 
            label: "Create File (Global)".into() 
        });
    }

    if !state.open {
        return;
    }

    let ctx = contexts.ctx_mut();
    
    // 3. Рендерим egui popup/window поверх всего
    egui::Window::new("ContextMenu")
        .fixed_pos(state.position)
        .title_bar(false)
        .resizable(false)
        .frame(egui::Frame::menu(&ctx.style()))
        .show(ctx, |ui| {
            for action in &state.actions {
                if ui.button(&action.label).clicked() {
                    // 4. Генерируем триггер намерения
                    if let Some(target) = state.target_window {
                        trigger_writer.send(ContextMenuActionTriggeredEvent {
                            target_window: target,
                            action_id: action.action_id.clone(),
                        });
                    }
                    state.open = false;
                }
            }

            // Закрытие при клике мимо
            if !opened_this_frame && ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(ui.max_rect()) {
                state.open = false;
            }
        });
}
   state.open = false;
            }
        });
}
