use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::symbol_index_vector_types::{
    LOCAL_HASH_VECTOR_DIM, LOCAL_HASH_VECTOR_MODEL, SUPPORTED_EMBEDDING_MODELS,
};

pub(crate) trait SymbolEmbeddingProvider: std::fmt::Debug {
    fn model(&self) -> &str;
    fn dim(&self) -> usize;
    fn embed_text(&self, text: &str) -> Vec<f32>;

    fn embed_chunk(&self, content: &str, summary: Option<&str>) -> Vec<f32> {
        let text = match summary.map(str::trim).filter(|value| !value.is_empty()) {
            Some(summary) => format!("{summary}\n{content}"),
            None => content.to_string(),
        };
        self.embed_text(&text)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LocalHashEmbeddingProvider {
    model: String,
}

impl LocalHashEmbeddingProvider {
    fn new(model: String) -> Self {
        Self { model }
    }
}

impl SymbolEmbeddingProvider for LocalHashEmbeddingProvider {
    fn model(&self) -> &str {
        &self.model
    }

    fn dim(&self) -> usize {
        LOCAL_HASH_VECTOR_DIM
    }

    fn embed_text(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0_f32; LOCAL_HASH_VECTOR_DIM];
        let mut freqs = std::collections::HashMap::<String, usize>::new();
        for term in embedding_terms(text) {
            *freqs.entry(term).or_default() += 1;
        }
        for (term, count) in freqs {
            let digest = Sha256::digest(term.as_bytes());
            let idx = u16::from_le_bytes([digest[0], digest[1]]) as usize % LOCAL_HASH_VECTOR_DIM;
            let sign = if digest[2] & 1 == 0 { 1.0 } else { -1.0 };
            vector[idx] += sign * (count as f32).ln_1p();
        }
        normalize_vector(&mut vector);
        vector
    }
}

pub(crate) fn resolve_embedding_provider(model: &str) -> Result<Box<dyn SymbolEmbeddingProvider>> {
    let model = model.trim();
    if model == LOCAL_HASH_VECTOR_MODEL {
        return Ok(Box::new(LocalHashEmbeddingProvider::new(model.to_string())));
    }

    bail!(
        "embedding 模型 `{}` 暂未配置 provider；当前支持: {}",
        model,
        SUPPORTED_EMBEDDING_MODELS.join(", ")
    )
}

pub(crate) fn embedding_terms(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn normalize_vector(vector: &mut [f32]) {
    let norm = vector_norm(vector);
    if norm == 0.0 {
        return;
    }
    for value in vector {
        *value /= norm;
    }
}

pub(crate) fn vector_norm(vector: &[f32]) -> f32 {
    vector.iter().map(|value| value * value).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_hash_embeddings_are_normalized_and_stable() {
        let provider = resolve_embedding_provider(LOCAL_HASH_VECTOR_MODEL).expect("provider");

        let first = provider.embed_text("repo map symbol index");
        let second = provider.embed_text("repo map symbol index");

        assert_eq!(provider.dim(), LOCAL_HASH_VECTOR_DIM);
        assert_eq!(first, second);
        assert!((vector_norm(&first) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn unknown_embedding_models_require_a_provider() {
        let err = resolve_embedding_provider("bge-m3").expect_err("unsupported provider");

        assert!(err.to_string().contains("暂未配置 provider"));
        assert!(err.to_string().contains(LOCAL_HASH_VECTOR_MODEL));
    }
}
