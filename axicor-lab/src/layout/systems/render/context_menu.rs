use crate::layout::ui::context_menu::ContextMenuState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{ContextMenuActionTriggeredEvent, MenuAction, OpenContextMenuEvent};

///       Window Manager.
///   OpenContextMenuEvent   MenuAction,
/// WM       ContextMenuActionTriggeredEvent.
pub fn render_context_menu_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ContextMenuState>,
    mut open_events: EventReader<OpenContextMenuEvent>,
    mut trigger_writer: EventWriter<ContextMenuActionTriggeredEvent>,
) {
    // 1.
    let mut just_opened = false;
    for ev in open_events.read() {
        state.open = true;
        state.target_window = Some(ev.target_window);
        state.position = ev.position;
        state.actions = ev.actions.clone();

        // WM
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

    // 2.   egui::Area (Strict Tile Isolation  egui::Window )
    let actions_snapshot: Vec<_> = state.actions.iter().cloned().collect();
    let mut clicked_action: Option<String> = None;

    let area_resp = egui::Area::new(menu_id)
        .fixed_pos(state.position)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::menu(&ctx.style()).show(ui, |ui| {
                for action in &actions_snapshot {
                    if ui.button(&action.label).clicked() {
                        clicked_action = Some(action.action_id.clone());
                    }
                }
            });
        });

    //    ( borrow scope)
    if let Some(action_id) = clicked_action {
        if let Some(target) = state.target_window {
            trigger_writer.send(ContextMenuActionTriggeredEvent {
                target_window: target,
                action_id,
            });
        }
        state.open = false;
    }

    // 3.     (  )
    if !just_opened && ctx.input(|i| i.pointer.any_click()) {
        let menu_rect = area_resp.response.rect;
        if !menu_rect.contains(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
            state.open = false;
        }
    }
}
