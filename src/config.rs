use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderDef {
    pub url: String,
}

fn default_magic_audio_dir() -> String {
    "/mnt/c/Users/ACER/Downloads/Music/ACDC".into()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub current_provider: String,
    pub default_model: String,
    pub providers: HashMap<String, ProviderDef>,
    pub timeout_secs: u64,
    pub history_limit: usize,
    pub show_tokens: bool,
    #[serde(default = "default_magic_audio_dir")]
    pub magic_audio_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert("ollama".into(), ProviderDef {
            url: "http://172.23.240.1:11434/api/chat".into(),
        });
        providers.insert("nvidia".into(), ProviderDef {
            url: "https://integrate.api.nvidia.com/v1/chat/completions".into(),
        });
        providers.insert("clawrouter".into(), ProviderDef {
            url: "http://localhost:3777/v1/chat/completions".into(),
        });
        Self {
            current_provider: "nvidia".into(),
            default_model: "deepseek-ai/deepseek-v4-flash".into(),
            providers,
            timeout_secs: 15,
            history_limit: 20,
            show_tokens: true,
            magic_audio_dir: "/mnt/c/Users/ACER/Downloads/Music/ACDC".into(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::get_config_path();
        let config = if path.exists() {
            let contents = fs::read_to_string(path).unwrap_or_default();
            toml::from_str(&contents).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        };
        config.save();
        config
    }

    pub fn save(&self) {
        let path = Self::get_config_path();
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(contents) = toml::to_string_pretty(self) {
            let _ = fs::write(path, contents);
        }
    }

    pub fn api_url(&self) -> String {
        self.providers
            .get(&self.current_provider)
            .map(|p| p.url.clone())
            .unwrap_or_else(|| {
                self.providers.values().next()
                    .map(|p| p.url.clone())
                    .unwrap_or_default()
            })
    }

    pub fn provider_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.providers.keys().cloned().collect();
        names.sort();
        names
    }

    fn get_config_path() -> PathBuf {
        dirs::home_dir().expect("HOME not set").join(".fuchecode").join("config.toml")
    }
}
