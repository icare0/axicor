use bevy::prelude::*;

#[derive(Component, Default)]
pub struct BlueprintEditorState {
    pub active_zone: Option<String>,
    pub selected_type_idx: usize,
    pub is_dirty: bool,
    pub debounce_timer: f32,
    pub show_delete_modal: bool,       // [DOD FIX] Флаг модалки
    pub type_to_delete: Option<usize>, // [DOD FIX] Индекс на удаление
}
