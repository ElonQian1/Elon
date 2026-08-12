use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, TransactionBehavior};

use super::{read::*, types::*};
use crate::{
    compute_federation::external_pool_adapter_credential_verifier_key::*,
    store::{
        compute_external_pool_adapter_credential_verifier::current_credential_verifier_authority_on,
        new_id, Store,
    },
};

impl Store {
    pub(crate) fn register_external_pool_adapter_credential_verifier_key(
        &self,
        input: RegisterCredentialVerifierKey,
    ) -> Result<CredentialVerifierKeyRegistrationWriteReceipt> {
        validate_register(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) =
            record_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let item = &stored.record.registration;
            if item.verifier_record_id != input.verifier_record_id
                || item.verifier_record_digest != input.expected_verifier_record_digest
                || item.verification_kind != input.verification_kind
                || item.verifier_id != input.verifier_id
                || item.verifier_revision != input.verifier_revision
                || item.verifier_digest != input.expected_verifier_digest
                || item.key_id != input.key_id
                || item.public_key_pem != input.public_key_pem
                || item.created_by_admin_user_id != input.created_by_admin_user_id
            {
                bail!("credential-verifier-key replay conflicts")
            }
            let output = CredentialVerifierKeyRegistrationWriteReceipt {
                key_record: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        let verifier = current_credential_verifier_authority_on(
            &tx,
            &input.verifier_record_id,
            &input.expected_verifier_record_digest,
            &input.verification_kind,
            &input.verifier_id,
            input.verifier_revision,
            &input.expected_verifier_digest,
        )?
        .ok_or_else(|| anyhow::anyhow!("active credential verifier was not found"))?;
        if verifier.created_by_admin_user_id() == input.created_by_admin_user_id {
            bail!("credential-verifier-key registration requires another administrator")
        }
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let registration = CredentialVerifierKeyRegistration {
            verifier_record_id: verifier.verifier_record_id().into(),
            verifier_record_digest: verifier.verifier_record_digest().into(),
            verifier_operator: verifier.verifier_operator().into(),
            verifier_product: verifier.verifier_product().into(),
            verification_kind: verifier.verification_kind().into(),
            verifier_id: verifier.verifier_id().into(),
            verifier_revision: verifier.verifier_revision(),
            verifier_digest: verifier.verifier_digest().into(),
            key_id: input.key_id,
            algorithm: KEY_ALGORITHM.into(),
            public_key_pem: input.public_key_pem,
            actor_kind: ACTOR_KIND.into(),
            created_by_admin_user_id: input.created_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            created_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: STATUS_ACTIVE.into(),
            credential_receipt_effect: NO_EFFECT.into(),
            adapter_effect: NO_EFFECT.into(),
            route_effect: NO_EFFECT.into(),
        };
        let mut record = CredentialVerifierKeyRecord {
            schema: RECORD_SCHEMA.into(),
            key_record_id: new_id("external_pool_adapter_credential_verifier_key"),
            key_record_digest: String::new(),
            registration_material_digest: registration_digest(&registration)?,
            canonicalization: CANONICALIZATION.into(),
            digest_algorithm: DIGEST_ALGORITHM.into(),
            registration,
        };
        record.key_record_digest = record_json_and_digest(&record)?.1;
        validate_record(&record)?;
        let (json, digest) = record_json_and_digest(&record)?;
        if digest != record.key_record_digest {
            bail!("credential-verifier-key digest drifted")
        }
        insert_root(&tx, &record, &json)?;
        let stored = record_by_id_on(&tx, &record.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier-key disappeared"))?;
        if stored.record != record || stored.json != json {
            bail!("credential-verifier-key changed during readback")
        }
        let output = CredentialVerifierKeyRegistrationWriteReceipt {
            key_record: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }

    pub(crate) fn revoke_external_pool_adapter_credential_verifier_key(
        &self,
        input: RevokeCredentialVerifierKey,
    ) -> Result<CredentialVerifierKeyRevocationWriteReceipt> {
        validate_revoke(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let root = record_by_id_on(&tx, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier-key not found"))?;
        if let Some(stored) =
            revocation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let item = &stored.receipt.revocation;
            if item.key_record_id != input.key_record_id
                || item.key_record_digest != input.expected_key_record_digest
                || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
                || item.reason != input.reason
            {
                bail!("credential-verifier-key revocation replay conflicts")
            }
            let output = CredentialVerifierKeyRevocationWriteReceipt {
                key_record: root.summary(),
                revocation: stored.summary(),
                replayed: true,
            };
            tx.commit()?;
            return Ok(output);
        }
        if root.record.key_record_digest != input.expected_key_record_digest {
            bail!("credential-verifier-key digest is stale")
        }
        if revocation_by_key_on(&tx, &input.key_record_id)?.is_some() {
            bail!("credential-verifier-key is already revoked")
        }
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let registration = &root.record.registration;
        let material = CredentialVerifierKeyRevocation {
            key_record_id: root.record.key_record_id.clone(),
            key_record_digest: root.record.key_record_digest.clone(),
            verifier_record_id: registration.verifier_record_id.clone(),
            verifier_record_digest: registration.verifier_record_digest.clone(),
            key_id: registration.key_id.clone(),
            actor_kind: ACTOR_KIND.into(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            revoked_at: timestamp.clone(),
            recorded_at: timestamp,
            currentness_effect: STATUS_REVOKED.into(),
            credential_receipt_effect: NO_EFFECT.into(),
            adapter_effect: NO_EFFECT.into(),
            route_effect: NO_EFFECT.into(),
        };
        let mut receipt = CredentialVerifierKeyRevocationReceipt {
            schema: REVOCATION_SCHEMA.into(),
            revocation_receipt_id: new_id(
                "external_pool_adapter_credential_verifier_key_revocation",
            ),
            revocation_receipt_digest: String::new(),
            revocation_material_digest: revocation_digest(&material)?,
            canonicalization: CANONICALIZATION.into(),
            digest_algorithm: DIGEST_ALGORITHM.into(),
            revocation: material,
        };
        receipt.revocation_receipt_digest = revocation_json_and_digest(&receipt)?.1;
        validate_revocation(&receipt)?;
        let (json, digest) = revocation_json_and_digest(&receipt)?;
        if digest != receipt.revocation_receipt_digest {
            bail!("credential-verifier-key revocation digest drifted")
        }
        insert_revocation(&tx, &receipt, &json)?;
        let stored = revocation_by_key_on(&tx, &input.key_record_id)?
            .ok_or_else(|| anyhow::anyhow!("credential-verifier-key revocation disappeared"))?;
        if stored.receipt != receipt || stored.json != json {
            bail!("credential-verifier-key revocation changed during readback")
        }
        let output = CredentialVerifierKeyRevocationWriteReceipt {
            key_record: root.summary(),
            revocation: stored.summary(),
            replayed: false,
        };
        tx.commit()?;
        Ok(output)
    }
}

fn insert_root(
    tx: &rusqlite::Transaction<'_>,
    record: &CredentialVerifierKeyRecord,
    json: &str,
) -> Result<()> {
    let i = &record.registration;
    tx.execute(
 "INSERT INTO compute_external_pool_adapter_credential_verifier_keys(key_record_id,key_record_schema,key_record_digest,key_record_json,registration_material_digest,canonicalization,digest_algorithm,
 verifier_record_id,verifier_record_digest,verifier_operator,verifier_product,verification_kind,verifier_id,verifier_revision,verifier_digest,key_id,algorithm,public_key_pem,actor_kind,
 created_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,created_at,recorded_at,currentness_effect,credential_receipt_effect,adapter_effect,route_effect)
 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29)",params![record.key_record_id,record.schema,record.key_record_digest,json,record.registration_material_digest,record.canonicalization,record.digest_algorithm,
 i.verifier_record_id,i.verifier_record_digest,i.verifier_operator,i.verifier_product,i.verification_kind,i.verifier_id,i.verifier_revision,i.verifier_digest,i.key_id,i.algorithm,i.public_key_pem,i.actor_kind,
 i.created_by_admin_user_id,i.confirmation,i.idempotency_scope,i.idempotency_key,i.created_at,i.recorded_at,i.currentness_effect,i.credential_receipt_effect,i.adapter_effect,i.route_effect])?;
    Ok(())
}

fn insert_revocation(
    tx: &rusqlite::Transaction<'_>,
    receipt: &CredentialVerifierKeyRevocationReceipt,
    json: &str,
) -> Result<()> {
    let i = &receipt.revocation;
    tx.execute(
 "INSERT INTO compute_external_pool_adapter_credential_verifier_key_revocations(revocation_receipt_id,revocation_receipt_schema,revocation_receipt_digest,revocation_receipt_json,revocation_material_digest,canonicalization,digest_algorithm,
 key_record_id,key_record_digest,verifier_record_id,verifier_record_digest,key_id,actor_kind,revoked_by_admin_user_id,reason,confirmation,idempotency_scope,idempotency_key,revoked_at,recorded_at,currentness_effect,credential_receipt_effect,adapter_effect,route_effect)
 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)",params![receipt.revocation_receipt_id,receipt.schema,receipt.revocation_receipt_digest,json,receipt.revocation_material_digest,receipt.canonicalization,receipt.digest_algorithm,
 i.key_record_id,i.key_record_digest,i.verifier_record_id,i.verifier_record_digest,i.key_id,i.actor_kind,i.revoked_by_admin_user_id,i.reason,i.confirmation,i.idempotency_scope,i.idempotency_key,i.revoked_at,i.recorded_at,i.currentness_effect,i.credential_receipt_effect,i.adapter_effect,i.route_effect])?;
    Ok(())
}

fn validate_register(i: &RegisterCredentialVerifierKey) -> Result<()> {
    for x in [
        &i.verifier_record_id,
        &i.verification_kind,
        &i.verifier_id,
        &i.created_by_admin_user_id,
        &i.idempotency_scope,
        &i.idempotency_key,
    ] {
        validate_exact(x, 200)?
    }
    for x in [
        &i.expected_verifier_record_digest,
        &i.expected_verifier_digest,
        &i.key_id,
    ] {
        validate_digest(x)?
    }
    if i.verifier_revision < 1
        || i.confirmation != REGISTER_CONFIRMATION
        || i.public_key_pem.is_empty()
        || i.public_key_pem.len() > 16384
    {
        bail!("credential-verifier-key registration input is invalid")
    }
    Ok(())
}
fn validate_revoke(i: &RevokeCredentialVerifierKey) -> Result<()> {
    for x in [
        &i.key_record_id,
        &i.revoked_by_admin_user_id,
        &i.idempotency_scope,
        &i.idempotency_key,
    ] {
        validate_exact(x, 200)?
    }
    validate_digest(&i.expected_key_record_digest)?;
    if i.reason.trim() != i.reason
        || i.reason.chars().count() < 8
        || i.reason.chars().count() > 2000
        || i.confirmation != REVOKE_CONFIRMATION
    {
        bail!("credential-verifier-key revocation input is invalid")
    }
    Ok(())
}
