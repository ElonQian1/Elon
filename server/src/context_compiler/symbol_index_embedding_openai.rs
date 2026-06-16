use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;

use super::symbol_index_embedding_provider::{vector_norm, SymbolEmbeddingProvider};

const EMBEDDING_TIMEOUT_SECS: u64 = 60;

#[derive(Clone)]
pub(crate) struct OpenAiCompatibleEmbeddingProvider {
    storage_model: String,
    api_model: String,
    api_base: String,
    api_key: String,
    client: Client,
}

impl std::fmt::Debug for OpenAiCompatibleEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatibleEmbeddingProvider")
            .field("storage_model", &self.storage_model)
            .field("api_model", &self.api_model)
            .field("api_base", &self.api_base)
            .field("api_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatibleEmbeddingProvider {
    pub(crate) fn new(
        storage_model: String,
        api_model: String,
        api_base: String,
        api_key: String,
    ) -> Result<Self> {
        let api_base = api_base.trim().trim_end_matches('/').to_string();
        if api_base.is_empty() {
            bail!("远程 embedding provider 缺少 api_base");
        }
        if api_key.trim().is_empty() {
            bail!("远程 embedding provider 缺少 api_key");
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(EMBEDDING_TIMEOUT_SECS))
            .build()
            .context("创建远程 embedding HTTP client 失败")?;

        Ok(Self {
            storage_model,
            api_model,
            api_base,
            api_key,
            client,
        })
    }
}

impl SymbolEmbeddingProvider for OpenAiCompatibleEmbeddingProvider {
    fn model(&self) -> &str {
        &self.storage_model
    }

    fn dim(&self) -> usize {
        0
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let input = text.trim();
        if input.is_empty() {
            bail!("embedding 输入不能为空");
        }

        let url = format!("{}/embeddings", self.api_base);
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.api_model,
                "input": input,
                "encoding_format": "float"
            }))
            .send()
            .context("调用远程 embedding 接口失败")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            bail!(
                "远程 embedding 接口返回 {}：{}",
                status,
                sanitize_error_body(&body)
            );
        }

        let body: EmbeddingResponse = response.json().context("解析远程 embedding 响应失败")?;
        let Some(first) = body.data.into_iter().next() else {
            bail!("远程 embedding 响应缺少 data[0].embedding");
        };
        if first.embedding.is_empty() {
            bail!("远程 embedding 响应向量为空");
        }

        let mut vector = first.embedding;
        normalize_vector(&mut vector);
        Ok(vector)
    }
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
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

fn sanitize_error_body(body: &str) -> String {
    let compact = body.lines().collect::<Vec<_>>().join(" ");
    compact.chars().take(240).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_error_body_is_bounded() {
        let body = "x".repeat(500);
        assert_eq!(sanitize_error_body(&body).chars().count(), 240);
    }

    #[test]
    fn remote_provider_rejects_missing_key() {
        let err = OpenAiCompatibleEmbeddingProvider::new(
            "openai:text-embedding-3-small".to_string(),
            "text-embedding-3-small".to_string(),
            "https://api.example.com/v1".to_string(),
            " ".to_string(),
        )
        .expect_err("missing api key");

        assert!(err.to_string().contains("api_key"));
    }

    #[test]
    fn debug_output_redacts_api_key() {
        let provider = OpenAiCompatibleEmbeddingProvider::new(
            "openai:text-embedding-3-small".to_string(),
            "text-embedding-3-small".to_string(),
            "https://api.example.com/v1".to_string(),
            "sk-secret".to_string(),
        )
        .expect("provider");
        let debug = format!("{provider:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-secret"));
    }
}
