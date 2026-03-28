use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{TileId, LinearDir};
use std::path::Path;

pub const DOMAIN_VIEWPORT: &str = "axicor.viewport_3d";
pub const DOMAIN_EXPLORER: &str = "axicor.explorer";
pub const DOMAIN_NODE_ED:  &str = "axicor.node_editor";

// Отражает размер системного DND-якоря (6.5px offset + 25px width + 10px gap)
pub const SYS_UI_SAFE_ZONE: f32 = 41.5;

#[derive(Resource, Default, Clone)]
pub struct AllocatedPanes {
    pub rects: bevy::utils::HashMap<String, egui::Rect>,
}

// --- Components ---

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PluginInput {
    pub local_cursor: Vec2,
    pub cursor_delta: Vec2,
    pub scroll_delta: f32,
    pub is_primary_pressed: bool,
    pub is_secondary_pressed: bool, // For rotation (RMB)
    pub is_middle_pressed: bool,    // For panning (MMB)
}

#[derive(Resource, Default, Debug, Clone)]
pub struct TopologyCache {
    pub tiles: bevy::utils::HashMap<egui_tiles::TileId, egui::Rect>,
}

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PluginGeometry {
    pub size: Vec2,
}

// DOD FIX: Строго динамическая идентификация
#[derive(Component)]
pub struct PluginWindow {
    pub plugin_id: String,
    pub texture: Option<Handle<Image>>,
    pub is_visible: bool,
}

// --- Enums & Commands (API Contract) ---

#[derive(Debug, Clone, Default, PartialEq)]
pub enum DragSource {
    #[default]
    EdgeTrigger,
    Header,
}

#[derive(Resource, Default, Debug, Clone)] // Added Resource
pub struct WindowDragRequest {
    pub active:       bool,
    pub start_pos:    egui::Pos2,
    pub current_pos:  egui::Pos2,
    pub target_tile:  Option<egui_tiles::TileId>,
    pub source:       DragSource,
}

#[derive(Default, PartialEq, Clone)]
pub enum DragIntent {
    #[default]
    None,
    Split { axis: egui_tiles::LinearDir, fraction: f32, insert_before: bool, plugin_id: String },
    Merge { victim: egui_tiles::TileId },
    Swap  { victim: egui_tiles::TileId },
}

pub enum TreeCommand {
    Split {
        target: TileId,
        axis: LinearDir,
        fraction: f32,
        insert_before: bool,
        plugin_id: String,
    },
    Merge { survivor: TileId, victim: TileId },
    SwapPanes { src: TileId, dst: TileId },
    ChangeDomain { tile_id: TileId, new_domain: String },
}

#[derive(Event, Debug, Clone)]
pub struct UpdateInputEvent {
    pub entity: Entity,
    pub input: PluginInput,
}

#[derive(Event, Debug, Clone)]
pub struct UpdateGeometryEvent {
    pub entity: Entity,
    pub geometry: PluginGeometry,
}

pub struct PaneData {
    pub plugin_id: String,
    pub texture_id: Option<egui::TextureId>,
}

#[derive(Resource, Default)]
pub struct ActiveBundle {
    pub project_name: String,
    pub archive: Option<genesis_core::vfs::AxicArchive>,
}

impl ActiveBundle {
    pub fn load(&mut self, axic_path: &Path, name: &str) -> Option<()> {
        self.archive = genesis_core::vfs::AxicArchive::open(axic_path);
        self.project_name = name.to_string();
        self.archive.as_ref().map(|_| ())
    }

    pub fn get_file(&self, path: &str) -> Option<&[u8]> {
        self.archive.as_ref()?.get_file(path)
    }

    pub fn toc(&self) -> Option<&std::collections::HashMap<String, (usize, usize)>> {
        Some(&self.archive.as_ref()?.toc)
    }
}

// Унифицированная палитра
pub const COLOR_HEADER_BG: egui::Color32 = egui::Color32::from_rgb(35, 35, 40);
pub const COLOR_HEADER_LINE: egui::Color32 = egui::Color32::from_rgb(20, 20, 20);

/// Отрисовывает унифицированный заголовок плагина и возвращает (Content_Rect, Toolbar_Rect)
pub fn draw_unified_header(ui: &mut egui::Ui, rect: egui::Rect, title: &str) -> (egui::Rect, egui::Rect) {
    let mut header_rect = rect;
    header_rect.set_height(28.0);

    // DOD FIX: Строгое скругление только верхних углов
    ui.painter().rect_filled(
        header_rect, 
        egui::Rounding { nw: 10.0, ne: 10.0, sw: 0.0, se: 0.0 }, 
        COLOR_HEADER_BG
    );
    
    // Сепаратор 1px
    ui.painter().line_segment(
        [header_rect.left_bottom(), header_rect.right_bottom()],
        egui::Stroke::new(1.0, COLOR_HEADER_LINE),
    );

    // Текст названия плагина
    let title_pos = header_rect.left_center() + egui::vec2(SYS_UI_SAFE_ZONE, 0.0);
    ui.painter().text(
        title_pos,
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(130, 130, 130), // DOD FIX: Идеальное совпадение с APP_TITLE
    );

    // Примерная ширина текста (защита от аллокации Font_Galley в горячем циклe)
    let text_width = title.len() as f32 * 8.0;
    let mut toolbar_rect = header_rect;
    // DOD FIX: Строгий отступ 25px от конца текста до начала зоны кнопок
    toolbar_rect.min.x = title_pos.x + text_width + 25.0; 

    let mut content_rect = rect;
    content_rect.min.y = header_rect.max.y; // Основная рабочая зона

    (content_rect, toolbar_rect)
}
