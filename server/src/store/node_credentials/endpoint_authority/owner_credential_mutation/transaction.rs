use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::TransactionBehavior;

use crate::node_compute_sharing::endpoint_authority::{
    authorize_password_owner_reauthentication, prepare_owner_reauthentication_consumption,
    NodeEndpointOwnerCredentialMutationRequest, OwnerApiResponsePermit,
    PresentedNodeEndpointCredentialSecret, VerifiedSecureOwnerApiTransport,
};

use super::super::{owner_reauthentication, Store};
use super::{
    authorization, commit_result, current_account, current_target, execute, replay, secret,
    NodeEndpointOwnerCredentialMutationCommit,
};

pub(super) fn mutate(
    store: &Store,
    bearer_token: &str,
    current_password: &str,
    presented_endpoint_secret: Option<&str>,
    request: NodeEndpointOwnerCredentialMutationRequest,
    transport: VerifiedSecureOwnerApiTransport,
    response_permit: OwnerApiResponsePermit,
) -> Result<NodeEndpointOwnerCredentialMutationCommit> {
    response_permit.validate_pair(&transport)?;
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let account_checked_at = Utc::now();
    let account = current_account::verify_current_owner_account_on(
        &transaction,
        bearer_token,
        current_password,
        account_checked_at,
    )?;
    let recorded_at = Utc::now();
    transport.ensure_fresh_at(recorded_at)?;
    let presented = presented_secret(request.authorization_action(), presented_endpoint_secret)?;

    if let Some(replayed) = replay::read_exact_on(
        &transaction,
        &account,
        &request,
        &transport,
        presented.as_ref(),
    )? {
        let committed = replayed.committed().clone();
        let digest = replayed.consumption_digest().to_string();
        let result_is_current = replayed.result_is_current();
        let consumption = replayed.consumption().clone();
        response_permit.ensure_fresh_at(Utc::now())?;
        transaction.commit()?;
        return Ok(commit_result(
            committed,
            consumption,
            digest,
            true,
            result_is_current,
            None,
            response_permit,
        ));
    }

    let expected_current =
        current_target::require_current_target_on(&transaction, &account, &request)?;
    let owner_authorization = authorize_password_owner_reauthentication(
        account,
        transport,
        &request,
        expected_current.clone(),
        recorded_at,
    )?;
    let prepared_owner = owner_authorization.prepare(recorded_at)?;
    let owner_receipt =
        owner_reauthentication::record_at_on(&transaction, &owner_authorization, recorded_at)?;
    if owner_receipt.replayed()
        || owner_receipt.envelope() != prepared_owner.envelope()
        || owner_receipt.receipt_digest() != prepared_owner.receipt_digest()
    {
        bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_FRESH_READBACK_MISMATCH");
    }

    let generated_secret = if request.returns_secret() {
        Some(secret::generate_endpoint_secret()?)
    } else {
        None
    };
    let prepared_mutation = authorization::prepare_on(
        &transaction,
        &prepared_owner,
        &request,
        expected_current,
        generated_secret
            .as_ref()
            .map(secret::GeneratedEndpointSecret::secret_hash),
        recorded_at,
    )?;
    let (authorized_mutation, result_binding) = prepared_mutation.into_parts();
    let prepared_consumption = prepare_owner_reauthentication_consumption(
        &prepared_owner,
        result_binding,
        recorded_at,
        recorded_at,
    )?;
    owner_reauthentication::consumption_rows::insert_on(&transaction, &prepared_consumption)?;

    let credential_receipt = execute::persist_at_on(
        &transaction,
        &authorized_mutation,
        presented.as_ref(),
        recorded_at,
    )?;
    require_result_matches(&credential_receipt, prepared_consumption.envelope())?;
    let stored = owner_reauthentication::consumption_rows::by_consumption_id_on(
        &transaction,
        prepared_consumption.envelope().consumption_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_MISSING"))?;
    stored.ensure_exact(&prepared_consumption)?;
    let committed = credential_receipt.current().clone();
    let consumption_digest = stored.consumption_digest().to_string();
    let consumption = stored.into_envelope();
    response_permit.ensure_fresh_at(Utc::now())?;
    transaction.commit()?;
    Ok(commit_result(
        committed,
        consumption,
        consumption_digest,
        false,
        true,
        generated_secret,
        response_permit,
    ))
}

fn presented_secret(
    action: &str,
    plaintext: Option<&str>,
) -> Result<Option<PresentedNodeEndpointCredentialSecret>> {
    match (action, plaintext) {
        ("credential_rotation", Some(value)) if !value.trim().is_empty() => Ok(Some(
            PresentedNodeEndpointCredentialSecret::from_secret_hash(secret::presented_secret_hash(
                value,
            )),
        )),
        ("credential_rotation", _) => bail!("NODE_ENDPOINT_CREDENTIAL_POSSESSION_REQUIRED"),
        (_, None) => Ok(None),
        _ => bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_POSSESSION_SHAPE_INVALID"),
    }
}

fn require_result_matches(
    receipt: &super::super::NodeEndpointCredentialMutationReceipt,
    expected: &crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationConsumptionEnvelope,
) -> Result<()> {
    if receipt.replayed()
        || receipt.current().credential_id() != expected.current_credential_id()
        || receipt.current().credential_revision() != expected.current_credential_revision()
        || receipt.current().credential_digest() != expected.current_credential_digest()
        || receipt.current().status() != expected.current_credential_status()
        || receipt.issued_version().map(|value| value.credential_id())
            != expected.issued_credential_id()
        || receipt
            .issued_version()
            .map(|value| value.credential_revision())
            != expected.issued_credential_revision()
        || receipt.revoked_version().map(|value| value.revocation_id()) != expected.revocation_id()
    {
        bail!("NODE_ENDPOINT_OWNER_CREDENTIAL_MUTATION_READBACK_MISMATCH");
    }
    Ok(())
}
