//! Non-secret capability contract for provider credential backup boundaries.

use serde_json::{json, Value};

pub(crate) fn provider_vault_contract(provider_id: &str) -> Value {
    match provider_id {
        "codex_cli" => json!({
            "schema": "elon.provider_credential_vault.v1",
            "backup_supported": true,
            "restore_supported": true,
            "explicit_consent_required": true,
            "automatic_backup": false,
            "credential_export_to_ui": false,
            "credential_owner": "codex_cli",
            "source": "official_codex_auth_store",
            "cloud_storage": "aes_256_gcm_ciphertext_only",
            "restore_target": "managed_temporary_codex_home",
            "overwrites_default_cli_home": false,
            "client_encrypted_envelope": "reserved_not_implemented"
        }),
        "gemini_cli" | "claude_cli" | "copilot_cli" => json!({
            "schema": "elon.provider_credential_vault.v1",
            "backup_supported": false,
            "restore_supported": false,
            "explicit_consent_required": true,
            "automatic_backup": false,
            "credential_export_to_ui": false,
            "credential_owner": provider_id,
            "reason_code": "official_credential_export_contract_unavailable",
            "client_encrypted_envelope": "reserved_not_implemented"
        }),
        _ => json!({
            "schema": "elon.provider_credential_vault.v1",
            "backup_supported": false,
            "restore_supported": false,
            "automatic_backup": false,
            "credential_export_to_ui": false,
            "reason_code": "provider_vault_not_available"
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_contract_never_turns_login_into_automatic_credential_upload() {
        let codex = provider_vault_contract("codex_cli");
        assert_eq!(codex["explicit_consent_required"], true);
        assert_eq!(codex["automatic_backup"], false);
        assert_eq!(codex["credential_export_to_ui"], false);
        let gemini = provider_vault_contract("gemini_cli");
        assert_eq!(gemini["backup_supported"], false);
    }
}
