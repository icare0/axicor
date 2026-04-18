use crate::domain::{AiCopilotState, ChatMessage, ChatRole};
use bevy::prelude::*;
use bevy_egui::egui;
use layout_api::{base_domain, PluginWindow, DOMAIN_AI_COPILOT};

const CHAT_BG: egui::Color32 = egui::Color32::from_rgb(25, 25, 28);
const CLR_USER: egui::Color32 = egui::Color32::LIGHT_BLUE;
const CLR_COPILOT: egui::Color32 = egui::Color32::from_rgb(200, 220, 200);

pub fn init_copilot_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Added<PluginWindow>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_AI_COPILOT {
            let mut state = AiCopilotState::default();
            //    (ignored by git)
            if let Ok(content) = std::fs::read_to_string("copilot.local.toml") {
                if let Ok(cfg) = toml::from_str::<crate::domain::CopilotConfig>(&content) {
                    state.api_endpoint = cfg.api_endpoint;
                    state.api_key = cfg.api_key;
                }
            }
            commands.entity(entity).insert(state);
        }
    }
}

pub fn render_copilot_system(
    mut contexts: bevy_egui::EguiContexts,
    mut windows: Query<(&PluginWindow, &mut AiCopilotState)>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else {
        return;
    };

    for (window, mut state) in windows.iter_mut() {
        if !window.is_visible || base_domain(&window.plugin_id) != DOMAIN_AI_COPILOT {
            continue;
        }

        // DOD FIX:
        if let Ok(response) = state.rx.try_recv() {
            state.history.push(ChatMessage {
                role: ChatRole::Copilot,
                content: response,
            });
            state.is_generating = false;
        }

        let area_id = format!("AiCopilotPortal_{:?}", window.id);
        egui::Area::new(area_id.into())
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);
                ui.set_min_size(window.rect.size());

                let (content_rect, header_rect) =
                    layout_api::draw_unified_header(ui, window.rect, "AI Copilot");

                render_gear_button(ui, header_rect, &mut state);

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(CHAT_BG)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            if state.show_settings {
                                render_settings(ui, &mut state);
                            }
                            render_input_panel(ui, &mut state);
                            render_chat_history(ui, &state);
                        });
                });
            });
    }
}

// ---------------------------------------------------------------------------

fn render_gear_button(ui: &mut egui::Ui, header_rect: egui::Rect, state: &mut AiCopilotState) {
    let gear_rect = egui::Rect::from_min_size(
        header_rect.right_top() + egui::vec2(-35.0, 5.0),
        egui::vec2(25.0, 25.0),
    );
    ui.allocate_ui_at_rect(gear_rect, |ui| {
        if ui.button("").clicked() {
            state.show_settings = !state.show_settings;
        }
    });
}

fn render_settings(ui: &mut egui::Ui, state: &mut AiCopilotState) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Copilot Settings").strong());
            if ui.button(" Save Keys").clicked() {
                let cfg = crate::domain::CopilotConfig {
                    api_endpoint: state.api_endpoint.clone(),
                    api_key: state.api_key.clone(),
                };
                if let Ok(toml_str) = toml::to_string(&cfg) {
                    let _ = std::fs::write("copilot.local.toml", toml_str);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Endpoint:");
            ui.text_edit_singleline(&mut state.api_endpoint);
        });
        ui.horizontal(|ui| {
            ui.label("API Key:");
            ui.text_edit_singleline(&mut state.api_key);
        });
    });
    ui.add_space(5.0);
}

fn render_input_panel(ui: &mut egui::Ui, state: &mut AiCopilotState) {
    egui::TopBottomPanel::bottom(ui.id().with("input_panel"))
        .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(0.0, 5.0)))
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let response = ui.add(
                    egui::TextEdit::multiline(&mut state.input_buffer)
                        .desired_rows(1)
                        .desired_width(ui.available_width() - 40.0)
                        .hint_text("Ask Copilot..."),
                );

                let enter_pressed = response.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift);
                let btn_clicked = ui
                    .add_enabled(!state.is_generating, egui::Button::new(""))
                    .clicked();

                if (enter_pressed || btn_clicked) && !state.is_generating {
                    try_send_message(state);
                }
            });
        });
}

fn render_chat_history(ui: &mut egui::Ui, state: &AiCopilotState) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &state.history {
                        render_message(ui, msg);
                        ui.add_space(8.0);
                    }
                    if state.is_generating {
                        ui.label(
                            egui::RichText::new("Generating...")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
                    }
                });
        });
}

fn render_message(ui: &mut egui::Ui, msg: &ChatMessage) {
    let (layout, bg_color, text_color) = match msg.role {
        ChatRole::User => (
            egui::Layout::right_to_left(egui::Align::TOP),
            egui::Color32::from_rgb(35, 45, 65), //
            CLR_USER,
        ),
        ChatRole::Copilot => (
            egui::Layout::left_to_right(egui::Align::TOP),
            egui::Color32::from_rgb(35, 38, 40), // -
            CLR_COPILOT,
        ),
        _ => return,
    };

    ui.with_layout(layout, |ui| {
        // DOD FIX:    85%    ,
        //          .
        let max_bubble_width = ui.available_width() * 0.85;

        bevy_egui::egui::Frame::none()
            .fill(bg_color)
            .rounding(6.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.set_max_width(max_bubble_width);

                // DOD FIX:    !
                ui.add(
                    bevy_egui::egui::Label::new(
                        bevy_egui::egui::RichText::new(&msg.content).color(text_color),
                    )
                    .wrap(true),
                );
            });
    });
}

///          
fn try_send_message(state: &mut AiCopilotState) {
    if state.input_buffer.trim().is_empty() {
        return;
    }
    let msg = std::mem::take(&mut state.input_buffer);
    state.history.push(ChatMessage {
        role: ChatRole::User,
        content: msg,
    });
    state.is_generating = true;

    //
    let tx = state.tx.clone();
    // DOD FIX:     (, \n),
    //       .toml
    let endpoint = state.api_endpoint.trim().to_string();
    let api_key = state.api_key.trim().to_string();
    let system_prompt = state.system_prompt.clone();
    let history = state.history.clone();

    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();

        let mut messages = vec![serde_json::json!({"role": "system", "content": system_prompt})];

        for h in &history {
            let role_str = match h.role {
                ChatRole::User => "user",
                ChatRole::Copilot => "assistant",
                ChatRole::System => "system",
            };
            messages.push(serde_json::json!({"role": role_str, "content": h.content}));
        }

        let body = serde_json::json!({
            "model": "deepseek-chat", //   LM Studio,  API DeepSeek
            "messages": messages,
            "temperature": 0.3
        });

        let url = if endpoint.ends_with('/') {
            format!("{}chat/completions", endpoint)
        } else {
            format!("{}/chat/completions", endpoint)
        };

        let res = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send();

        match res {
            Ok(response) => {
                let status = response.status();
                // DOD FIX:    .
                // HTML- Cloudflare (502)     (401)
                let text = response.text().unwrap_or_default();

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                        let _ = tx.send(content.to_string());
                    } else if let Some(err) = json["error"]["message"].as_str() {
                        let _ = tx.send(format!("API Error ({}): {}", status, err));
                    } else {
                        let _ = tx.send(format!("Unknown JSON format: {}", text));
                    }
                } else {
                    //     JSON (, 404  502 HTML )
                    let _ = tx.send(format!("HTTP {} - Raw response: {}", status, text));
                }
            }
            Err(e) => {
                let _ = tx.send(format!("Network Error: {}", e));
            }
        }
    });
}
