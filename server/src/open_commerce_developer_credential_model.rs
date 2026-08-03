//! Production developer credential contracts and authenticated call context.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::open_commerce_developer_model::OpenCommerceDeveloperApp;

pub(crate) const PRODUCTION_CREDENTIAL_ENV: &str = "OPEN_COMMERCE_PRODUCTION_CREDENTIALS_ENABLED";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperProductionCredential {
    pub schema: &'static str,
    pub id: String,
    pub app_record_id: String,
    pub project_id: String,
    pub admission_id: String,
    pub manifest_revision: i64,
    pub environment: &'static str,
    pub scopes: Vec<String>,
    pub status: String,
    pub token_hint: String,
    pub issued_by_user_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
    pub revocation_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DeveloperProductionCredentialSecret {
    pub schema: &'static str,
    pub credential: DeveloperProductionCredential,
    pub live_token: String,
    pub token_visible_once: bool,
    pub funds_moved: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IssueDeveloperProductionCredentialRequest {
    pub expected_manifest_revision: i64,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub expires_in_days: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RevokeDeveloperProductionCredentialRequest {
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedDeveloperCredential {
    pub app: OpenCommerceDeveloperApp,
    pub environment: &'static str,
    pub credential_id: Option<String>,
    pub scopes: Option<Vec<String>>,
}

impl AuthenticatedDeveloperCredential {
    pub(crate) fn sandbox(app: OpenCommerceDeveloperApp) -> Self {
        Self {
            app,
            environment: "sandbox",
            credential_id: None,
            scopes: None,
        }
    }

    pub(crate) fn production(
        app: OpenCommerceDeveloperApp,
        credential_id: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            app,
            environment: "production",
            credential_id: Some(credential_id),
            scopes: Some(scopes),
        }
    }

    pub(crate) fn ensure_scope(&self, capability_key: &str) -> Result<()> {
        let Some(scopes) = &self.scopes else {
            return Ok(());
        };
        if !scopes.iter().any(|scope| scope == capability_key) {
            bail!("生产凭据未获准调用能力 {capability_key}");
        }
        Ok(())
    }
}

pub(crate) fn production_credentials_enabled() -> bool {
    std::env::var(PRODUCTION_CREDENTIAL_ENV)
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "yes" | "enabled"))
}
