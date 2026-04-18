use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use layout_api::{PluginWindow, draw_unified_header, base_domain, DOMAIN_BLUEPRINT_EDITOR};
use crate::domain::BlueprintEditorState;
use node_editor::domain::BrainTopologyGraph;

pub fn render_blueprint_editor_system(
    mut contexts: EguiContexts,
    windows: Query<(Entity, &PluginWindow)>,
    mut states: Query<&mut BlueprintEditorState>,
    mut graph: ResMut<BrainTopologyGraph>, // [DOD FIX] Mut access for In-Place edits
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (entity, window) in windows.iter() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_BLUEPRINT_EDITOR { continue; }
        let Ok(mut state) = states.get_mut(entity) else { continue };

        egui::Area::new(window.id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                let (content_rect, _) = draw_unified_header(ui, window.rect, "Neuron Settings");

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(18, 18, 20))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            let Some(zone) = &state.active_zone else {
                                ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("Select a Shard").color(egui::Color32::DARK_GRAY)); });
                                return;
                            };

                            let Some(active_path) = graph.active_path.clone() else { return; };
                            let Some(session) = graph.sessions.get_mut(&active_path) else { return; };
                            
                            if let Some(blueprint) = session.shard_blueprints.get_mut(zone) {
                                if blueprint.neuron_type.is_empty() {
                                    ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("No Neuron Types Defined").color(egui::Color32::DARK_GRAY)); });
                                    return;
                                }

                                // Safety: Prevent OOB access
                                if state.selected_type_idx >= blueprint.neuron_type.len() {
                                    state.selected_type_idx = 0;
                                }

                                // Handle CRUD actions
                                let action = crate::ui::type_selector::draw_type_selector(ui, &blueprint.neuron_type, &mut state.selected_type_idx);
                                match action {
                                    crate::ui::type_selector::TypeAction::Add => {
                                        let mut new_id = blueprint.neuron_type.len();
                                        let mut new_name = format!("Type_{}", new_id);
                                        while blueprint.neuron_type.iter().any(|t| t.name == new_name) {
                                            new_id += 1;
                                            new_name = format!("Type_{}", new_id);
                                        }
                                        
                                        // [DOD FIX] Library defaults (no zero-values)
                                        let mut new_type = node_editor::domain::NeuronType {
                                            name: new_name,
                                            ..Default::default()
                                        };
                                        new_type.threshold = 20000;
                                        new_type.leak_rate = 100;
                                        new_type.homeostasis_penalty = 1000;
                                        new_type.homeostasis_decay = 10;
                                        new_type.refractory_period = 10;
                                        new_type.synapse_refractory_period = 10;
                                        new_type.signal_propagation_length = 5;
                                        
                                        blueprint.neuron_type.push(new_type);
                                        state.selected_type_idx = blueprint.neuron_type.len() - 1;
                                        state.is_dirty = true;
                                        state.debounce_timer = 0.0;
                                    }
                                    crate::ui::type_selector::TypeAction::Delete => {
                                        state.show_delete_modal = true;
                                        state.type_to_delete = Some(state.selected_type_idx);
                                    }
                                    _ => {}
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);

                                let selected_idx = state.selected_type_idx;

                                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                                    let mut changed = false; 
                                    // DOD: Pass Whitelist names from the current blueprint
                                    let all_type_names: Vec<String> = blueprint.neuron_type.iter().map(|t| t.name.clone()).collect();
                                    let nt = &mut blueprint.neuron_type[selected_idx];

                                    changed |= crate::ui::sections::membrane::draw_membrane_section(ui, nt);
                                    changed |= crate::ui::sections::timing::draw_timing_section(ui, nt);
                                    changed |= crate::ui::sections::plasticity::draw_plasticity_section(ui, nt);
                                    changed |= crate::ui::sections::homeostasis::draw_homeostasis_section(ui, nt);
                                    changed |= crate::ui::sections::growth::draw_growth_section(ui, nt);
                                    changed |= crate::ui::sections::sprouting::draw_sprouting_section(ui, nt);
                                    changed |= crate::ui::sections::inertia::draw_inertia_section(ui, nt);
                                    changed |= crate::ui::sections::dendrite_filter::draw_dendrite_filter_section(ui, nt, &all_type_names);

                                    // [DOD FIX] Reset Debounce-timer on changes
                                    if changed {
                                        state.is_dirty = true;
                                        state.debounce_timer = 0.0;
                                    }
                                });

                            } else {
                                ui.centered_and_justified(|ui| { ui.label(egui::RichText::new("No Blueprint Data").color(egui::Color32::DARK_GRAY)); });
                            }
                        }); //  Frame::show

                    // [DOD FIX] Confirmation Modal (manual overlay)
                    if state.show_delete_modal {
                        if let Some(del_idx) = state.type_to_delete {
                            // Re-fetch blueprint to avoid borrow checker conflicts
                            let active_path = graph.active_path.clone().unwrap();
                            let session = graph.sessions.get_mut(&active_path).unwrap();
                            let blueprint = session.shard_blueprints.get_mut(state.active_zone.as_ref().unwrap()).unwrap();
                            
                            let type_name = blueprint.neuron_type.get(del_idx).map(|t| t.name.clone()).unwrap_or_default();
                            let (confirmed, closed) = crate::ui::modals::draw_delete_type_modal(ctx, window.rect, &type_name);
                            
                            if confirmed {
                                // 1. Dependency Cleanup: Remove from whitelists
                                for (i, t) in blueprint.neuron_type.iter_mut().enumerate() {
                                    if i != del_idx {
                                        t.dendrite_whitelist.retain(|name| name != &type_name);
                                    }
                                }
                                // 2. Physical deletion
                                blueprint.neuron_type.remove(del_idx);
                                
                                // 3. Update selection (prevent OOB)
                                if state.selected_type_idx >= blueprint.neuron_type.len() {
                                    state.selected_type_idx = blueprint.neuron_type.len().saturating_sub(1);
                                }
                                
                                state.is_dirty = true;
                                state.debounce_timer = 0.0;
                            }
                            
                            if closed {
                                state.show_delete_modal = false;
                                state.type_to_delete = None;
                            }
                        } else {
                            state.show_delete_modal = false;
                        }
                    }
                }); //  ui.allocate_ui_at_rect
            });
    }
}
