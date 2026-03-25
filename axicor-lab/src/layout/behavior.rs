use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{Behavior, TileId, UiResponse, LinearDir};
use std::collections::HashMap;
use crate::layout::data::{PluginInput, PluginGeometry, WindowDragState, PaneData};

pub struct PaneBehavior<'a> {
    pub panes: &'a HashMap<Entity, PaneData>,
    pub drag_state: &'a mut WindowDragState,
    pub rects: &'a mut HashMap<TileId, egui::Rect>,
    pub input_updates: Vec<(Entity, PluginInput)>,
    pub geometry_updates: Vec<(Entity, PluginGeometry)>,
}

impl<'a> Behavior<Entity> for PaneBehavior<'a> {
    fn tab_bar_height(&self, _style: &egui::Style) -> f32 { 0.0 }

    fn pane_ui(&mut self, ui: &mut egui::Ui, tile_id: TileId, entity: &mut Entity) -> UiResponse {
        let rect = ui.max_rect().shrink(5.0);
        self.rects.insert(tile_id, rect);

        if let Some(data) = self.panes.get(entity) {
            ui.put(
                rect,
                egui::Image::new(egui::load::SizedTexture::new(data.texture_id, rect.size()))
                    .rounding(10.0),
            );

            // Death Mark
            if rect.width() < 100.0 || rect.height() < 100.0 {
                ui.painter().rect_filled(rect, 10.0, egui::Color32::from_black_alpha(150));
            }

            // Hitboxes (Initiation only)
            if !self.drag_state.is_dragging {
                let thickness = 10.0;
                let edges = [
                    (egui::Rect::from_min_max(rect.left_top(), rect.right_top() + egui::vec2(0.0, thickness)), LinearDir::Vertical, -1.0),
                    (egui::Rect::from_min_max(rect.left_bottom() - egui::vec2(0.0, thickness), rect.right_bottom()), LinearDir::Vertical, 1.0),
                    (egui::Rect::from_min_max(rect.left_top(), rect.left_bottom() + egui::vec2(thickness, 0.0)), LinearDir::Horizontal, -1.0),
                    (egui::Rect::from_min_max(rect.right_top() - egui::vec2(thickness, 0.0), rect.right_bottom()), LinearDir::Horizontal, 1.0),
                ];

                for (edge_rect, axis, normal_sign) in edges {
                    let resp = ui.interact(edge_rect, ui.id().with(axis).with(normal_sign as i32), egui::Sense::drag());
                    
                    if resp.hovered() {
                        ui.painter().rect_filled(edge_rect, 5.0, egui::Color32::from_white_alpha(13));
                    }

                    if resp.drag_started() {
                        self.drag_state.is_dragging = true;
                        self.drag_state.source_tile = Some(tile_id);
                        self.drag_state.start_pos = ui.ctx().pointer_interact_pos();
                        self.drag_state.drag_axis = Some(axis);
                        self.drag_state.drag_normal = Some(normal_sign);
                        break;
                    }
                }
            }

            // Local Inputs
            let pointer_pos = ui.ctx().pointer_hover_pos();
            let mut input = PluginInput::default();
            if let Some(pos) = pointer_pos {
                if rect.contains(pos) {
                    let local = pos - rect.min;
                    input.local_cursor = Vec2::new(local.x, local.y);
                    input.is_pressed = ui.ctx().input(|i| i.pointer.primary_down());
                }
            }
            self.input_updates.push((*entity, input));

            let new_size = Vec2::new(rect.width(), rect.height());
            self.geometry_updates.push((*entity, PluginGeometry { size: new_size }));
        }
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, _entity: &Entity) -> egui::WidgetText { "".into() }
}
