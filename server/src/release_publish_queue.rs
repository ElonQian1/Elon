//! Durable publish queue operations shared by all release lanes.

use crate::release_manager::{
    lane_mut, Lane, PublishCompletion, PublishLeaseEntry, ReleaseStateFile,
};

pub(crate) const PUBLISH_HEARTBEAT_TIMEOUT_SECS: i64 = 180;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PublishAdmission {
    Owner,
    Waiter { queue_position: usize },
}

pub(crate) fn normalize_legacy_owner(state: &mut ReleaseStateFile) {
    let Some(owner) = state.global_publish.owner.take() else {
        return;
    };
    if !state
        .global_publish
        .owners
        .iter()
        .any(|current| current.token == owner.token)
    {
        state.global_publish.owners.push(owner);
    }
}

pub(crate) fn owner_for_kind<'a>(
    state: &'a ReleaseStateFile,
    kind: &str,
) -> Option<&'a PublishLeaseEntry> {
    state
        .global_publish
        .owners
        .iter()
        .find(|owner| owner.kind == kind)
}

pub(crate) fn owner_by_token<'a>(
    state: &'a ReleaseStateFile,
    token: &str,
) -> Option<&'a PublishLeaseEntry> {
    state
        .global_publish
        .owners
        .iter()
        .find(|owner| owner.token == token)
}

pub(crate) fn active_owner_count(state: &ReleaseStateFile) -> usize {
    state.global_publish.owners.len()
}

pub(crate) fn waiter_count_for_kind(state: &ReleaseStateFile, kind: &str) -> usize {
    state
        .global_publish
        .waiters
        .iter()
        .filter(|waiter| waiter.kind == kind)
        .count()
}

pub(crate) fn queue_position_for_token(
    state: &ReleaseStateFile,
    kind: &str,
    token: &str,
) -> Option<usize> {
    state
        .global_publish
        .waiters
        .iter()
        .filter(|waiter| waiter.kind == kind)
        .position(|waiter| waiter.token == token)
        .map(|index| index + 1)
}

fn entry_expired(entry: &PublishLeaseEntry, now: i64) -> bool {
    entry.lease_expires_at <= now
        || entry
            .last_heartbeat
            .saturating_add(PUBLISH_HEARTBEAT_TIMEOUT_SECS)
            <= now
}

pub(crate) fn sweep_global_expired(state: &mut ReleaseStateFile, now: i64) {
    normalize_legacy_owner(state);
    let owners = std::mem::take(&mut state.global_publish.owners);
    for owner in owners {
        if entry_expired(&owner, now) {
            crate::release_batch::expire_stage(state, &owner, now);
            remove_publish_token_from_lanes(state, &owner.token);
            state.global_publish.completed.push(PublishCompletion {
                token: owner.token,
                kind: owner.kind,
                sha: owner.sha,
                batch_id: owner.batch_id,
                stage: owner.stage,
                success: false,
                coalesced: false,
                finished_at: now,
                error_message: Some("publish owner lease expired".to_string()),
            });
        } else {
            state.global_publish.owners.push(owner);
        }
    }
    let expired_waiters = state
        .global_publish
        .waiters
        .iter()
        .filter(|waiter| entry_expired(waiter, now))
        .map(|waiter| waiter.token.clone())
        .collect::<Vec<_>>();
    state
        .global_publish
        .waiters
        .retain(|waiter| !entry_expired(waiter, now));
    for token in expired_waiters {
        remove_publish_token_from_lanes(state, &token);
    }
    for kind in ["server", "apk", "node_agent"] {
        promote_next_for_kind(state, kind);
    }
    if state.global_publish.completed.len() > 200 {
        let keep_from = state.global_publish.completed.len() - 200;
        state.global_publish.completed.drain(..keep_from);
    }
}

pub(crate) fn enqueue_global_publish(
    state: &mut ReleaseStateFile,
    entry: PublishLeaseEntry,
) -> PublishAdmission {
    normalize_legacy_owner(state);
    if owner_for_kind(state, &entry.kind).is_none() {
        state.global_publish.owners.push(entry);
        PublishAdmission::Owner
    } else {
        let kind = entry.kind.clone();
        state.global_publish.waiters.push(entry);
        PublishAdmission::Waiter {
            queue_position: waiter_count_for_kind(state, &kind),
        }
    }
}

pub(crate) fn finish_global_publish(
    state: &mut ReleaseStateFile,
    owner: &PublishLeaseEntry,
    success: bool,
    error_message: Option<String>,
    now: i64,
) -> Vec<String> {
    normalize_legacy_owner(state);
    state
        .global_publish
        .owners
        .retain(|current| current.token != owner.token);
    state.global_publish.completed.push(PublishCompletion {
        token: owner.token.clone(),
        kind: owner.kind.clone(),
        sha: owner.sha.clone(),
        batch_id: owner.batch_id.clone(),
        stage: owner.stage.clone(),
        success,
        coalesced: false,
        finished_at: now,
        error_message,
    });

    let mut coalesced_tokens = Vec::new();
    if success {
        state.global_publish.waiters.retain(|waiter| {
            let same_release = waiter.kind == owner.kind
                && waiter.sha == owner.sha
                && waiter.batch_id == owner.batch_id
                && waiter.stage == owner.stage;
            if same_release {
                coalesced_tokens.push(waiter.token.clone());
            }
            !same_release
        });
        for token in &coalesced_tokens {
            remove_publish_token_from_lanes(state, token);
            state.global_publish.completed.push(PublishCompletion {
                token: token.clone(),
                kind: owner.kind.clone(),
                sha: owner.sha.clone(),
                batch_id: owner.batch_id.clone(),
                stage: owner.stage.clone(),
                success: true,
                coalesced: true,
                finished_at: now,
                error_message: None,
            });
        }
    }
    promote_next_for_kind(state, &owner.kind);
    coalesced_tokens
}

fn promote_next_for_kind(state: &mut ReleaseStateFile, kind: &str) {
    if owner_for_kind(state, kind).is_some() {
        return;
    }
    if let Some(index) = state
        .global_publish
        .waiters
        .iter()
        .position(|waiter| waiter.kind == kind)
    {
        let promoted = state.global_publish.waiters.remove(index);
        state.global_publish.owners.push(promoted);
    }
}

fn remove_publish_token_from_lanes(state: &mut ReleaseStateFile, token: &str) {
    for kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        lane_mut(state, kind)
            .in_flight
            .retain(|item| item.token != token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    fn lease(token: &str, kind: &str, sha: &str, expires: i64) -> PublishLeaseEntry {
        PublishLeaseEntry {
            token: token.to_string(),
            kind: kind.to_string(),
            sha: sha.to_string(),
            batch_id: crate::release_batch::default_batch_id_for_kind(kind, sha),
            stage: crate::release_batch::default_stage(kind).to_string(),
            builder_id: token.to_string(),
            builder_label: token.to_string(),
            requested_at: 1,
            last_heartbeat: 1,
            lease_expires_at: expires,
        }
    }

    #[test]
    fn same_lane_publish_is_fifo_under_concurrent_claims() {
        let state = Arc::new(StdMutex::new(ReleaseStateFile::default()));
        let mut joins = Vec::new();
        for index in 0..8 {
            let state = state.clone();
            joins.push(std::thread::spawn(move || {
                let mut guard = state.lock().expect("publish test state");
                enqueue_global_publish(
                    &mut guard,
                    lease(
                        &format!("token-{index}"),
                        "server",
                        &format!("sha-{index}"),
                        100,
                    ),
                )
            }));
        }
        for join in joins {
            join.join().expect("claim thread");
        }
        let guard = state.lock().expect("publish test state");
        assert_eq!(guard.global_publish.owners.len(), 1);
        assert_eq!(guard.global_publish.waiters.len(), 7);
        let requested = guard
            .global_publish
            .owners
            .iter()
            .chain(guard.global_publish.waiters.iter())
            .map(|item| item.token.clone())
            .collect::<Vec<_>>();
        assert_eq!(requested.len(), 8);
        let unique = requested.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn independent_lanes_do_not_block_each_other_and_same_sha_waiters_coalesce() {
        let mut state = ReleaseStateFile::default();
        let owner = lease("owner", "node_agent", "sha-a", 100);
        assert_eq!(
            enqueue_global_publish(&mut state, owner.clone()),
            PublishAdmission::Owner
        );
        enqueue_global_publish(&mut state, lease("same", "node_agent", "sha-a", 100));
        enqueue_global_publish(&mut state, lease("next", "apk", "sha-b", 100));
        assert_eq!(state.global_publish.owners.len(), 2);
        assert_eq!(owner_for_kind(&state, "apk").unwrap().token, "next");

        let coalesced = finish_global_publish(&mut state, &owner, true, None, 10);

        assert_eq!(coalesced, vec!["same"]);
        assert_eq!(owner_for_kind(&state, "apk").unwrap().token, "next");
        assert!(owner_for_kind(&state, "node_agent").is_none());
        assert!(state
            .global_publish
            .completed
            .iter()
            .any(|item| item.token == "same" && item.coalesced));
    }

    #[test]
    fn stale_heartbeat_yields_to_first_same_lane_waiter_without_blocking_apk() {
        let mut state = ReleaseStateFile::default();
        let mut expired = lease("expired", "server", "sha-a", 10_000);
        expired.last_heartbeat = 1;
        crate::release_batch::record_claim(&mut state, &expired, "running", 1);
        state.global_publish.owner = Some(expired);
        let mut first = lease("first", "server", "sha-b", 10_000);
        first.last_heartbeat = 100;
        let mut apk = lease("apk", "apk", "sha-c", 10_000);
        apk.last_heartbeat = 100;
        state.global_publish.waiters = vec![first, apk];

        sweep_global_expired(&mut state, 182);

        assert_eq!(owner_for_kind(&state, "server").unwrap().token, "first");
        assert_eq!(owner_for_kind(&state, "apk").unwrap().token, "apk");
        assert!(state.global_publish.waiters.is_empty());
        assert_eq!(state.release_batches[0].status, "failed_closed");
        assert_eq!(state.release_batches[0].stages[0].status, "expired");
    }

    #[test]
    fn legacy_single_owner_is_migrated_without_losing_identity() {
        let mut state = ReleaseStateFile::default();
        state.global_publish.owner = Some(lease("legacy", "server", "sha-a", 1_000));

        sweep_global_expired(&mut state, 2);

        assert!(state.global_publish.owner.is_none());
        assert_eq!(owner_for_kind(&state, "server").unwrap().token, "legacy");
    }
}
