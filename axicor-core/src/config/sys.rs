use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct SystemMeta {
    pub id: String,
    pub version: String,
    pub created_at: String,
}

impl SystemMeta {
    pub fn generate() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            version: "1.0.0".to_string(),
            created_at: "2026-04-17T00:00:00Z".to_string(), // ISO8601 placeholder
        }
    }
}
