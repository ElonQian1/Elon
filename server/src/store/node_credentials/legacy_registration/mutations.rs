use anyhow::Result;
use rusqlite::{params, OptionalExtension, Transaction};

use super::normalize::NormalizedRegistrationRequest;
use crate::store::node_credentials::{merge_legacy_device_duplicates, select_legacy_credential};
use crate::store::now;

pub(super) fn renew_by_install_id_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<Option<String>> {
    let Some(install_id) = request.install_id else {
        return Ok(None);
    };
    let agent_id = transaction
        .query_row(
            "SELECT agent_id
               FROM node_credentials
              WHERE owner_user_id=?1 AND install_id=?2
              ORDER BY created_at DESC
              LIMIT 1",
            params![request.owner_user_id, install_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(agent_id) = agent_id else {
        return Ok(None);
    };
    transaction.execute(
        "UPDATE node_credentials
            SET secret_hash=?3,
                label=COALESCE(NULLIF(label, ''), ?4, ''),
                device_name=COALESCE(?5, device_name)
          WHERE agent_id=?1 AND owner_user_id=?2",
        params![
            agent_id,
            request.owner_user_id,
            request.new_secret_hash,
            request.label,
            request.device_name
        ],
    )?;
    Ok(Some(agent_id))
}

pub(super) fn renew_by_existing_secret_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<Option<String>> {
    let (Some(agent_id), Some(existing_secret_hash)) =
        (request.existing_agent_id, request.existing_secret_hash)
    else {
        return Ok(None);
    };
    let updated = transaction.execute(
        "UPDATE node_credentials
            SET secret_hash=?4,
                install_id=COALESCE(NULLIF(install_id, ''), ?5),
                device_name=COALESCE(?6, device_name)
          WHERE agent_id=?1 AND secret_hash=?2 AND owner_user_id=?3",
        params![
            agent_id,
            existing_secret_hash,
            request.owner_user_id,
            request.new_secret_hash,
            request.install_id,
            request.device_name
        ],
    )?;
    Ok((updated == 1).then(|| agent_id.to_string()))
}

pub(super) fn renew_by_legacy_device_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<Option<String>> {
    let (Some(install_id), Some(device_name)) = (request.install_id, request.device_name) else {
        return Ok(None);
    };
    let Some(agent_id) = select_legacy_credential(transaction, request.owner_user_id, device_name)?
    else {
        return Ok(None);
    };
    transaction.execute(
        "UPDATE node_credentials
            SET secret_hash=?3,
                install_id=?4,
                label=COALESCE(NULLIF(label, ''), ?5, ''),
                device_name=COALESCE(?6, device_name)
          WHERE agent_id=?1 AND owner_user_id=?2",
        params![
            agent_id,
            request.owner_user_id,
            request.new_secret_hash,
            install_id,
            request.label,
            device_name
        ],
    )?;
    merge_legacy_device_duplicates(
        transaction,
        request.owner_user_id,
        device_name,
        &agent_id,
        &now(),
    )?;
    Ok(Some(agent_id))
}

pub(super) fn create_legacy_credential_on(
    transaction: &Transaction<'_>,
    request: &NormalizedRegistrationRequest<'_>,
) -> Result<()> {
    transaction.execute(
        "INSERT INTO node_credentials
            (agent_id, secret_hash, owner_user_id, label, device_name, install_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            request.proposed_agent_id,
            request.new_secret_hash,
            request.owner_user_id,
            request.label.unwrap_or(""),
            request.device_name,
            request.install_id,
            now()
        ],
    )?;
    Ok(())
}
