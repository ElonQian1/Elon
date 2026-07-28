//! Transactional heartbeat and finish mutations for release leases.

use super::*;

pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let mut candidate = guard.clone();
    let now = now_secs();

    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut candidate, lane_kind), now);
    }
    sweep_global_expired(&mut candidate, now);

    let mut lease_entry = candidate
        .global_publish
        .owners
        .iter()
        .find(|item| item.token == req.token && item.kind == kind.as_str())
        .or_else(|| {
            candidate
                .global_publish
                .waiters
                .iter()
                .find(|item| item.token == req.token && item.kind == kind.as_str())
        })
        .cloned()
        .ok_or_else(|| {
            err(
                StatusCode::GONE,
                "token-not-active",
                "lease expired or unknown token",
            )
        })?;
    if lease_entry.batch_id.is_empty() {
        let Some(batch_id) = req
            .batch_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            return Err(err(
                StatusCode::CONFLICT,
                "legacy-batch-required",
                "legacy publish token must adopt its deterministic release batch",
            ));
        };
        if batch_id
            != crate::release_batch::default_batch_id_for_kind(kind.as_str(), &lease_entry.sha)
        {
            return Err(err(
                StatusCode::CONFLICT,
                "legacy-batch-migration-refused",
                "legacy publish token may only adopt its deterministic immutable-sha batch",
            ));
        }
        crate::release_batch::validate_batch_identity(&candidate, batch_id, &lease_entry.sha)
            .map_err(|message| err(StatusCode::CONFLICT, "batch-sha-mismatch", message))?;
        adopt_legacy_batch_identity(&mut candidate, kind, &req.token, batch_id);
        lease_entry.batch_id = batch_id.to_string();
        lease_entry.stage = crate::release_batch::default_stage(kind.as_str()).to_string();
    }
    if req.sha.trim() != lease_entry.sha {
        return Err(err(
            StatusCode::CONFLICT,
            "immutable-sha-mismatch",
            "heartbeat sha must match the immutable sha captured by claim",
        ));
    }
    let stage = heartbeat_stage(&candidate, &lease_entry, &req)?;
    let lease = clamp_lease(req.lease_secs.unwrap_or(DEFAULT_LEASE_SECS));
    let new_expiry = now + lease;

    if let Some(owner) = candidate
        .global_publish
        .owners
        .iter_mut()
        .find(|item| item.token == req.token && item.kind == kind.as_str())
    {
        owner.last_heartbeat = now;
        owner.lease_expires_at = new_expiry;
    } else if let Some(waiter) = candidate
        .global_publish
        .waiters
        .iter_mut()
        .find(|item| item.token == req.token && item.kind == kind.as_str())
    {
        waiter.last_heartbeat = now;
        waiter.lease_expires_at = new_expiry;
    } else {
        return Err(err(
            StatusCode::GONE,
            "token-not-active",
            "lease disappeared before heartbeat commit",
        ));
    }
    crate::release_batch::record_stage_phase(
        &mut candidate,
        &lease_entry.batch_id,
        &lease_entry.sha,
        &lease_entry.kind,
        stage,
        req.phase.as_deref(),
        req.phase_status.as_deref(),
        &lease_entry.builder_id,
        &lease_entry.builder_label,
        req.stage_status.as_deref().unwrap_or("running"),
        new_expiry,
        None,
        now,
    );
    if let Some(item) = lane_mut(&mut candidate, kind)
        .in_flight
        .iter_mut()
        .find(|item| item.token == req.token)
    {
        item.last_heartbeat = now;
        item.lease_expires_at = new_expiry;
    }

    commit_candidate(&mgr, &mut guard, candidate).await?;
    Ok(Json(HeartbeatResponse {
        ok: true,
        lease_expires_at: new_expiry,
    }))
}

pub async fn finish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinishRequest>,
) -> Result<Json<FinishResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let mut candidate = guard.clone();
    let now = now_secs();
    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut candidate, lane_kind), now);
    }
    sweep_global_expired(&mut candidate, now);

    if let Some(completion) = candidate
        .global_publish
        .completed
        .iter()
        .rev()
        .find(|completion| completion.token == req.token)
    {
        let identity_matches = completion.kind == kind.as_str()
            && completion.sha == req.sha
            && completion.batch_id == req.batch_id
            && completion.stage == req.stage
            && completion.success == req.success;
        if !identity_matches {
            return Err(err(
                StatusCode::CONFLICT,
                "terminal-evidence-mismatch",
                "completed release token is bound to different terminal evidence",
            ));
        }
        commit_candidate(&mgr, &mut guard, candidate).await?;
        return Ok(Json(FinishResponse {
            ok: true,
            recorded: false,
        }));
    }

    let owner = candidate
        .global_publish
        .owners
        .iter()
        .find(|owner| owner.token == req.token && owner.kind == kind.as_str())
        .cloned()
        .ok_or_else(|| {
            err(
                StatusCode::CONFLICT,
                "not-publish-owner",
                "only the current global publish owner may finish this lease",
            )
        })?;
    validate_finish_owner(&owner, &req)?;
    let build = lane(&candidate, kind)
        .in_flight
        .iter()
        .find(|item| item.token == req.token)
        .cloned();
    if let Some(build) = build.as_ref() {
        validate_finish_identity(build, &req)
            .map_err(|(kind, message)| err(StatusCode::CONFLICT, kind, message))?;
    }

    let recorded = if let Some(build) = build {
        lane_mut(&mut candidate, kind)
            .in_flight
            .retain(|item| item.token != req.token);
        let final_vn = build.assigned_version_name.clone();
        let final_vc = build.assigned_version_code;
        let lane_state = lane_mut(&mut candidate, kind);
        lane_state.last_release = Some(LastRelease {
            success: req.success,
            sha: build.sha,
            version_name: final_vn.clone(),
            version_code: final_vc,
            finished_at: now,
            builder_label: build.builder_label,
            error_message: req.error_message.clone(),
        });
        if req.success {
            lane_state.last_published_version_name = Some(
                lane_state
                    .last_published_version_name
                    .as_deref()
                    .filter(|previous| !semver_ge(&final_vn, previous))
                    .map(ToOwned::to_owned)
                    .unwrap_or(final_vn),
            );
            if let Some(code) = final_vc {
                lane_state.last_published_version_code = Some(
                    lane_state
                        .last_published_version_code
                        .map(|previous| previous.max(code))
                        .unwrap_or(code),
                );
            }
        }
        crate::release_batch::record_stage(
            &mut candidate,
            &owner.batch_id,
            &owner.sha,
            &owner.kind,
            &owner.stage,
            &owner.builder_id,
            &owner.builder_label,
            if req.success { "succeeded" } else { "failed" },
            owner.lease_expires_at,
            req.error_message.clone(),
            now,
        );
        finish_global_publish(
            &mut candidate,
            &owner,
            req.success,
            req.error_message.clone(),
            now,
        );
        true
    } else {
        false
    };

    commit_candidate(&mgr, &mut guard, candidate).await?;
    Ok(Json(FinishResponse { ok: true, recorded }))
}
