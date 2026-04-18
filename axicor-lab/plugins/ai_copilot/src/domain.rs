use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Serialize, Deserialize)]
pub struct CopilotConfig {
    pub api_endpoint: String,
    pub api_key: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatRole { User, Copilot, System }

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Component)]
pub struct AiCopilotState {
    pub history: Vec<ChatMessage>,
    pub input_buffer: String,
    pub system_prompt: String,
    pub is_generating: bool,
    pub show_settings: bool,
    pub api_endpoint: String,
    pub api_key: String,
    pub tx: Sender<String>,
    pub rx: Receiver<String>,
}

impl Default for AiCopilotState {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            history: vec![ChatMessage {
                role: ChatRole::Copilot,
                content: "Greetings, Engineer. I am the Axicor AI Copilot. How can I assist with your connectome today?".to_string(),
            }],
            input_buffer: String::new(),
            system_prompt: "You are an expert AI Copilot embedded within the Axicor Lab IDE. You help neuroengineers design spiking neural networks. Format your answers nicely with markdown.".to_string(),
            is_generating: false,
            show_settings: false,
            api_endpoint: "https://api.deepseek.com/v1".to_string(), // Default DeepSeek or Localhost
            api_key: "".to_string(),
            tx, rx,
        }
    }
}
