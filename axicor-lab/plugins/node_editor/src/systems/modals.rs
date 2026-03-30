use bevy::prelude::*;
use bevy_egui::EguiContexts;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};
use crate::ui::modals::draw_clear_graph_modal;

pub fn clear_graph_modal_system(
    mut contexts: EguiContexts,
    mut query: Query<(&layout_api::PluginWindow, &mut NodeGraphUiState)>,
    graph: Res<BrainTopologyGraph>,
    mut topo_events: EventWriter<TopologyMutation>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (window, mut state) in query.iter_mut() {
        if !window.is_visible || !state.show_clear_modal {
            continue;
        }

        // 1. Вызов чистого слоя отрисовки
        let confirmed = draw_clear_graph_modal(ctx, window.rect, &mut state);

        // 2. Оркестрация намерений (Строго вне замыканий egui)
        if confirmed {
            if let Some(active_path) = &graph.active_path {
                if let Some(session) = graph.sessions.get(active_path) {
                    let zones = session.zones.clone();
                    for zone in zones {
                        let id = session.zone_ids.get(&zone).cloned().unwrap_or_default();
                        topo_events.send(TopologyMutation::RemoveZone {
                            name: zone,
                            id,
                            context_path: Some(active_path.clone()),
                        });
                    }
                    info!("✅ Node Editor: Triggered Clear Graph cascade.");
                }
            }
        }
    }
}
