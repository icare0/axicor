use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{OpenContextMenuEvent, ContextMenuActionTriggeredEvent, MenuAction};

#[derive(Resource, Default)]
pub struct ContextMenuState {
    pub open: bool,
    pub target_window: Option<Entity>,
    pub position: egui::Pos2,
    pub actions: Vec<MenuAction>,
}

/// Context menu implementation (WM Side)
pub fn context_menu_ui_system(
    mut contexts: EguiContexts,
    mut state: ResMut<ContextMenuState>,
    mut open_events: EventReader<OpenContextMenuEvent>,
    mut trigger_writer: EventWriter<ContextMenuActionTriggeredEvent>,
) {
    let ctx = contexts.ctx_mut();
    let mut opened_this_frame = false;

    // 1. Process Open Events
    for ev in open_events.read() {
        state.open = true;
        opened_this_frame = true;
        state.target_window = Some(ev.target_window);
        state.position = ev.position;
        state.actions = ev.actions.clone();

        // 2. Global WM Actions (Always Present)
        state.actions.push(MenuAction { 
            action_id: "wm.create_file".into(), 
            label: " Create File".into() 
        });
    }

    if !state.open {
        return;
    }

    let mut should_close = false;
    let area_id = egui::Id::new("axicor_unified_menu");

    // 3. Render Menu (Order::Foreground)
    egui::Area::new(area_id)
        .fixed_pos(state.position)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_max_width(200.0);
            
            let frame = egui::Frame::menu(ui.style())
                .fill(egui::Color32::from_rgb(25, 25, 27))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 65)))
                .rounding(6.0)
                .inner_margin(egui::Margin::symmetric(8.0, 10.0));

            frame.show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);

                    for action in &state.actions {
                        let btn = egui::Button::new(&action.label)
                            .fill(egui::Color32::TRANSPARENT);
                        
                        if ui.add(btn).clicked() {
                            // 4. Dispatch Event (Intent Routing)
                            if let Some(target) = state.target_window {
                                trigger_writer.send(ContextMenuActionTriggeredEvent {
                                    target_window: target,
                                    action_id: action.action_id.clone(),
                                });
                            }
                            should_close = true;
                        }
                    }
                });

                // Auto-close on outside click
                if !opened_this_frame && ui.input(|i| i.pointer.any_click()) && !ui.rect_contains_pointer(ui.max_rect()) {
                    should_close = true;
                }
            });
        });

    if should_close {
        state.open = false;
    }
}
