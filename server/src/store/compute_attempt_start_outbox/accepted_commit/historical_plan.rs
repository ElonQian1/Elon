//! Ordered one-shot admission for historical Accepted closure rows.

use anyhow::Result;
use rusqlite::{types::Value, Connection};

use crate::store::compute_attempt_dispatches::PreparedApplication;
use crate::store::compute_external_pool_adapter_task_delivery::{
    install_external_pool_adapter_task_reachability_pending_plan_on,
    ExternalPoolAdapterTaskReachabilityPendingPlan,
    ExternalPoolAdapterTaskReachabilityPendingPlanGuard,
    ExternalPoolAdapterTaskReachabilityPendingWrite,
    ExternalPoolAdapterTaskReachabilityPendingWriteKind,
};

use super::DerivedAcceptedCommitClosure;

pub(super) fn install_on(
    connection: &Connection,
    expected: &DerivedAcceptedCommitClosure,
    application: &PreparedApplication,
    created_at: &str,
) -> Result<ExternalPoolAdapterTaskReachabilityPendingPlanGuard> {
    let actor = &expected.actor;
    let authority = &expected.authority;
    let commit = &expected.commit;
    let application = application.envelope();
    let plan = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        pending(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::HistoricalAcceptedActor,
            vec![
                text(&actor.actor_receipt_id),
                text(&actor.actor_receipt_digest),
                text(&expected.actor_json),
                text(&actor.command_id),
                optional_text(actor.ack_id.as_deref()),
                optional_text(actor.application_id.as_deref()),
                text(&actor.recorded_at),
            ],
        )?,
        pending(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::HistoricalAcceptedLeaseAuthority,
            vec![
                text(&authority.lease_authority_id),
                Value::Integer(authority.authority_revision),
                text(&authority.lease_authority_digest),
                text(&expected.authority_json),
                text(&authority.command_id),
                text(&authority.application_id),
                text(&authority.recorded_at),
            ],
        )?,
        pending(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::HistoricalAcceptedCommit,
            vec![
                text(&commit.outbox_id),
                text(&commit.outbox_digest),
                text(&expected.commit_json),
                text(&commit.command_id),
                optional_text(commit.application_id.as_deref()),
                text(created_at),
            ],
        )?,
        pending(
            ExternalPoolAdapterTaskReachabilityPendingWriteKind::HistoricalAcceptedApplication,
            vec![
                text(&application.application_id),
                text(&application.application_digest),
                text(&application.command_id),
                text(&application.ack_id),
                text(&application.applied_at),
                text(created_at),
            ],
        )?,
    ])?;
    install_external_pool_adapter_task_reachability_pending_plan_on(connection, plan)
}

fn pending(
    kind: ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    values: Vec<Value>,
) -> Result<ExternalPoolAdapterTaskReachabilityPendingWrite> {
    ExternalPoolAdapterTaskReachabilityPendingWrite::new(kind, values)
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

fn optional_text(value: Option<&str>) -> Value {
    value.map(text).unwrap_or(Value::Null)
}
