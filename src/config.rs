use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_model: String,
    pub ollama_url: String,
    pub history_limit: usize,
    pub show_tokens: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_model: "qwen3-coder:480b-cloud".to_string(),
            ollama_url: "http://172.23.240.1:11434/api/chat".to_string(),
            history_limit: 20,
            show_tokens: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            let contents = fs::read_to_string(path).unwrap_or_default();
            toml::from_str(&contents).unwrap_or_else(|_| Self::default())
        } else {
            let config = Self::default();
            config.save();
            config
        }
    }
    
    pub fn save(&self) {
        let path = Self::get_config_path();
        let contents = toml::to_string_pretty(self).unwrap();
        fs::write(path, contents).unwrap();
    }
    
    fn get_config_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".fuchecode").join("config.toml")
    }
}
