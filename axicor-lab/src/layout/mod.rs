pub mod domain;
pub mod plugin;
pub mod behavior;
pub mod systems;
pub mod overlay;
pub mod ui;

// Реэкспорт для удобства внешних плагинов
pub use domain::Pane;
pub use plugin::WindowManagerPlugin;
