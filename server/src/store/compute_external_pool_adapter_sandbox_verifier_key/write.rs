use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_sandbox_verifier_key::*,
    store::{new_id, Store},
};

use super::{read::*, types::*};

impl Store {
    pub(crate) fn register_external_pool_adapter_sandbox_verifier_key(
        &self,
        input: RegisterExternalPoolAdapterSandboxVerifierKey,
    ) -> Result<ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt> {
        validate_registration_input(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            record_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_registration_replay(&stored, &input)?;
            let result = ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt {
                key_record: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(result);
        }
        if record_by_key_id_on(&tx, &input.key_id)?.is_some()
            || trust_role_exists(
                &tx,
                "compute_external_pool_adapter_artifact_signing_keys",
                &input.key_id,
            )?
            || trust_role_exists(
                &tx,
                "compute_external_pool_adapter_scanner_keys",
                &input.key_id,
            )?
        {
            bail!("sandbox verifier key is already assigned to another immutable trust role");
        }
        let timestamp = now();
        let registration = ExternalPoolAdapterSandboxVerifierKeyRegistration {
            verifier_operator: input.verifier_operator,
            verifier_product: input.verifier_product,
            key_id: input.key_id,
            algorithm: SANDBOX_VERIFIER_KEY_ALGORITHM.into(),
            public_key_pem: input.public_key_pem,
            actor_kind: SANDBOX_VERIFIER_KEY_ACTOR_KIND.into(),
            created_by_admin_user_id: input.created_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            created_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: SANDBOX_VERIFIER_KEY_STATUS_PENDING.into(),
            conformance_report_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            vulnerability_report_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            adapter_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            route_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
        };
        let mut record = ExternalPoolAdapterSandboxVerifierKeyRecord {
            schema: SANDBOX_VERIFIER_KEY_RECORD_SCHEMA.into(),
            key_record_id: new_id("external_pool_adapter_sandbox_verifier_key"),
            key_record_digest: String::new(),
            registration_material_digest: sandbox_verifier_key_registration_digest(&registration)?,
            canonicalization: SANDBOX_VERIFIER_KEY_CANONICALIZATION.into(),
            digest_algorithm: SANDBOX_VERIFIER_KEY_DIGEST_ALGORITHM.into(),
            registration,
        };
        record.key_record_digest = sandbox_verifier_key_record_json_and_digest(&record)?.1;
        validate_sandbox_verifier_key_record(&record)?;
        let (json, digest) = sandbox_verifier_key_record_json_and_digest(&record)?;
        if digest != record.key_record_digest {
            bail!("sandbox-verifier-key digest drifted");
        }
        insert_record(&tx, &record, &json)?;
        let stored = record_by_id_on(&tx, &record.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox-verifier-key disappeared"))?;
        if stored.record != record || stored.json != json {
            bail!("sandbox-verifier-key changed during readback");
        }
        let result = ExternalPoolAdapterSandboxVerifierKeyRegistrationWriteReceipt {
            key_record: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn activate_external_pool_adapter_sandbox_verifier_key(
        &self,
        input: ActivateExternalPoolAdapterSandboxVerifierKey,
    ) -> Result<ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt> {
        validate_transition_input(
            &input.key_record_id,
            &input.expected_key_record_digest,
            &input.activated_by_admin_user_id,
            None,
            &input.confirmation,
            &input.idempotency_scope,
            &input.idempotency_key,
            SANDBOX_VERIFIER_KEY_ACTIVATE_CONFIRMATION,
        )?;
        self.write_sandbox_verifier_key_transition(
            input.key_record_id,
            input.expected_key_record_digest,
            input.activated_by_admin_user_id,
            None,
            input.confirmation,
            input.idempotency_scope,
            input.idempotency_key,
        )
    }

    pub(crate) fn revoke_external_pool_adapter_sandbox_verifier_key(
        &self,
        input: RevokeExternalPoolAdapterSandboxVerifierKey,
    ) -> Result<ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt> {
        validate_transition_input(
            &input.key_record_id,
            &input.expected_key_record_digest,
            &input.revoked_by_admin_user_id,
            Some(&input.reason),
            &input.confirmation,
            &input.idempotency_scope,
            &input.idempotency_key,
            SANDBOX_VERIFIER_KEY_REVOKE_CONFIRMATION,
        )?;
        self.write_sandbox_verifier_key_transition(
            input.key_record_id,
            input.expected_key_record_digest,
            input.revoked_by_admin_user_id,
            Some(input.reason),
            input.confirmation,
            input.idempotency_scope,
            input.idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_sandbox_verifier_key_transition(
        &self,
        id: String,
        expected_digest: String,
        actor: String,
        reason: Option<String>,
        confirmation: String,
        scope: String,
        key: String,
    ) -> Result<ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt> {
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let root = record_by_id_on(&tx, &id)?
            .ok_or_else(|| anyhow::anyhow!("sandbox-verifier-key was not found"))?;
        if let Some(stored) = transition_by_idempotency_on(&tx, &scope, &key)? {
            ensure_transition_replay(
                &stored,
                &id,
                &expected_digest,
                &actor,
                reason.as_deref(),
                &confirmation,
                &scope,
                &key,
            )?;
            let result = ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt {
                key_record: root.summary(),
                transition: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(result);
        }
        if root.record.key_record_digest != expected_digest {
            bail!("sandbox-verifier-key root is not exact");
        }
        let kind = if reason.is_some() {
            "revocation"
        } else {
            "activation"
        };
        if transition_by_kind_on(&tx, &id, kind)?.is_some()
            || (kind == "activation"
                && (actor == root.record.registration.created_by_admin_user_id
                    || transition_by_kind_on(&tx, &id, "revocation")?.is_some()))
            || (kind == "revocation" && transition_by_kind_on(&tx, &id, "activation")?.is_none())
        {
            bail!("sandbox-verifier-key transition is not allowed from current state");
        }
        let timestamp = now();
        let transition = ExternalPoolAdapterSandboxVerifierKeyTransition {
            key_record_id: id,
            key_record_digest: expected_digest,
            key_id: root.record.registration.key_id.clone(),
            verifier_operator: root.record.registration.verifier_operator.clone(),
            verifier_product: root.record.registration.verifier_product.clone(),
            actor_kind: SANDBOX_VERIFIER_KEY_ACTOR_KIND.into(),
            actor_user_id: actor,
            reason,
            confirmation,
            idempotency_scope: scope,
            idempotency_key: key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: if kind == "activation" {
                SANDBOX_VERIFIER_KEY_STATUS_ACTIVE
            } else {
                SANDBOX_VERIFIER_KEY_STATUS_REVOKED
            }
            .into(),
            conformance_report_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            vulnerability_report_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            adapter_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
            route_effect: SANDBOX_VERIFIER_KEY_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt {
            schema: if kind == "activation" {
                SANDBOX_VERIFIER_KEY_ACTIVATION_SCHEMA
            } else {
                SANDBOX_VERIFIER_KEY_REVOCATION_SCHEMA
            }
            .into(),
            transition_receipt_id: new_id("external_pool_adapter_sandbox_verifier_key_transition"),
            transition_receipt_digest: String::new(),
            transition_material_digest: sandbox_verifier_key_transition_digest(&transition)?,
            canonicalization: SANDBOX_VERIFIER_KEY_CANONICALIZATION.into(),
            digest_algorithm: SANDBOX_VERIFIER_KEY_DIGEST_ALGORITHM.into(),
            transition,
        };
        receipt.transition_receipt_digest =
            sandbox_verifier_key_transition_json_and_digest(&receipt)?.1;
        validate_sandbox_verifier_key_transition(&receipt)?;
        let (json, digest) = sandbox_verifier_key_transition_json_and_digest(&receipt)?;
        if digest != receipt.transition_receipt_digest {
            bail!("sandbox-verifier-key transition digest drifted");
        }
        insert_transition(&tx, kind, &receipt, &json)?;
        let stored = transition_by_kind_on(&tx, &receipt.transition.key_record_id, kind)?
            .ok_or_else(|| anyhow::anyhow!("sandbox-verifier-key transition disappeared"))?;
        if stored.receipt != receipt || stored.json != json {
            bail!("sandbox-verifier-key transition changed during readback");
        }
        let result = ExternalPoolAdapterSandboxVerifierKeyTransitionWriteReceipt {
            key_record: root.summary(),
            transition: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(result)
    }
}

fn insert_record(
    tx: &rusqlite::Transaction<'_>,
    record: &ExternalPoolAdapterSandboxVerifierKeyRecord,
    json: &str,
) -> Result<()> {
    let item = &record.registration;
    tx.execute("INSERT INTO compute_external_pool_adapter_sandbox_verifier_keys(
      key_record_id,key_record_schema,key_record_digest,key_record_json,registration_material_digest,
      canonicalization,digest_algorithm,verifier_operator,verifier_product,key_id,algorithm,public_key_pem,
      actor_kind,created_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,created_at,recorded_at,
      currentness_effect,conformance_report_effect,vulnerability_report_effect,adapter_effect,route_effect)
      VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",
      params![record.key_record_id,record.schema,record.key_record_digest,json,record.registration_material_digest,
        record.canonicalization,record.digest_algorithm,item.verifier_operator,item.verifier_product,item.key_id,item.algorithm,
        item.public_key_pem,item.actor_kind,item.created_by_admin_user_id,item.confirmation,item.idempotency_scope,item.idempotency_key,
        item.created_at,item.recorded_at,item.currentness_effect,item.conformance_report_effect,item.vulnerability_report_effect,
        item.adapter_effect,item.route_effect])?;
    Ok(())
}

fn insert_transition(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    receipt: &ExternalPoolAdapterSandboxVerifierKeyTransitionReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.transition;
    tx.execute("INSERT INTO compute_external_pool_adapter_sandbox_verifier_key_transitions(
      transition_receipt_id,transition_receipt_schema,transition_receipt_digest,transition_receipt_json,
      transition_material_digest,transition_kind,canonicalization,digest_algorithm,key_record_id,key_record_digest,
      key_id,verifier_operator,verifier_product,actor_kind,actor_user_id,reason,confirmation,idempotency_scope,
      idempotency_key,occurred_at,recorded_at,currentness_effect,conformance_report_effect,vulnerability_report_effect,adapter_effect,route_effect)
      VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
      params![receipt.transition_receipt_id,receipt.schema,receipt.transition_receipt_digest,json,receipt.transition_material_digest,
        kind,receipt.canonicalization,receipt.digest_algorithm,item.key_record_id,item.key_record_digest,item.key_id,item.verifier_operator,
        item.verifier_product,item.actor_kind,item.actor_user_id,item.reason,item.confirmation,item.idempotency_scope,item.idempotency_key,
        item.occurred_at,item.recorded_at,item.currentness_effect,item.conformance_report_effect,item.vulnerability_report_effect,item.adapter_effect,item.route_effect])?;
    Ok(())
}

fn trust_role_exists(tx: &rusqlite::Transaction<'_>, table: &str, key_id: &str) -> Result<bool> {
    Ok(tx
        .query_row(
            &format!("SELECT 1 FROM {table} WHERE key_id=?1"),
            [key_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ensure_registration_replay(
    stored: &StoredKeyRecord,
    input: &RegisterExternalPoolAdapterSandboxVerifierKey,
) -> Result<()> {
    let item = &stored.record.registration;
    if item.verifier_operator != input.verifier_operator
        || item.verifier_product != input.verifier_product
        || item.key_id != input.key_id
        || item.public_key_pem != input.public_key_pem
        || item.created_by_admin_user_id != input.created_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("sandbox-verifier-key registration replay conflicts with immutable history");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn ensure_transition_replay(
    stored: &StoredTransition,
    id: &str,
    digest: &str,
    actor: &str,
    reason: Option<&str>,
    confirmation: &str,
    scope: &str,
    key: &str,
) -> Result<()> {
    let item = &stored.receipt.transition;
    if item.key_record_id != id
        || item.key_record_digest != digest
        || item.actor_user_id != actor
        || item.reason.as_deref() != reason
        || item.confirmation != confirmation
        || item.idempotency_scope != scope
        || item.idempotency_key != key
    {
        bail!("sandbox-verifier-key transition replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_registration_input(
    input: &RegisterExternalPoolAdapterSandboxVerifierKey,
) -> Result<()> {
    validate_exact(&input.verifier_operator, 160)?;
    validate_exact(&input.verifier_product, 160)?;
    validate_digest(&input.key_id)?;
    validate_exact(&input.created_by_admin_user_id, 160)?;
    validate_exact(&input.idempotency_scope, 200)?;
    validate_exact(&input.idempotency_key, 160)?;
    if input.confirmation != SANDBOX_VERIFIER_KEY_REGISTER_CONFIRMATION
        || input.public_key_pem.is_empty()
        || input.public_key_pem.len() > 16 * 1024
    {
        bail!("sandbox-verifier-key registration input is invalid");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_transition_input(
    id: &str,
    digest: &str,
    actor: &str,
    reason: Option<&String>,
    confirmation: &str,
    scope: &str,
    key: &str,
    expected_confirmation: &str,
) -> Result<()> {
    validate_exact(id, 160)?;
    validate_digest(digest)?;
    validate_exact(actor, 160)?;
    validate_exact(scope, 200)?;
    validate_exact(key, 160)?;
    if let Some(reason) = reason {
        validate_exact(reason, 2_000)?;
        if reason.chars().count() < 8 {
            bail!("sandbox-verifier-key revocation reason is too short");
        }
    }
    if confirmation != expected_confirmation {
        bail!("sandbox-verifier-key transition confirmation is invalid");
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
