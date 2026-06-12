pub struct Config {
    pub model: String,
    pub ollama_url: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            model: "qwen3-coder:480b-cloud".to_string(),
            ollama_url: "http://172.23.240.1:11434/api/chat".to_string(),  // Changed from /generate
        }
    }
}
