use bevy::prelude::*;

pub const COLOR_BG_ROOT: Color = Color::rgb(0.051, 0.067, 0.09);   // #0d1117
pub const COLOR_BG_PANEL: Color = Color::rgb(0.086, 0.106, 0.133); // #161b22
pub const COLOR_ACCENT: Color = Color::rgb(0.345, 0.651, 1.0);     // #58a6ff
pub const COLOR_BORDER: Color = Color::rgb(0.188, 0.212, 0.239);   // #30363d
pub const COLOR_TEXT_MAIN: Color = Color::WHITE;
pub const COLOR_TEXT_DIM: Color = Color::rgb(0.7, 0.7, 0.7);

pub fn color_bg_root() -> Color { COLOR_BG_ROOT }
pub fn color_bg_panel() -> Color { COLOR_BG_PANEL }
pub fn color_accent() -> Color { COLOR_ACCENT }
pub fn color_border() -> Color { COLOR_BORDER }
pub fn color_text_main() -> Color { COLOR_TEXT_MAIN }
pub fn color_text_dim() -> Color { COLOR_TEXT_DIM }
