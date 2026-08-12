use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_credential_verifier::*,
    store::{new_id, Store},
};

use super::{read::*, types::*};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

impl Store {
    pub(crate) fn register_external_pool_adapter_credential_verifier(
        &self,
        input: RegisterExternalPoolAdapterCredentialVerifier,
    ) -> Result<ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt> {
        validate_registration_input(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            record_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_registration_replay(&stored, &input)?;
            let result = ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt {
                verifier_record: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(result);
        }
        if record_by_identity_on(
            &tx,
            &input.verification_kind,
            &input.verifier_id,
            input.verifier_revision,
        )?
        .is_some()
        {
            bail!("credential-verifier identity revision is already registered");
        }

        let timestamp = now();
        let registration = ExternalPoolAdapterCredentialVerifierRegistration {
            verifier_operator: input.verifier_operator,
            verifier_product: input.verifier_product,
            verification_kind: input.verification_kind,
            verifier_id: input.verifier_id,
            verifier_revision: input.verifier_revision,
            verifier_digest: input.verifier_digest,
            actor_kind: CREDENTIAL_VERIFIER_ACTOR_KIND.into(),
            created_by_admin_user_id: input.created_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            created_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: CREDENTIAL_VERIFIER_STATUS_PENDING.into(),
            credential_receipt_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            adapter_adoption_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            route_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            execution_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
        };
        let mut record = ExternalPoolAdapterCredentialVerifierRecord {
            schema: CREDENTIAL_VERIFIER_RECORD_SCHEMA.into(),
            verifier_record_id: new_id("external_pool_adapter_credential_verifier"),
            verifier_record_digest: String::new(),
            registration_material_digest: credential_verifier_registration_digest(&registration)?,
            canonicalization: CREDENTIAL_VERIFIER_CANONICALIZATION.into(),
            digest_algorithm: CREDENTIAL_VERIFIER_DIGEST_ALGORITHM.into(),
            registration,
        };
        record.verifier_record_digest = credential_verifier_record_json_and_digest(&record)?.1;
        validate_credential_verifier_record(&record)?;
        let (json, digest) = credential_verifier_record_json_and_digest(&record)?;
        if digest != record.verifier_record_digest {
            bail!("credential-verifier digest drifted");
        }
        insert_record(&tx, &record, &json)?;
        let stored = record_by_id_on(&tx, &record.verifier_record_id)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier disappeared"))?;
        if stored.record != record || stored.json != json {
            bail!("credential-verifier changed during readback");
        }
        let result = ExternalPoolAdapterCredentialVerifierRegistrationWriteReceipt {
            verifier_record: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(result)
    }

    pub(crate) fn activate_external_pool_adapter_credential_verifier(
        &self,
        input: ActivateExternalPoolAdapterCredentialVerifier,
    ) -> Result<ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt> {
        validate_transition_input(
            &input.verifier_record_id,
            &input.expected_verifier_record_digest,
            &input.activated_by_admin_user_id,
            None,
            &input.confirmation,
            &input.idempotency_scope,
            &input.idempotency_key,
            CREDENTIAL_VERIFIER_ACTIVATE_CONFIRMATION,
        )?;
        self.write_credential_verifier_transition(
            input.verifier_record_id,
            input.expected_verifier_record_digest,
            input.activated_by_admin_user_id,
            None,
            input.confirmation,
            input.idempotency_scope,
            input.idempotency_key,
        )
    }

    pub(crate) fn revoke_external_pool_adapter_credential_verifier(
        &self,
        input: RevokeExternalPoolAdapterCredentialVerifier,
    ) -> Result<ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt> {
        validate_transition_input(
            &input.verifier_record_id,
            &input.expected_verifier_record_digest,
            &input.revoked_by_admin_user_id,
            Some(&input.reason),
            &input.confirmation,
            &input.idempotency_scope,
            &input.idempotency_key,
            CREDENTIAL_VERIFIER_REVOKE_CONFIRMATION,
        )?;
        self.write_credential_verifier_transition(
            input.verifier_record_id,
            input.expected_verifier_record_digest,
            input.revoked_by_admin_user_id,
            Some(input.reason),
            input.confirmation,
            input.idempotency_scope,
            input.idempotency_key,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn write_credential_verifier_transition(
        &self,
        id: String,
        expected_digest: String,
        actor: String,
        reason: Option<String>,
        confirmation: String,
        scope: String,
        key: String,
    ) -> Result<ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt> {
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let root = record_by_id_on(&tx, &id)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier was not found"))?;
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
            let result = ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt {
                verifier_record: root.summary(),
                transition: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(result);
        }
        if root.record.verifier_record_digest != expected_digest {
            bail!("credential-verifier root digest is stale");
        }
        let kind = if reason.is_some() {
            "revocation"
        } else {
            "activation"
        };
        if transition_by_kind_on(&tx, &id, kind)?.is_some() {
            bail!("credential-verifier transition already exists");
        }
        if reason.is_none() {
            if actor == root.record.registration.created_by_admin_user_id {
                bail!("credential-verifier activation requires another administrator");
            }
        } else if transition_by_kind_on(&tx, &id, "activation")?.is_none() {
            bail!("credential-verifier must be active before revocation");
        }

        let timestamp = now();
        let registration = &root.record.registration;
        let transition = ExternalPoolAdapterCredentialVerifierTransition {
            verifier_record_id: id,
            verifier_record_digest: expected_digest,
            verification_kind: registration.verification_kind.clone(),
            verifier_id: registration.verifier_id.clone(),
            verifier_revision: registration.verifier_revision,
            verifier_digest: registration.verifier_digest.clone(),
            verifier_operator: registration.verifier_operator.clone(),
            verifier_product: registration.verifier_product.clone(),
            actor_kind: CREDENTIAL_VERIFIER_ACTOR_KIND.into(),
            actor_user_id: actor,
            reason,
            confirmation,
            idempotency_scope: scope,
            idempotency_key: key,
            occurred_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: if kind == "activation" {
                CREDENTIAL_VERIFIER_STATUS_ACTIVE
            } else {
                CREDENTIAL_VERIFIER_STATUS_REVOKED
            }
            .into(),
            credential_receipt_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            adapter_adoption_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            route_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
            execution_effect: CREDENTIAL_VERIFIER_NO_EFFECT.into(),
        };
        let mut receipt = ExternalPoolAdapterCredentialVerifierTransitionReceipt {
            schema: if kind == "activation" {
                CREDENTIAL_VERIFIER_ACTIVATION_SCHEMA
            } else {
                CREDENTIAL_VERIFIER_REVOCATION_SCHEMA
            }
            .into(),
            transition_receipt_id: new_id("external_pool_adapter_credential_verifier_transition"),
            transition_receipt_digest: String::new(),
            transition_material_digest: credential_verifier_transition_digest(&transition)?,
            canonicalization: CREDENTIAL_VERIFIER_CANONICALIZATION.into(),
            digest_algorithm: CREDENTIAL_VERIFIER_DIGEST_ALGORITHM.into(),
            transition,
        };
        receipt.transition_receipt_digest =
            credential_verifier_transition_json_and_digest(&receipt)?.1;
        validate_credential_verifier_transition(&receipt)?;
        let (json, digest) = credential_verifier_transition_json_and_digest(&receipt)?;
        if digest != receipt.transition_receipt_digest {
            bail!("credential-verifier transition digest drifted");
        }
        insert_transition(&tx, kind, &receipt, &json)?;
        let stored = transition_by_kind_on(&tx, &receipt.transition.verifier_record_id, kind)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier transition disappeared"))?;
        if stored.receipt != receipt || stored.json != json {
            bail!("credential-verifier transition changed during readback");
        }
        let result = ExternalPoolAdapterCredentialVerifierTransitionWriteReceipt {
            verifier_record: root.summary(),
            transition: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(result)
    }
}

fn insert_record(
    tx: &rusqlite::Transaction<'_>,
    record: &ExternalPoolAdapterCredentialVerifierRecord,
    json: &str,
) -> Result<()> {
    let item = &record.registration;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_credential_verifiers(
         verifier_record_id,verifier_record_schema,verifier_record_digest,verifier_record_json,registration_material_digest,
         canonicalization,digest_algorithm,verifier_operator,verifier_product,verification_kind,verifier_id,
         verifier_revision,verifier_digest,actor_kind,created_by_admin_user_id,confirmation,idempotency_scope,
         idempotency_key,created_at,recorded_at,currentness_effect,credential_receipt_effect,
         adapter_adoption_effect,route_effect,execution_effect)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
        params![
            record.verifier_record_id,
            record.schema,
            record.verifier_record_digest,
            json,
            record.registration_material_digest,
            record.canonicalization,
            record.digest_algorithm,
            item.verifier_operator,
            item.verifier_product,
            item.verification_kind,
            item.verifier_id,
            item.verifier_revision,
            item.verifier_digest,
            item.actor_kind,
            item.created_by_admin_user_id,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.created_at,
            item.recorded_at,
            item.currentness_effect,
            item.credential_receipt_effect,
            item.adapter_adoption_effect,
            item.route_effect,
            item.execution_effect,
        ],
    )?;
    Ok(())
}

fn insert_transition(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    receipt: &ExternalPoolAdapterCredentialVerifierTransitionReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.transition;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_credential_verifier_transitions(
         transition_receipt_id,transition_receipt_schema,transition_receipt_digest,transition_receipt_json,
         transition_material_digest,transition_kind,canonicalization,digest_algorithm,verifier_record_id,
         verifier_record_digest,verification_kind,verifier_id,verifier_revision,verifier_digest,verifier_operator,
         verifier_product,actor_kind,actor_user_id,reason,confirmation,idempotency_scope,idempotency_key,
         occurred_at,recorded_at,currentness_effect,credential_receipt_effect,adapter_adoption_effect,
         route_effect,execution_effect)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29)",
        params![
            receipt.transition_receipt_id,
            receipt.schema,
            receipt.transition_receipt_digest,
            json,
            receipt.transition_material_digest,
            kind,
            receipt.canonicalization,
            receipt.digest_algorithm,
            item.verifier_record_id,
            item.verifier_record_digest,
            item.verification_kind,
            item.verifier_id,
            item.verifier_revision,
            item.verifier_digest,
            item.verifier_operator,
            item.verifier_product,
            item.actor_kind,
            item.actor_user_id,
            item.reason,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.occurred_at,
            item.recorded_at,
            item.currentness_effect,
            item.credential_receipt_effect,
            item.adapter_adoption_effect,
            item.route_effect,
            item.execution_effect,
        ],
    )?;
    Ok(())
}

fn ensure_registration_replay(
    stored: &StoredVerifierRecord,
    input: &RegisterExternalPoolAdapterCredentialVerifier,
) -> Result<()> {
    let item = &stored.record.registration;
    if item.verifier_operator != input.verifier_operator
        || item.verifier_product != input.verifier_product
        || item.verification_kind != input.verification_kind
        || item.verifier_id != input.verifier_id
        || item.verifier_revision != input.verifier_revision
        || item.verifier_digest != input.verifier_digest
        || item.created_by_admin_user_id != input.created_by_admin_user_id
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("credential-verifier registration replay conflicts with immutable history");
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
    if item.verifier_record_id != id
        || item.verifier_record_digest != digest
        || item.actor_user_id != actor
        || item.reason.as_deref() != reason
        || item.confirmation != confirmation
        || item.idempotency_scope != scope
        || item.idempotency_key != key
    {
        bail!("credential-verifier transition replay conflicts with immutable history");
    }
    Ok(())
}

fn validate_registration_input(
    input: &RegisterExternalPoolAdapterCredentialVerifier,
) -> Result<()> {
    validate_exact(&input.verifier_operator, 160)?;
    validate_exact(&input.verifier_product, 160)?;
    validate_exact(&input.verification_kind, 80)?;
    validate_exact(&input.verifier_id, 160)?;
    validate_digest(&input.verifier_digest)?;
    validate_exact(&input.created_by_admin_user_id, 160)?;
    validate_exact(&input.idempotency_scope, 200)?;
    validate_exact(&input.idempotency_key, 160)?;
    if input.confirmation != CREDENTIAL_VERIFIER_REGISTER_CONFIRMATION
        || !(1..=MAX_SAFE_INTEGER).contains(&input.verifier_revision)
    {
        bail!("credential-verifier registration input is invalid");
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
            bail!("credential-verifier revocation reason is too short");
        }
    }
    if confirmation != expected_confirmation {
        bail!("credential-verifier transition confirmation is invalid");
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
