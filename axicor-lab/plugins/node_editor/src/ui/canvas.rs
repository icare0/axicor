// ui/canvas.rs
use bevy_egui::egui::{self, Color32, Pos2, Vec2, Rect};
use crate::domain::NodeGraphUiState;

const ZOOM_MIN:   f32 = 0.5;
const ZOOM_MAX:   f32 = 1.5;
const ZOOM_SPEED: f32 = 0.002;

// ---   ---
const CLR_BG:         Color32 = Color32::from_rgb(9, 9, 10);

const GRID_STEP:       f32 = 40.0;
const GRID_MAJOR_MULT: i32 = 4;
const GRID_SUPER_MULT: i32 = 4;

const WIDTH_THIN:  f32 = 0.5;
const WIDTH_MID:   f32 = 0.25;
const WIDTH_THICK: f32 = 0.75;

#[derive(Copy, Clone)]
pub struct CanvasTransform {
    pub pan:    Vec2,
    pub zoom:   f32,
    pub origin: Pos2,
}

impl CanvasTransform {
    pub fn to_screen(&self, local: Pos2) -> Pos2 {
        self.origin + self.pan + local.to_vec2() * self.zoom
    }

    pub fn to_local(&self, screen: Pos2) -> Pos2 {
        ((screen.to_vec2() - self.origin.to_vec2() - self.pan) / self.zoom).to_pos2()
    }
}

pub fn handle_input(
    ui: &mut egui::Ui,
    rect: Rect,
    state: &mut NodeGraphUiState,
) -> (CanvasTransform, egui::Response) {
    let response = ui.interact(rect, ui.id().with("canvas_bg"), egui::Sense::click_and_drag());

    let is_pan = response.dragged_by(egui::PointerButton::Middle)
        || (response.dragged_by(egui::PointerButton::Primary) && ui.ctx().dragged_id().is_none());

    if is_pan { state.pan += response.drag_delta(); }

    if ui.rect_contains_pointer(rect) {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = state.zoom;
            state.zoom = (state.zoom + scroll * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
            if let Some(mouse) = ui.input(|i| i.pointer.hover_pos()) {
                let local = (mouse.to_vec2() - rect.min.to_vec2() - state.pan) / old_zoom;
                state.pan = mouse.to_vec2() - rect.min.to_vec2() - local * state.zoom;
            }
        }
    }

    let transform = CanvasTransform { pan: state.pan, zoom: state.zoom, origin: rect.min };

    (transform, response)
}

pub fn draw_background(painter: &egui::Painter, rect: Rect, transform: &CanvasTransform) {
    painter.rect_filled(rect, 0.0, CLR_BG);

    let step = GRID_STEP * transform.zoom;
    if step < 4.0 { return; }

    let offset_x = transform.pan.x.rem_euclid(step);
    let offset_y = transform.pan.y.rem_euclid(step);

    let start_col = (-(transform.pan.x / step)).floor() as i32;
    let start_row = (-(transform.pan.y / step)).floor() as i32;

    let cols = (rect.width()  / step).ceil() as i32 + 1;
    let rows = (rect.height() / step).ceil() as i32 + 1;

    //  
    for col in 0..cols {
        let x = rect.min.x + offset_x + col as f32 * step;
        if x < rect.min.x || x > rect.max.x { continue; }

        let idx = start_col + col;
        let (width, color) = line_style(idx, 0);

        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            egui::Stroke::new(width, color),
        );
    }

    //  
    for row in 0..rows {
        let y = rect.min.y + offset_y + row as f32 * step;
        if y < rect.min.y || y > rect.max.y { continue; }

        let idx = start_row + row;
        let (width, color) = line_style(0, idx);

        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            egui::Stroke::new(width, color),
        );
    }
}

#[inline]
fn line_style(col: i32, row: i32) -> (f32, Color32) {
    let clr_thin  = Color32::from_rgba_unmultiplied(255, 255, 255, 8);
    let clr_mid   = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
    let clr_thick = Color32::from_rgba_unmultiplied(255, 255, 255, 35);

    let idx = if col != 0 { col } else { row };
    let super_step = GRID_MAJOR_MULT * GRID_SUPER_MULT;

    if idx % super_step == 0 {
        (WIDTH_THICK, clr_thick)
    } else if idx % GRID_MAJOR_MULT == 0 {
        (WIDTH_MID, clr_mid)
    } else {
        (WIDTH_THIN, clr_thin)
    }
}
