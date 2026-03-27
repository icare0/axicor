use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::layout::domain::{WorkspaceState, OsWindowCommand, WindowDragState, TreeCommands};
use layout_api::{AllocatedPanes, WindowDragRequest, TopologyCache};
use crate::layout::behavior::PaneBehavior;
use crate::layout::overlay::draw_drag_intent_overlay;

// --- Палитра ---
const COLOR_BG:       egui::Color32 = egui::Color32::from_rgb(20, 20, 22);
const COLOR_TOPBAR:   egui::Color32 = egui::Color32::from_rgb(15, 15, 17);
const COLOR_TITLE:    egui::Color32 = egui::Color32::from_rgb(130, 130, 130);
const APP_TITLE:      &str          = "Axicor Lab v0.0.0";

pub fn render_workspace_system(
    mut contexts: EguiContexts,
    mut workspace: ResMut<WorkspaceState>,
    mut allocated_panes: ResMut<AllocatedPanes>,
    mut topology: ResMut<TopologyCache>,
    mut os_cmd: EventWriter<OsWindowCommand>,
    mut exit: EventWriter<bevy::app::AppExit>,
    drag_state: Res<WindowDragState>,
    mut drag_request: ResMut<WindowDragRequest>,
    mut tree_commands: ResMut<TreeCommands>, // <-- ДОБАВИТЬ СЮДА
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    apply_visuals(ctx);

    allocated_panes.rects.clear();
    topology.tiles.clear();

    render_top_bar(ctx, &mut os_cmd, &mut exit);

    let mut behavior = PaneBehavior {
        allocated_panes: &mut allocated_panes,
        topology: &mut topology,
        drag_request: &mut drag_request,
        tree_commands: &mut tree_commands, // <-- ДОБАВИТЬ СЮДА
    };

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(COLOR_BG).inner_margin(3.0))
        .show(ctx, |ui| {
            ui.scope(|ui| {
                apply_splitter_visuals(ui);
                workspace.tree.ui(&mut behavior, ui);
            });

            if drag_state.is_dragging {
                draw_drag_intent_overlay(ui, &drag_state, behavior.drag_request, &behavior.topology.tiles);
            }
        });
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn apply_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.window_fill                          = egui::Color32::from_rgb(30, 30, 32);
    v.panel_fill                           = COLOR_BG;
    v.widgets.noninteractive.bg_fill       = COLOR_BG;
    ctx.set_visuals(v);
}

fn apply_splitter_visuals(ui: &mut egui::Ui) {
    let hover_stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(38));
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);
    ui.visuals_mut().widgets.hovered.bg_stroke         = hover_stroke;
    ui.visuals_mut().widgets.active.bg_stroke          = hover_stroke;
}

fn render_top_bar(
    ctx: &egui::Context,
    os_cmd: &mut EventWriter<OsWindowCommand>,
    exit: &mut EventWriter<bevy::app::AppExit>,
) {
    egui::TopBottomPanel::top("axicor_top_bar")
        .frame(egui::Frame::none().fill(COLOR_TOPBAR).inner_margin(4.0))
        .show(ctx, |ui| {
            // DOD FIX: Унификация шрифтов и стилей меню под главный дизайн-код
            ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            ui.visuals_mut().widgets.inactive.fg_stroke.color = COLOR_TITLE;
            ui.visuals_mut().widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 45, 50);

            ui.horizontal(|ui| {
                // DOD FIX: Кликабельный квадрат логотипа (16x16, отступы берутся из inner_margin(4.0))
                let logo_size = 16.0;
                let (logo_rect, logo_resp) = ui.allocate_exact_size(egui::vec2(logo_size, logo_size), egui::Sense::click());
                let logo_color = if logo_resp.hovered() { egui::Color32::from_rgb(100, 100, 105) } else { egui::Color32::from_rgb(60, 60, 65) };
                ui.painter().rect_filled(logo_rect, 2.0, logo_color);
                
                if logo_resp.clicked() {
                    info!("[WM] Logo clicked");
                }

                // Тройной отступ вправо (базовый margin 4.0 * 3)
                ui.add_space(12.0);

                ui.menu_button("File", |_| {});
                ui.menu_button("View", |_| {});
                ui.menu_button("Settings", |_| {});

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(" ✕ ").frame(false)).clicked() { exit.send(bevy::app::AppExit); }
                    if ui.add(egui::Button::new(" 🗖 ").frame(false)).clicked() { os_cmd.send(OsWindowCommand::Maximize); }
                    if ui.add(egui::Button::new(" 🗕 ").frame(false)).clicked() { os_cmd.send(OsWindowCommand::Minimize); }

                    let rect = ui.available_rect_before_wrap();
                    if ui.interact(rect, ui.id().with("drag_area"), egui::Sense::drag()).drag_started() {
                        os_cmd.send(OsWindowCommand::Drag);
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                        APP_TITLE, egui::FontId::proportional(14.0), COLOR_TITLE);
                });
            });
        });
}