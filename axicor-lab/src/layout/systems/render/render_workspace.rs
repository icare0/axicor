use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use crate::layout::domain::{WorkspaceState, OsWindowCommand, WindowDragState, TreeCommands, SaveDefaultLayoutEvent};
use layout_api::{AllocatedPanes, WindowDragRequest, TopologyCache, CreateNewModelEvent};
use crate::layout::behavior::PaneBehavior;

// --- Visual constants ---
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
    mut tree_commands: ResMut<TreeCommands>,
    mut create_model_ev: EventWriter<CreateNewModelEvent>,
    mut save_layout_ev: EventWriter<SaveDefaultLayoutEvent>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    apply_visuals(ctx);

    allocated_panes.rects.clear();
    topology.tiles.clear();

    render_top_bar(ctx, &mut workspace, &mut os_cmd, &mut exit, &mut create_model_ev, &mut save_layout_ev);

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(COLOR_BG))
        .show(ctx, |ui| {
            let active_ws = workspace.active_workspace.clone();
            if let Some(tree) = workspace.trees.get_mut(&active_ws) {
                let mut behavior = PaneBehavior {
                    allocated_panes: &mut allocated_panes,
                    topology: &mut topology,
                    drag_request: &mut drag_request,
                    tree_commands: &mut tree_commands,
                };
                tree.ui(&mut behavior, ui);
            }

            // [DOD FIX]        DND.
            if drag_request.active || drag_state.is_dragging {
                crate::layout::overlay::draw_drag_intent_overlay(
                    ui, 
                    &drag_state, 
                    &drag_request, 
                    &topology.tiles
                );
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

fn render_top_bar(
    ctx: &egui::Context,
    workspace: &mut WorkspaceState,
    os_cmd: &mut EventWriter<OsWindowCommand>,
    exit: &mut EventWriter<bevy::app::AppExit>,
    create_model_ev: &mut EventWriter<CreateNewModelEvent>,
    save_layout_ev: &mut EventWriter<SaveDefaultLayoutEvent>,
) {
    egui::TopBottomPanel::top("axicor_top_bar")
        .frame(egui::Frame::none().fill(COLOR_TOPBAR).inner_margin(4.0))
        .show(ctx, |ui| {
            // DOD FIX: Custom font styling for top bar buttons
            ui.style_mut().text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            ui.visuals_mut().widgets.inactive.fg_stroke.color = COLOR_TITLE;
            ui.visuals_mut().widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);
            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 45, 50);

            ui.horizontal(|ui| {
                // App Logo
                let logo_size = 16.0;
                let (logo_rect, logo_resp) = ui.allocate_exact_size(egui::vec2(logo_size, logo_size), egui::Sense::click());
                let logo_color = if logo_resp.hovered() { egui::Color32::from_rgb(100, 100, 105) } else { egui::Color32::from_rgb(60, 60, 65) };
                ui.painter().rect_filled(logo_rect, 2.0, logo_color);

                if logo_resp.clicked() {
                    info!("[WM] Logo clicked");
                }

                ui.add_space(12.0);

                // 1. Main Menus (File, View, etc.)
                ui.menu_button("File", |ui| {
                    if ui.button("Create Model").clicked() {
                        create_model_ev.send(CreateNewModelEvent {
                            model_name: "Untitled_Model".to_string(),
                        });
                        ui.close_menu();
                    }
                    if ui.button("Save Default Layout").clicked() {
                        save_layout_ev.send(SaveDefaultLayoutEvent);
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |_| {});
                ui.menu_button("Settings", |_| {});

                ui.add_space(8.0);
                ui.painter().vline(ui.cursor().min.x, ui.max_rect().y_range(), egui::Stroke::new(1.0, egui::Color32::from_rgb(40, 40, 45)));
                ui.add_space(8.0);

                // 2. Workspace Tabs
                let mut tab_to_remove = None;
                let mut new_active = None;
                
                let order = workspace.workspace_order.clone();
                for ws in order {
                    let is_active = workspace.active_workspace == ws;

                    //   
                    if workspace.renaming_workspace.as_ref() == Some(&ws) {
                        let res = ui.add(egui::TextEdit::singleline(&mut workspace.rename_buffer).desired_width(80.0));
                        if res.lost_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                workspace.renaming_workspace = None;
                            } else {
                                let new_name = workspace.rename_buffer.trim().to_string();
                                if !new_name.is_empty() && new_name != ws && !workspace.trees.contains_key(&new_name) {
                                    if let Some(tree) = workspace.trees.remove(&ws) {
                                        workspace.trees.insert(new_name.clone(), tree);
                                    }
                                    if let Some(pos) = workspace.workspace_order.iter().position(|x| x == &ws) {
                                        workspace.workspace_order[pos] = new_name.clone();
                                    }
                                    if is_active {
                                        workspace.active_workspace = new_name.clone();
                                    }
                                }
                                workspace.renaming_workspace = None;
                            }
                        } else if !res.has_focus() {
                            res.request_focus();
                        }
                    } else {
                        // Double-click to rename
                        let color = if is_active { egui::Color32::WHITE } else { egui::Color32::GRAY };
                        let resp = ui.selectable_label(is_active, egui::RichText::new(&ws).strong().color(color));

                        if resp.clicked() {
                            new_active = Some(ws.clone());
                        }
                        if resp.double_clicked() {
                            workspace.renaming_workspace = Some(ws.clone());
                            workspace.rename_buffer = ws.clone();
                        }
                        // Right-click to close (with safety check)
                        if resp.secondary_clicked() && workspace.workspace_order.len() > 1 {
                            tab_to_remove = Some(ws.clone());
                        }
                    }
                }

                // Apply Active Workspace
                if let Some(ws) = new_active {
                    workspace.active_workspace = ws.clone();
                }
                if let Some(ws) = tab_to_remove {
                    workspace.trees.remove(&ws);
                    workspace.workspace_order.retain(|x| x != &ws);
                    if workspace.active_workspace == ws {
                        workspace.active_workspace = workspace.workspace_order.first().cloned().unwrap_or_default();
                    }
                }

                // 3. New Tab Button [+]
                if ui.button(egui::RichText::new("+").color(egui::Color32::GRAY)).clicked() {
                    let mut i = 1;
                    let mut new_name = format!("New Tab {}", i);
                    while workspace.trees.contains_key(&new_name) {
                        i += 1;
                        new_name = format!("New Tab {}", i);
                    }
                    
                    // Auto-insert Project Explorer in the new tab
                    let mut tiles = egui_tiles::Tiles::default();
                    let root = tiles.insert_pane(crate::layout::domain::Pane { 
                        plugin_id: "axicor.explorer".to_string(), 
                        title: "Explorer".to_string() 
                    });
                    let tree = egui_tiles::Tree::new("custom_ws", root, tiles);
                    
                    workspace.trees.insert(new_name.clone(), tree);
                    workspace.workspace_order.push(new_name.clone());
                    workspace.active_workspace = new_name.clone();
                    
                    // Auto-focus renaming
                    workspace.renaming_workspace = Some(new_name.clone());
                    workspace.rename_buffer = new_name;
                }

                // 4. Window Controls (Min/Max/Close)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(" X ").frame(false)).clicked() { exit.send(bevy::app::AppExit); }
                    if ui.add(egui::Button::new(" [] ").frame(false)).clicked() { os_cmd.send(OsWindowCommand::Maximize); }
                    if ui.add(egui::Button::new(" _ ").frame(false)).clicked() { os_cmd.send(OsWindowCommand::Minimize); }

                    let rect = ui.available_rect_before_wrap();
                    if ui.interact(rect, ui.id().with("drag_area"), egui::Sense::drag()).drag_started() {
                        os_cmd.send(OsWindowCommand::Drag);
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, APP_TITLE, egui::FontId::proportional(14.0), COLOR_TITLE);
                });
            });
        });
}

pub fn sync_plugin_visibility_system(
    allocated_panes: Res<AllocatedPanes>,
    mut query: Query<&mut layout_api::PluginWindow>,
) {
    for mut window in query.iter_mut() {
        window.is_visible = allocated_panes.rects.contains_key(&window.plugin_id);
    }
}
