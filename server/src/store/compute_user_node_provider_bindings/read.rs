use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::compute_federation::user_node_provider_binding::{
    user_node_provider_binding_receipt_from_json, UserNodeProviderBindingReceiptV1,
};

use super::{
    current_user_node_provider_binding_on, inspection, Store, UserNodeProviderBindingInspection,
};

const BINDING_COLUMNS: &str = "binding_id, binding_schema, binding_digest, binding_json,
    binding_material_digest, canonicalization, digest_algorithm, provider_id,
    provider_genesis_policy_revision,
    provider_genesis_digest, node_id, owner_user_id, installation_identity_digest,
    endpoint_installation_binding_digest, source_endpoint_credential_id,
    source_endpoint_credential_revision, source_endpoint_credential_digest,
    source_consent_receipt_id, source_consent_policy_revision, source_consent_policy_digest,
    source_authorization_ref, source_authorization_revision, source_authorization_digest,
    confirmation, idempotency_scope, idempotency_key, request_digest, bound_at, recorded_at,
    binding_effect, provider_effect, capacity_effect, offer_effect, readiness_effect, route_effect,
    execution_effect, settlement_effect";

pub(super) fn inspect_for_owner(
    store: &Store,
    owner_user_id: &str,
    provider_id: &str,
) -> Result<Option<UserNodeProviderBindingInspection>> {
    exact_identifier("绑定所有者", owner_user_id, 160)?;
    exact_identifier("Provider ID", provider_id, 160)?;
    let mut connection = store.conn()?;
    let transaction = connection.transaction()?;
    let Some(receipt) = binding_by_provider_on(&transaction, provider_id)? else {
        transaction.commit()?;
        return Ok(None);
    };
    if receipt.binding().owner_user_id() != owner_user_id {
        bail!("节点 Provider 绑定不属于当前登录用户");
    }
    let current = current_user_node_provider_binding_on(
        &transaction,
        provider_id,
        receipt.binding_id(),
        owner_user_id,
    )?
    .is_some();
    transaction.commit()?;
    Ok(Some(inspection(receipt, current)))
}

pub(super) fn binding_by_provider_on(
    connection: &Connection,
    provider_id: &str,
) -> Result<Option<UserNodeProviderBindingReceiptV1>> {
    query_one(
        connection,
        &format!(
            "SELECT {BINDING_COLUMNS} FROM compute_user_node_provider_bindings
              WHERE provider_id=?1"
        ),
        params![provider_id],
    )
}

pub(super) fn binding_by_idempotency_on(
    connection: &Connection,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<Option<UserNodeProviderBindingReceiptV1>> {
    query_one(
        connection,
        &format!(
            "SELECT {BINDING_COLUMNS} FROM compute_user_node_provider_bindings
              WHERE idempotency_scope=?1 AND idempotency_key=?2"
        ),
        params![idempotency_scope, idempotency_key],
    )
}

pub(super) fn binding_by_node_on(
    connection: &Connection,
    node_id: &str,
) -> Result<Option<UserNodeProviderBindingReceiptV1>> {
    query_one(
        connection,
        &format!(
            "SELECT {BINDING_COLUMNS} FROM compute_user_node_provider_bindings WHERE node_id=?1"
        ),
        params![node_id],
    )
}

fn query_one<P: rusqlite::Params>(
    connection: &Connection,
    sql: &str,
    parameters: P,
) -> Result<Option<UserNodeProviderBindingReceiptV1>> {
    connection
        .query_row(sql, parameters, StoredBindingRow::from_row)
        .optional()?
        .map(StoredBindingRow::into_receipt)
        .transpose()
}

struct StoredBindingRow {
    values: Vec<rusqlite::types::Value>,
}

impl StoredBindingRow {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let mut values = Vec::with_capacity(37);
        for index in 0..37 {
            values.push(row.get(index)?);
        }
        Ok(Self { values })
    }

    fn text(&self, index: usize) -> Result<&str> {
        match &self.values[index] {
            rusqlite::types::Value::Text(value) => Ok(value),
            _ => bail!("节点 Provider 绑定列 {index} 类型错误"),
        }
    }

    fn integer(&self, index: usize) -> Result<i64> {
        match &self.values[index] {
            rusqlite::types::Value::Integer(value) => Ok(*value),
            _ => bail!("节点 Provider 绑定列 {index} 类型错误"),
        }
    }

    fn into_receipt(self) -> Result<UserNodeProviderBindingReceiptV1> {
        let binding_json = self.text(3)?.to_string();
        let receipt = user_node_provider_binding_receipt_from_json(&binding_json)?;
        if receipt.binding_json()? != binding_json || !self.matches(&receipt)? {
            bail!("节点 Provider 绑定标量投影或 canonical readback 不一致");
        }
        Ok(receipt)
    }

    fn matches(&self, receipt: &UserNodeProviderBindingReceiptV1) -> Result<bool> {
        let binding = receipt.binding();
        Ok(self.text(0)? == receipt.binding_id()
            && self.text(1)? == receipt.schema()
            && self.text(2)? == receipt.binding_digest()
            && self.text(4)? == receipt.binding_material_digest()
            && self.text(5)? == receipt.canonicalization()
            && self.text(6)? == receipt.digest_algorithm()
            && self.text(7)? == binding.provider_id()
            && self.integer(8)? == binding.provider_genesis_policy_revision()
            && self.text(9)? == binding.provider_genesis_digest()
            && self.text(10)? == binding.node_id()
            && self.text(11)? == binding.owner_user_id()
            && self.text(12)? == binding.installation_identity_digest()
            && self.text(13)? == binding.endpoint_installation_binding_digest()
            && self.text(14)? == binding.source_endpoint_credential_id()
            && self.integer(15)? == binding.source_endpoint_credential_revision()
            && self.text(16)? == binding.source_endpoint_credential_digest()
            && self.text(17)? == binding.source_consent_receipt_id()
            && self.integer(18)? == binding.source_consent_policy_revision()
            && self.text(19)? == binding.source_consent_policy_digest()
            && self.text(20)? == binding.source_authorization_ref()
            && self.integer(21)? == binding.source_authorization_revision()
            && self.text(22)? == binding.source_authorization_digest()
            && self.text(23)? == binding.confirmation()
            && self.text(24)? == binding.idempotency_scope()
            && self.text(25)? == binding.idempotency_key()
            && self.text(26)? == binding.request_digest()
            && self.text(27)? == binding.bound_at()
            && self.text(28)? == binding.recorded_at()
            && self.text(29)? == binding.binding_effect()
            && self.text(30)? == binding.provider_effect()
            && self.text(31)? == binding.capacity_effect()
            && self.text(32)? == binding.offer_effect()
            && self.text(33)? == binding.readiness_effect()
            && self.text(34)? == binding.route_effect()
            && self.text(35)? == binding.execution_effect()
            && self.text(36)? == binding.settlement_effect())
    }
}

fn exact_identifier(label: &str, value: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.trim() != value || value.chars().count() > max {
        bail!("{label}不能为空、包含首尾空白或超过 {max} 字符");
    }
    Ok(())
}
