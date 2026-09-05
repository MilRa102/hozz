use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_gemini_models")]
    pub gemini_models: Vec<String>,
    #[serde(default = "default_copilot_models")]
    pub copilot_models: Vec<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            gemini_models: default_gemini_models(),
            copilot_models: default_copilot_models(),
        }
    }
}

fn default_gemini_models() -> Vec<String> {
    vec![
        "gemini-flash-latest".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-3.5-flash-lite".to_string(),
        "gemini-3.6-flash".to_string(),
        "gemini-3.8-flash".to_string(),
    ]
}

fn default_copilot_models() -> Vec<String> {
    vec!["gpt-5.3-codex".to_string()]
}

impl AiConfig {
    pub fn normalized_lists(&self) -> (Vec<String>, Vec<String>) {
        let gemini = normalize_model_list(&self.gemini_models);
        let copilot = normalize_model_list(&self.copilot_models);
        (gemini, copilot)
    }
}

fn normalize_model_list(models: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();

    for model in models {
        let cleaned = model.trim();
        if cleaned.is_empty() || seen.contains(cleaned) {
            continue;
        }
        seen.insert(cleaned.to_string());
        normalized.push(cleaned.to_string());
    }

    normalized
}
