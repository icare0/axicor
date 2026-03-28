use egui_tiles::{Behavior, TileId, UiResponse};
use bevy_egui::egui;
use crate::layout::domain::{Pane, TreeCommands};
use crate::layout::systems::input::window_input::edge_triggers;
use layout_api::{AllocatedPanes, TopologyCache, WindowDragRequest};

const MIN_TILE_SIZE:    f32 = 95.0;
const PANE_ROUNDING:    f32 = 10.0;
const PANE_BG:          egui::Color32 = egui::Color32::from_rgb(15, 15, 17);
const BORDER_STROKE:    egui::Color32 = egui::Color32::from_rgb(40, 40, 40);
const PANE_SHRINK:      f32 = 3.0;

pub struct PaneBehavior<'a> {
    pub allocated_panes: &'a mut AllocatedPanes,
    pub topology:        &'a mut TopologyCache,
    pub drag_request:    &'a mut WindowDragRequest,
    pub tree_commands:   &'a mut TreeCommands,
}

impl<'a> Behavior<Pane> for PaneBehavior<'a> {
    fn min_size(&self) -> f32 { MIN_TILE_SIZE }

    fn pane_ui(&mut self, ui: &mut egui::Ui, tile_id: TileId, pane: &mut Pane) -> UiResponse {
        ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, BORDER_STROKE);

        let rect         = ui.available_rect_before_wrap();
        let payload_rect = rect.shrink(PANE_SHRINK);

        // DOD FIX: WM отдаёт плагину 100% пространства. Никаких системных хедеров.
        self.topology.tiles.insert(tile_id, payload_rect);
        self.allocated_panes.rects.insert(pane.plugin_id.clone(), payload_rect);

        ui.painter().rect_filled(payload_rect, PANE_ROUNDING, PANE_BG);

        // DOD FIX: Изолируем логику якоря и пробиваем Z-Index плагинов
        handle_system_dnd_anchor(ui, tile_id, payload_rect, self.drag_request, self.tree_commands);

        draw_trigger_highlights(ui, payload_rect);
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title.clone().into()
    }
}

fn draw_trigger_highlights(ui: &mut egui::Ui, rect: egui::Rect) {
    let triggers = edge_triggers(rect);
    let pointer_pos = ui.ctx().pointer_hover_pos();
    
    // DOD FIX: O(1) поиск без аллокаций. Определяем, наведен ли курсор на один из триггеров
    let hovered_idx = pointer_pos.and_then(|p| triggers.iter().position(|t| t.contains(p)));

    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, ui.id().with("trig_fg")));

    for (idx, &trigger) in triggers.iter().enumerate() {
        let is_hovered = hovered_idx == Some(idx);
        
        // 10% видимости (alpha 25) по умолчанию, 50% (alpha 127) при наведении
        let alpha = if is_hovered { 127 } else { 25 }; 
        let trigger_color = egui::Color32::from_white_alpha(alpha);

        let points = match idx {
            0 => vec![trigger.left_top(), trigger.right_top(), trigger.left_bottom()],       // NW
            1 => vec![trigger.left_top(), trigger.right_top(), trigger.right_bottom()],      // NE
            2 => vec![trigger.left_top(), trigger.right_bottom(), trigger.left_bottom()],    // SW
            3 => vec![trigger.right_top(), trigger.right_bottom(), trigger.left_bottom()],   // SE
            _ => unreachable!(),
        };

        painter.add(egui::Shape::convex_polygon(points, trigger_color, egui::Stroke::NONE));
    }
}
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn handle_system_dnd_anchor(
    ui: &mut egui::Ui,
    tile_id: TileId,
    payload_rect: egui::Rect,
    drag_request: &mut WindowDragRequest,
    tree_commands: &mut TreeCommands,
) {
    let btn_size = egui::vec2(25.0, 15.0);
    let offset = 6.5;
    let btn_rect = egui::Rect::from_min_size(payload_rect.min + egui::vec2(offset, offset), btn_size);

    let mut is_hovered = false;

    // DOD FIX: Area пробивает z-index плагинов.
    egui::Area::new(ui.id().with(tile_id).with("sys_anchor_area"))
        .fixed_pos(btn_rect.min)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ui.ctx(), |ui| {
            // DOD FIX: egui ВСЕГДА использует абсолютные координаты экрана. 
            // Передаём btn_rect напрямую, чтобы хитбокс совпал с визуалом.
            let response = ui.interact(btn_rect, ui.id().with("sys_btn"), egui::Sense::click_and_drag());

            is_hovered = response.hovered();

            // 1. Интент на Swap (Drag)
            if response.drag_started() {
                if let Some(pos) = ui.ctx().pointer_interact_pos() {
                    drag_request.active      = true;
                    drag_request.source      = layout_api::DragSource::Header;
                    drag_request.target_tile = Some(tile_id);
                    drag_request.start_pos   = pos;
                }
            }

            // 2. Интент на смену домена (Click -> Popup)
            let popup_id = ui.id().with(tile_id).with("domain_switcher");
            if response.clicked() {
                ui.memory_mut(|m| m.toggle_popup(popup_id));
            }

            egui::popup_below_widget(ui, popup_id, &response, |ui| {
                ui.set_min_width(140.0);
                let domains = [
                    (layout_api::DOMAIN_EXPLORER, "Project Explorer"),
                    (layout_api::DOMAIN_VIEWPORT, "Connectome Viewer"),
                    (layout_api::DOMAIN_NODE_ED,  "Topology Editor"),
                    (layout_api::DOMAIN_CODE_EDITOR, "Code Editor"),
                ];
                for (dom_id, label) in domains {
                    if ui.button(label).clicked() {
                        tree_commands.queue.push(layout_api::TreeCommand::ChangeDomain {
                            tile_id,
                            new_domain: dom_id.to_string(),
                        });
                        ui.memory_mut(|m| m.close_popup());
                    }
                }
            });
        });

    // Отрисовка якоря на слое Foreground
    let fg_painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, ui.id().with("wm_fg_anchor")));
    let bg_color = if is_hovered { egui::Color32::from_rgb(70, 70, 75) } else { egui::Color32::from_rgb(50, 50, 55) };
    let stroke_color = egui::Color32::from_rgb(80, 80, 85);

    let min = btn_rect.min;
    let max = btn_rect.max;

    let points = vec![
        min + egui::vec2(5.0, 0.0),
        egui::pos2(max.x, min.y),
        max,
        egui::pos2(min.x, max.y),
        min + egui::vec2(0.0, 5.0),
    ];

    fg_painter.add(egui::Shape::convex_polygon(points, bg_color, egui::Stroke::new(1.0, stroke_color)));
}
