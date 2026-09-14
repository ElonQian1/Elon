use serde::{Deserialize, Serialize};

pub(super) const ENVIRONMENT: &str = "native_paper";
pub(super) const MAX_LIFETIME: i64 = 300;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum Scope {
    #[serde(rename = "native_grid.read")]
    Read,
    #[serde(rename = "native_grid.create")]
    Create,
    #[serde(rename = "native_grid.control")]
    Control,
}

// Credentials and requests deliberately have no Debug implementation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IssueRequest {
    pub schema: String,
    pub environment: String,
    pub scopes: Vec<Scope>,
    pub explicit_consent: bool,
    pub confirmation: String,
}

impl IssueRequest {
    pub fn validate(&self) -> Result<Vec<Scope>, Error> {
        if self.schema != "yilong.quant.native_grid_issue.v1"
            || self.environment != ENVIRONMENT
            || !self.explicit_consent
            || self.confirmation != "授权使用原生模拟网格"
            || self.scopes.is_empty()
            || self.scopes.len() > 3
        {
            return Err(Error::InvalidInput);
        }
        let mut scopes = self.scopes.clone();
        scopes.sort_unstable();
        scopes.dedup();
        if scopes.len() != self.scopes.len() {
            return Err(Error::InvalidInput);
        }
        Ok(scopes)
    }
}

#[derive(Serialize)]
pub(super) struct GrantResponse {
    pub schema: &'static str,
    pub token_type: &'static str,
    pub access_token: String,
    pub grant_id: String,
    pub subject_ref: String,
    pub scopes: Vec<Scope>,
    pub expires_in: i64,
    pub expires_at_unix: i64,
    pub environment: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Error {
    InvalidInput,
    Unauthorized,
    Disabled,
    Misconfigured,
    Unavailable,
}
