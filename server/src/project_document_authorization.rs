//! Shared authorization policy for AI-assisted project document organization.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DocumentAutomationMode {
    GitBackedFull,
    TrustedReversible,
    ReviewAll,
    SuggestionsOnly,
}

impl Default for DocumentAutomationMode {
    fn default() -> Self {
        Self::GitBackedFull
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DocumentAuthorization {
    pub mode: DocumentAutomationMode,
    pub auto_authorized: bool,
}

pub(crate) fn authorize_document_apply(
    mode: DocumentAutomationMode,
    reviewed: bool,
) -> Result<DocumentAuthorization> {
    match mode {
        DocumentAutomationMode::GitBackedFull | DocumentAutomationMode::TrustedReversible => {
            Ok(DocumentAuthorization {
                mode,
                auto_authorized: !reviewed,
            })
        }
        DocumentAutomationMode::ReviewAll if reviewed => Ok(DocumentAuthorization {
            mode,
            auto_authorized: false,
        }),
        DocumentAutomationMode::ReviewAll => {
            bail!("review_all 模式必须显式传入 reviewed=true")
        }
        DocumentAutomationMode::SuggestionsOnly => {
            bail!("suggestions_only 模式只允许生成建议，不能应用变更")
        }
    }
}

pub(crate) fn operation_permission_granted(
    authorization: DocumentAuthorization,
    explicitly_allowed: bool,
) -> bool {
    matches!(
        authorization.mode,
        DocumentAutomationMode::GitBackedFull | DocumentAutomationMode::TrustedReversible
    ) || explicitly_allowed
}
