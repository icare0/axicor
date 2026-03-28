use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use layout_api::AllocatedPanes;
use crate::domain::ProjectFsCache;
use node_editor::domain::LoadGraphEvent;
use connectome_viewer::domain::ZoneSelectedEvent;

pub mod ui_components;

pub fn render_project_explorer_system(
    mut contexts: EguiContexts,
    allocated: Res<AllocatedPanes>,
    cache: Res<ProjectFsCache>,
    mut load_events: EventWriter<LoadGraphEvent>,
    mut zone_events: EventWriter<ZoneSelectedEvent>,
) {
    let Some(rect) = allocated.rects.get(layout_api::DOMAIN_EXPLORER) else { return };
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    egui::Area::new("ExplorerPortal".into())
        .fixed_pos(rect.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.set_clip_rect(*rect);
            // DOD FIX: Отрисовка хедера до передачи управления в UI плагина
            let (content_rect, _) = layout_api::draw_unified_header(ui, *rect, "Project Explorer");

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
                    |proj| { load_events.send(LoadGraphEvent { project_name: proj }); },
                    |proj, shard| { zone_events.send(ZoneSelectedEvent { project_name: proj, shard_name: shard }); }
                );
            });
        });
}