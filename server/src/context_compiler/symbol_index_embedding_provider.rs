use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::{
    symbol_index_embedding_openai::OpenAiCompatibleEmbeddingProvider,
    symbol_index_vector_types::{
        LOCAL_HASH_VECTOR_DIM, LOCAL_HASH_VECTOR_MODEL, REMOTE_EMBEDDING_MODEL_PREFIXES,
        SUPPORTED_EMBEDDING_MODELS,
    },
};

pub(crate) trait SymbolEmbeddingProvider: std::fmt::Debug {
    fn model(&self) -> &str;
    fn dim(&self) -> usize;
    fn embed_text(&self, text: &str) -> Result<Vec<f32>>;

    fn embed_chunk(&self, content: &str, summary: Option<&str>) -> Result<Vec<f32>> {
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

    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
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
        Ok(vector)
    }
}

#[derive(Clone, Default)]
pub(crate) struct SymbolEmbeddingProviderContext {
    pub(crate) api_base: Option<String>,
    pub(crate) api_key: Option<String>,
    pub(crate) source: Option<String>,
}

impl std::fmt::Debug for SymbolEmbeddingProviderContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolEmbeddingProviderContext")
            .field("api_base", &self.api_base)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("source", &self.source)
            .finish()
    }
}

impl SymbolEmbeddingProviderContext {
    pub(crate) fn from_agent(api_base: &str, api_key: &str, source: impl Into<String>) -> Self {
        Self {
            api_base: Some(api_base.trim().trim_end_matches('/').to_string()),
            api_key: Some(api_key.to_string()),
            source: Some(source.into()),
        }
    }

    pub(crate) fn remote_provider_configured(&self) -> bool {
        remote_config(Some(self)).is_some()
    }

    pub(crate) fn remote_provider_source(&self) -> Option<String> {
        remote_config(Some(self)).map(|config| config.source)
    }
}

#[derive(Clone)]
struct RemoteEmbeddingConfig {
    api_base: String,
    api_key: String,
    source: String,
}

pub(crate) fn resolve_embedding_provider(
    model: &str,
    context: Option<&SymbolEmbeddingProviderContext>,
) -> Result<Box<dyn SymbolEmbeddingProvider>> {
    let model = model.trim();
    if model == LOCAL_HASH_VECTOR_MODEL {
        return Ok(Box::new(LocalHashEmbeddingProvider::new(model.to_string())));
    }

    if let Some(api_model) = remote_embedding_api_model(model) {
        let config = remote_config(context).ok_or_else(|| {
            anyhow::anyhow!(
                "embedding 模型 `{}` 需要远程 provider；请配置用户 API key 或 ELON_EMBEDDING_API_KEY",
                model
            )
        })?;
        return Ok(Box::new(OpenAiCompatibleEmbeddingProvider::new(
            model.to_string(),
            api_model.to_string(),
            config.api_base,
            config.api_key,
        )?));
    }

    bail!(
        "embedding 模型 `{}` 暂未配置 provider；当前支持: {}",
        model,
        SUPPORTED_EMBEDDING_MODELS.join(", ")
    )
}

pub(crate) fn is_remote_embedding_model(model: &str) -> bool {
    remote_embedding_api_model(model).is_some()
}

fn remote_embedding_api_model(model: &str) -> Option<&str> {
    let model = model.trim();
    REMOTE_EMBEDDING_MODEL_PREFIXES
        .iter()
        .find_map(|prefix| model.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn remote_config(
    context: Option<&SymbolEmbeddingProviderContext>,
) -> Option<RemoteEmbeddingConfig> {
    context
        .and_then(|context| {
            let api_base = clean(context.api_base.as_deref())?;
            let api_key = clean(context.api_key.as_deref())?;
            Some(RemoteEmbeddingConfig {
                api_base,
                api_key,
                source: context
                    .source
                    .clone()
                    .unwrap_or_else(|| "agent_config".to_string()),
            })
        })
        .or_else(remote_config_from_env)
}

fn remote_config_from_env() -> Option<RemoteEmbeddingConfig> {
    let api_key = std::env::var("ELON_EMBEDDING_API_KEY")
        .ok()
        .and_then(|value| clean(Some(&value)))
        .or_else(|| {
            std::env::var("OPENAI_API_KEY")
                .ok()
                .and_then(|value| clean(Some(&value)))
        })?;
    let api_base = std::env::var("ELON_EMBEDDING_API_BASE")
        .ok()
        .and_then(|value| clean(Some(&value)))
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    Some(RemoteEmbeddingConfig {
        api_base,
        api_key,
        source: "env".to_string(),
    })
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_end_matches('/').to_string())
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
        let provider = resolve_embedding_provider(LOCAL_HASH_VECTOR_MODEL, None).expect("provider");

        let first = provider.embed_text("repo map symbol index").unwrap();
        let second = provider.embed_text("repo map symbol index").unwrap();

        assert_eq!(provider.dim(), LOCAL_HASH_VECTOR_DIM);
        assert_eq!(first, second);
        assert!((vector_norm(&first) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn unknown_embedding_models_require_a_provider() {
        let err = resolve_embedding_provider("bge-m3", None).expect_err("unsupported provider");

        assert!(err.to_string().contains("暂未配置 provider"));
        assert!(err.to_string().contains(LOCAL_HASH_VECTOR_MODEL));
    }

    #[test]
    fn remote_embedding_model_resolves_with_agent_context_without_calling_api() {
        let context = SymbolEmbeddingProviderContext::from_agent(
            "https://api.example.com/v1/",
            "sk-test",
            "user_api_key_proxy",
        );

        let provider = resolve_embedding_provider("openai:text-embedding-3-small", Some(&context))
            .expect("remote provider");

        assert_eq!(provider.model(), "openai:text-embedding-3-small");
        assert!(context.remote_provider_configured());
        assert_eq!(
            context.remote_provider_source().as_deref(),
            Some("user_api_key_proxy")
        );
    }
}
