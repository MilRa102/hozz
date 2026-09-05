use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OllamaModel {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct OllamaTagsResponse {
    pub models: Vec<OllamaModel>,
}

pub async fn list_ollama_models(base_url: &str) -> Result<Vec<String>> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        anyhow::bail!("Ollama base URL is empty");
    }

    let normalized = if base_url.ends_with('/') {
        base_url.trim_end_matches('/').to_string()
    } else {
        base_url.to_string()
    };

    let url = format!("{normalized}/api/tags");
    let response = Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .with_context(|| format!("failed to fetch Ollama models from {url}"))?;

    if !response.status().is_success() {
        anyhow::bail!("Ollama API returned status {} for {url}", response.status());
    }

    let payload: OllamaTagsResponse = response
        .json()
        .await
        .with_context(|| format!("failed to decode Ollama tags response from {url}"))?;

    let mut models = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for model in payload.models {
        let name = model.name.trim();
        if name.is_empty() || seen.contains(name) {
            continue;
        }
        seen.insert(name.to_string());
        models.push(name.to_string());
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_ollama_tags_response() {
        let payload = OllamaTagsResponse {
            models: vec![
                OllamaModel { name: "llama3.2".to_string() },
                OllamaModel { name: "qwen2.5".to_string() },
            ],
        };

        let names: Vec<String> = payload.models.into_iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["llama3.2", "qwen2.5"]);
    }

    #[test]
    fn deduplicates_model_names() {
        let models = vec![
            OllamaModel { name: "llama3".to_string() },
            OllamaModel { name: "llama3".to_string() },
            OllamaModel { name: "  qwen2.5  ".to_string() },
        ];

        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for model in models {
            let name = model.name.trim();
            if name.is_empty() || seen.contains(name) {
                continue;
            }
            seen.insert(name.to_string());
            unique.push(name.to_string());
        }

        assert_eq!(unique, vec!["llama3", "qwen2.5"]);
    }
}
