use rig::{
    client::{EmbeddingsClient, Nothing},
    embeddings::{Embedding, EmbeddingModel},
    providers::{gemini, ollama},
    vector_store::{
        VectorSearchRequest, VectorStoreIndex, in_memory_store::InMemoryVectorStore,
    },
};
use serde::{Deserialize, Serialize};

use crate::{MemoryMapStore, model::ProviderKind, settings::AiPrefsReader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryMapDocument {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryMapHit {
    pub id: String,
    pub content: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct MemoryMapRetriever {
    store: MemoryMapStore,
    provider: ProviderKind,
    model: String,
    ollama_base_url: String,
    gemini_api_key: Option<String>,
}

impl MemoryMapRetriever {
    pub fn from_prefs() -> Self {
        let prefs = AiPrefsReader;
        let provider = prefs
            .memory_map_provider()
            .unwrap_or_else(|| prefs.provider().unwrap_or(ProviderKind::Gemini));
        let model = prefs.memory_map_model(provider);

        Self {
            store: MemoryMapStore,
            provider,
            model,
            ollama_base_url: prefs.ollama_base_url(),
            gemini_api_key: prefs.gemini_api_key(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryMapHit>> {
        let records = self.store.recent(100)?;
        if records.is_empty() || query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let documents = records
            .into_iter()
            .filter_map(|entry| {
                let embedding = if entry.embed.is_empty() {
                    None
                } else {
                    Some(Embedding {
                        document: entry.content.clone(),
                        vec: entry
                            .embed
                            .iter()
                            .map(|value| *value as f64)
                            .collect(),
                    })
                };
                embedding.map(|embedding| {
                    let id = entry.id.clone();
                    (
                        id.clone(),
                        MemoryMapDocument {
                            id,
                            content: entry.content,
                        },
                        vec![embedding],
                    )
                })
            })
            .collect::<Vec<_>>();

        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let results = match self.provider {
            ProviderKind::Gemini => {
                let api_key = self.gemini_api_key.clone().unwrap_or_default();
                if api_key.trim().is_empty() {
                    return Ok(Vec::new());
                }
                let client = gemini::Client::new(api_key)?;
                let model = client.embedding_model(self.model.as_str());
                let index =
                    InMemoryVectorStore::from_documents_with_ids(documents).index(model);
                let req = VectorSearchRequest::builder()
                    .query(query)
                    .samples(limit as u64)
                    .build();
                index.top_n::<MemoryMapDocument>(req).await?
            },
            ProviderKind::Ollama => {
                let client = ollama::Client::builder()
                    .api_key(Nothing)
                    .base_url(&self.ollama_base_url)
                    .build()?;
                let model = client.embedding_model(self.model.as_str());
                let index =
                    InMemoryVectorStore::from_documents_with_ids(documents).index(model);
                let req = VectorSearchRequest::builder()
                    .query(query)
                    .samples(limit as u64)
                    .build();
                index.top_n::<MemoryMapDocument>(req).await?
            },
            ProviderKind::Copilot => {
                anyhow::bail!("Copilot embeddings are not enabled for the memory map yet")
            },
        };

        Ok(results
            .into_iter()
            .filter_map(|(score, _id, doc)| {
                if doc.content.trim().is_empty() {
                    None
                } else {
                    Some(MemoryMapHit {
                        id: doc.id,
                        content: doc.content,
                        score,
                    })
                }
            })
            .collect())
    }
}

pub async fn search_memory_map(
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<MemoryMapHit>> {
    MemoryMapRetriever::from_prefs()
        .search(query, limit)
        .await
}

pub(crate) async fn embed_memory_map_document(content: &str) -> anyhow::Result<Vec<f32>> {
    let prefs = AiPrefsReader;
    let provider = prefs
        .memory_map_provider()
        .unwrap_or_else(|| prefs.provider().unwrap_or(ProviderKind::Gemini));
    let model_name = prefs.memory_map_model(provider);

    let embedding = match provider {
        ProviderKind::Gemini => {
            let api_key = prefs.gemini_api_key().unwrap_or_default();
            if api_key.trim().is_empty() {
                anyhow::bail!("a Gemini API key is required to embed the memory map");
            }
            let client = gemini::Client::new(api_key)?;
            client
                .embedding_model(model_name)
                .embed_text(content)
                .await?
        },
        ProviderKind::Ollama => {
            let client = ollama::Client::builder()
                .api_key(Nothing)
                .base_url(prefs.ollama_base_url())
                .build()?;
            client
                .embedding_model(model_name)
                .embed_text(content)
                .await?
        },
        ProviderKind::Copilot => {
            anyhow::bail!("Copilot embeddings are not enabled for the memory map yet");
        },
    };

    Ok(embedding
        .vec
        .into_iter()
        .map(|value| value as f32)
        .collect())
}

pub fn memory_map_context(entries: &[MemoryMapHit]) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let lines = entries
        .iter()
        .map(|entry| format!("- {}", entry.content.trim()))
        .collect::<Vec<_>>()
        .join("\n");

    format!("Карта памяти:\n{lines}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_map_context_uses_hit_texts() {
        let entries = vec![
            MemoryMapHit {
                id: "one".to_string(),
                content: "remember this".to_string(),
                score: 0.9,
            },
            MemoryMapHit {
                id: "two".to_string(),
                content: "and this too".to_string(),
                score: 0.8,
            },
        ];

        let context = memory_map_context(&entries);
        assert!(context.contains("Карта памяти"));
        assert!(context.contains("remember this"));
        assert!(context.contains("and this too"));
    }
}
