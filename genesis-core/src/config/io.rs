use serde::Deserialize;

/// Represents external projection connections coming into this shard (White Matter/Atlas).
#[derive(Debug, Deserialize, Clone)]
pub struct IoConfig {
    #[serde(rename = "input")]
    pub inputs: Vec<InputMap>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputMap {
    /// Имя признака/канала, например "retina_edges"
    pub name: String,
    
    /// Название зоны куда инжектится этот ввод, например "V1"
    pub target_zone: String,
    
    /// Тип нейрона к которому нужно подключаться, например "L4_Stellate".
    /// Используйте "ALL" чтобы не ограничивать выбор типом.
    pub target_type: String,
    
    /// Ширина входной матрицы в пикселях
    pub width: u32,
    
    /// Высота входной матрицы в пикселях
    pub height: u32,
}

impl IoConfig {
    /// Парсит конфиг из TOML строки.
    pub fn parse(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| format!("TOML parse error: {}", e))
    }

    /// Загружает конфиг с диска.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
        Self::parse(&content)
    }
}

#[cfg(test)]
#[path = "test_io.rs"]
mod test_io;
