use std::sync::{LazyLock, Mutex};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::MemoryMapStore;

static EMBEDDER: LazyLock<Mutex<Option<TextEmbedding>>> =
    LazyLock::new(|| Mutex::new(None));

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

pub fn compute_blake3_hash(content: &str) -> String {
    blake3::hash(content.as_bytes())
        .to_hex()
        .to_string()
}

pub fn embed_memory_map_document_sync(content: &str) -> anyhow::Result<Vec<f32>> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut guard = EMBEDDER
        .lock()
        .map_err(|e| anyhow::anyhow!("mutex poisoned: {e}"))?;

    if guard.is_none() {
        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::AllMiniLML6V2;
        let model = TextEmbedding::try_new(options)?;
        *guard = Some(model);
    }

    let model = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("FastEmbed model not initialized"))?;

    let embeddings = model.embed(vec![content], None)?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("FastEmbed produced no vector"))
}

pub async fn embed_memory_map_document(content: &str) -> anyhow::Result<Vec<f32>> {
    let content = content.to_string();
    spawn_blocking(move || embed_memory_map_document_sync(&content)).await?
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

pub async fn search_memory_map(
    query: &str,
    limit: usize,
) -> anyhow::Result<Vec<MemoryMapHit>> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let store = MemoryMapStore;
    let records = store.all()?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let query_vector = embed_memory_map_document(query).await?;
    if query_vector.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();
    for entry in records {
        if entry.embed.is_empty() || entry.content.trim().is_empty() {
            continue;
        }
        let score = cosine_similarity(&query_vector, &entry.embed) as f64;
        if score > 0.0 {
            hits.push(MemoryMapHit {
                id: entry.id,
                content: entry.content,
                score,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);

    Ok(hits)
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

    #[test]
    fn cosine_similarity_computes_expected_value() {
        let v1 = vec![1.0, 0.0, 0.0];
        let v2 = vec![1.0, 0.0, 0.0];
        let v3 = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&v1, &v2) - 1.0).abs() < 1e-5);
        assert!((cosine_similarity(&v1, &v3) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn compute_blake3_hash_returns_hex() {
        let hash = compute_blake3_hash("test content");
        assert_eq!(hash.len(), 64);
    }
}
