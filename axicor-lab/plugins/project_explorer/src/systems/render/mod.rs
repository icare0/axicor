use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::{PluginWindow, base_domain, DOMAIN_EXPLORER, OpenFileEvent};
use crate::domain::ProjectFsCache;
use node_editor::domain::LoadGraphEvent;
use connectome_viewer::domain::ZoneSelectedEvent;

pub mod ui_components;

pub fn render_project_explorer_system(
    mut contexts: EguiContexts,
    cache: Res<ProjectFsCache>,
    mut load_events: EventWriter<LoadGraphEvent>,
    mut zone_events: EventWriter<ZoneSelectedEvent>,
    mut open_file_ev: EventWriter<OpenFileEvent>,
    window_query: Query<&PluginWindow>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for window in window_query.iter() {
        if !window.is_visible { continue; }
        if base_domain(&window.plugin_id) != DOMAIN_EXPLORER { continue; }

        let rect = window.rect;
        let area_id = format!("ExplorerPortal_{:?}", window.id);

        egui::Area::new(area_id.into())
            .fixed_pos(rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(rect);
                // DOD FIX: Отрисовка хедера до передачи управления в UI плагина
                let (content_rect, _) = layout_api::draw_unified_header(ui, rect, "Project Explorer");

                // Подготовка данных
                let mut bundles = Vec::new();
                let mut sources = Vec::new();
                for project in &cache.projects {
                    if project.is_bundle { bundles.push(project); } 
                    else { sources.push(project); }
                }
                
                ui.allocate_ui_at_rect(content_rect, |ui| {
                    ui_components::draw_explorer_tree(
                        ui, 
                        &bundles, 
                        &sources,
                        &mut open_file_ev,
                        |proj| { load_events.send(LoadGraphEvent { project_name: proj }); },
                        |proj, shard| { zone_events.send(ZoneSelectedEvent { project_name: proj, shard_name: shard }); }
                    );
                });
            });
    }
}