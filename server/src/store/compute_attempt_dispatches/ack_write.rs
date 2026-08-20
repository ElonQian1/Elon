use anyhow::{anyhow, bail, ensure, Result};
use rusqlite::Connection;

use crate::{
    compute_federation::attempt_gateway::{
        VerifiedComputeAttemptAdapterAckView, COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED,
        COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED,
    },
    store::{
        compute_attempt_activations::{
            activate_compute_attempt_at_on, activate_compute_attempt_on,
        },
        compute_attempt_start_outbox::{
            audit_accepted_start_commit_closure_on,
            audit_historical_accepted_start_commit_closure_on,
            ensure_fresh_accepted_start_commit_on, ensure_historical_accepted_start_commit_on,
            has_cleanup_pair_on, persist_accepted_start_commit_closure_on,
            persist_historical_accepted_start_commit_closure_on,
            record_prepare_rejected_no_start_at_on, record_prepare_rejected_no_start_on,
            record_verified_observation_at_on, record_verified_observation_on,
            AcceptedStartCommitFreshness,
        },
        compute_external_pool_adapter_task_delivery::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    },
};

mod mutation;
mod times;

use mutation::{
    activation_request, application_created_at, ensure_ack_binds_command, insert_ack_on,
    now_dispatch, quarantine_ack_on,
};
pub(in crate::store) use times::ComputeAttemptAdapterAckIngressTimes;

use super::{
    read::{
        ack_by_adapter_ack_on, ack_by_command_on, ack_receipt, application_by_command_on,
        command_by_id_on, ensure_ack_replay_matches, ensure_application_matches,
        ensure_remote_ack_replay_matches,
    },
    replay::{ensure_activation_matches_command, replay_ack_commit},
    source::{ack_received_after_deadline, current_budget_blocker_on, current_source_blocker_on},
    types::ComputeAttemptDispatchAckCommit,
    validation::{prepare_application, PreparedVerifiedAck},
};

pub(super) fn ingest_verified_ack_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    prepared: &PreparedVerifiedAck,
) -> Result<ComputeAttemptDispatchAckCommit> {
    ingest_verified_ack_with_times_on(
        connection,
        verified,
        prepared,
        None,
        AcceptedAckIngressMode::Fresh,
    )
}

pub(in crate::store) fn ingest_verified_ack_at_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    times: &ComputeAttemptAdapterAckIngressTimes,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let prepared = super::validation::prepare_verified_ack(verified)?;
    ingest_verified_ack_with_times_on(
        connection,
        verified,
        &prepared,
        Some(times),
        AcceptedAckIngressMode::Fresh,
    )
}

pub(in crate::store) fn ingest_verified_historical_external_pool_adapter_ack_at_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    times: &ComputeAttemptAdapterAckIngressTimes,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let prepared = super::validation::prepare_verified_ack(verified)?;
    ingest_verified_ack_with_times_on(
        connection,
        verified,
        &prepared,
        Some(times),
        AcceptedAckIngressMode::HistoricalTerminal(authority),
    )
}

#[derive(Clone, Copy)]
enum AcceptedAckIngressMode<'a, 'tx, 'conn> {
    Fresh,
    HistoricalTerminal(&'a HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>),
}

impl<'a, 'tx, 'conn> AcceptedAckIngressMode<'a, 'tx, 'conn> {
    fn historical_authority(
        self,
    ) -> Option<&'a HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>> {
        match self {
            Self::Fresh => None,
            Self::HistoricalTerminal(authority) => Some(authority),
        }
    }
}

fn ingest_verified_ack_with_times_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    prepared: &PreparedVerifiedAck,
    times: Option<&ComputeAttemptAdapterAckIngressTimes>,
    mode: AcceptedAckIngressMode<'_, '_, '_>,
) -> Result<ComputeAttemptDispatchAckCommit> {
    let ack = verified.ack();
    if let Some(stored) = ack_by_adapter_ack_on(
        connection,
        &verified.adapter().provider_id,
        &verified.adapter().adapter_id,
        &ack.adapter_ack_id,
    )? {
        ensure_remote_ack_replay_matches(&stored, verified, prepared)?;
        let command = command_by_id_on(connection, &stored.ack.command_id)?
            .ok_or_else(|| anyhow!("Stored Adapter ACK lost its immutable command"))?;
        ensure_ack_binds_command(&command, verified, prepared)?;
        record_observation_on(connection, verified, times)?;
        if ack.outcome == COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED {
            record_rejected_no_start_on(connection, verified, times)?;
        }
        return replay_ack_commit(connection, &command, stored, mode.historical_authority());
    }
    if ack_by_command_on(connection, &ack.command_id)?.is_some() {
        bail!("Attempt command already has a different immutable Adapter ACK");
    }
    let command = command_by_id_on(connection, &ack.command_id)?
        .ok_or_else(|| anyhow!("Adapter ACK references an unknown Attempt command"))?;
    ensure_ack_binds_command(&command, verified, prepared)?;
    let ingested_at = times
        .map(ComputeAttemptAdapterAckIngressTimes::ingested_at)
        .map(str::to_string)
        .unwrap_or_else(now_dispatch);
    if chrono::DateTime::parse_from_rfc3339(&ack.received_at)?
        > chrono::DateTime::parse_from_rfc3339(&ingested_at)?
    {
        bail!("Adapter ACK received_at cannot be later than durable ingestion");
    }
    record_observation_on(connection, verified, times)?;
    match ack.outcome.as_str() {
        COMPUTE_ATTEMPT_ADAPTER_ACK_REJECTED => {
            insert_ack_on(
                connection,
                &command,
                verified,
                prepared,
                "rejected",
                None,
                None,
                &ingested_at,
            )?;
            record_rejected_no_start_on(connection, verified, times)?;
            let stored = ack_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Rejected Adapter ACK is not visible after insert"))?;
            ensure_ack_replay_matches(&stored, verified, prepared)?;
            Ok(ComputeAttemptDispatchAckCommit::Rejected {
                ack: ack_receipt(stored, false),
            })
        }
        COMPUTE_ATTEMPT_ADAPTER_ACK_ACCEPTED => {
            let remote_execution_ref = ack
                .remote_execution_ref
                .as_deref()
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its execution ref"))?;
            if has_cleanup_pair_on(connection, &ack.command_id)? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    "CLEANUP_ALREADY_ISSUED",
                    &ingested_at,
                );
            }
            if ack_received_after_deadline(ack, &command.command.not_after, &ingested_at)? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    "COMMAND_EXPIRED",
                    &ingested_at,
                );
            }
            if let Some(reason) = current_source_blocker_on(
                connection,
                &command.command,
                &command.adapter,
                &command.activated_by_user_id,
                &command.activation_idempotency_key,
                true,
            )? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    reason,
                    &ingested_at,
                );
            }
            if let Some(reason) = current_budget_blocker_on(connection, &command, &ingested_at)? {
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    reason,
                    &ingested_at,
                );
            }
            match ensure_accepted_currentness_on(connection, &ack.command_id, &ingested_at, mode)? {
                AcceptedStartCommitFreshness::Current => {}
                AcceptedStartCommitFreshness::Quarantine { reason_code } => {
                    return quarantine_ack_on(
                        connection,
                        &command,
                        verified,
                        prepared,
                        reason_code,
                        &ingested_at,
                    );
                }
            }
            connection.execute_batch("SAVEPOINT accepted_apply_v215")?;
            insert_ack_on(
                connection,
                &command,
                verified,
                prepared,
                "accepted_applied",
                None,
                Some(&command.command.command.identity.attempt_lease_id),
                &ingested_at,
            )?;
            let activation_request = activation_request(&command, remote_execution_ref);
            let activation = match times {
                Some(times) => activate_compute_attempt_at_on(
                    connection,
                    &activation_request,
                    times.activated_at(),
                )?,
                None => activate_compute_attempt_on(connection, &activation_request)?,
            };
            ensure_activation_matches_command(&command, ack, &activation)?;
            let prepared_application = prepare_application(ack, &activation)?;
            let closure_at = match times {
                Some(times) => times.closure_at().to_string(),
                None => application_created_at(&activation.activated_at)?,
            };
            if let AcceptedStartCommitFreshness::Quarantine { reason_code } =
                ensure_accepted_currentness_on(connection, &ack.command_id, &closure_at, mode)?
            {
                connection.execute_batch(
                    "ROLLBACK TO accepted_apply_v215; RELEASE accepted_apply_v215",
                )?;
                return quarantine_ack_on(
                    connection,
                    &command,
                    verified,
                    prepared,
                    reason_code,
                    &closure_at,
                );
            }
            let accepted_closure = match mode {
                AcceptedAckIngressMode::Fresh => persist_accepted_start_commit_closure_on(
                    connection,
                    &ack.command_id,
                    &prepared_application,
                    &closure_at,
                )?,
                AcceptedAckIngressMode::HistoricalTerminal(authority) => {
                    persist_historical_accepted_start_commit_closure_on(
                        connection,
                        &ack.command_id,
                        &prepared_application,
                        &closure_at,
                        authority,
                    )?
                }
            };
            let stored_ack = ack_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is not visible after activation"))?;
            ensure_ack_replay_matches(&stored_ack, verified, prepared)?;
            if stored_ack.activation_lease_id.as_deref()
                != Some(command.command.command.identity.attempt_lease_id.as_str())
                || stored_ack.application_id.as_deref()
                    != Some(prepared_application.application_id.as_str())
            {
                bail!("Accepted Adapter ACK does not bind the activated lease");
            }
            let stored_application = application_by_command_on(connection, &ack.command_id)?
                .ok_or_else(|| anyhow!("Accepted Adapter ACK is missing its application"))?;
            ensure_application_matches(&stored_application, ack, &activation)?;
            let mut audited_closure = match mode {
                AcceptedAckIngressMode::Fresh => {
                    audit_accepted_start_commit_closure_on(connection, &ack.command_id)?
                }
                AcceptedAckIngressMode::HistoricalTerminal(authority) => {
                    audit_historical_accepted_start_commit_closure_on(
                        connection,
                        &ack.command_id,
                        authority,
                    )?
                }
            };
            audited_closure.replayed = false;
            ensure!(
                audited_closure == accepted_closure,
                "Accepted Adapter ACK commit closure failed exact readback"
            );
            connection.execute_batch("RELEASE accepted_apply_v215")?;
            Ok(ComputeAttemptDispatchAckCommit::Activated {
                ack: ack_receipt(stored_ack, false),
                application: stored_application.into_receipt(false),
                accepted_closure,
                activation,
            })
        }
        _ => bail!("Unsupported Adapter ACK outcome"),
    }
}

fn ensure_accepted_currentness_on(
    connection: &Connection,
    command_id: &str,
    checked_at: &str,
    mode: AcceptedAckIngressMode<'_, '_, '_>,
) -> Result<AcceptedStartCommitFreshness> {
    match mode {
        AcceptedAckIngressMode::Fresh => {
            ensure_fresh_accepted_start_commit_on(connection, command_id, checked_at)
        }
        AcceptedAckIngressMode::HistoricalTerminal(authority) => {
            ensure_historical_accepted_start_commit_on(
                connection, command_id, checked_at, authority,
            )
        }
    }
}

fn record_observation_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    times: Option<&ComputeAttemptAdapterAckIngressTimes>,
) -> Result<()> {
    match times {
        Some(times) => {
            record_verified_observation_at_on(
                connection,
                verified.prepare_observation(),
                times.observation_transitioned_at(),
            )?;
        }
        None => {
            record_verified_observation_on(connection, verified.prepare_observation())?;
        }
    }
    Ok(())
}

fn record_rejected_no_start_on(
    connection: &Connection,
    verified: &dyn VerifiedComputeAttemptAdapterAckView,
    times: Option<&ComputeAttemptAdapterAckIngressTimes>,
) -> Result<()> {
    match times {
        Some(times) => {
            record_prepare_rejected_no_start_at_on(connection, verified, times.ingested_at())?;
        }
        None => {
            record_prepare_rejected_no_start_on(connection, verified)?;
        }
    }
    Ok(())
}
