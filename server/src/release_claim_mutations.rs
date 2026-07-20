//! Heartbeat and finish mutations for release leases.

use super::*;

pub async fn heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();

    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut guard, lane_kind), now);
    }
    sweep_global_expired(&mut guard, now);

    let lease = clamp_lease(req.lease_secs.unwrap_or(DEFAULT_LEASE_SECS));
    let new_expiry = now + lease;
    let mut found = false;
    if let Some(owner) = guard
        .global_publish
        .owner
        .as_mut()
        .filter(|item| item.token == req.token && item.kind == kind.as_str())
    {
        owner.last_heartbeat = now;
        owner.lease_expires_at = new_expiry;
        found = true;
    } else if let Some(waiter) = guard
        .global_publish
        .waiters
        .iter_mut()
        .find(|item| item.token == req.token && item.kind == kind.as_str())
    {
        waiter.last_heartbeat = now;
        waiter.lease_expires_at = new_expiry;
        found = true;
    }
    if !found {
        return Err(err(
            StatusCode::GONE,
            "token-not-active",
            "lease expired or unknown token",
        ));
    }
    let mut lease_entry = guard
        .global_publish
        .owner
        .as_ref()
        .filter(|item| item.token == req.token)
        .or_else(|| {
            guard
                .global_publish
                .waiters
                .iter()
                .find(|item| item.token == req.token)
        })
        .cloned()
        .ok_or_else(|| err(StatusCode::GONE, "token-not-active", "lease disappeared"))?;
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
        if batch_id != crate::release_batch::default_batch_id(&lease_entry.sha) {
            return Err(err(
                StatusCode::CONFLICT,
                "legacy-batch-migration-refused",
                "legacy publish token may only adopt release-<immutable-sha>",
            ));
        }
        crate::release_batch::validate_batch_identity(&guard, batch_id, &lease_entry.sha)
            .map_err(|message| err(StatusCode::CONFLICT, "batch-sha-mismatch", message))?;
        adopt_legacy_batch_identity(&mut guard, kind, &req.token, batch_id);
        lease_entry.batch_id = batch_id.to_string();
        lease_entry.stage = crate::release_batch::default_stage(kind.as_str()).to_string();
    }
    let stage = heartbeat_stage(&guard, &lease_entry, &req)?;
    crate::release_batch::record_stage(
        &mut guard,
        &lease_entry.batch_id,
        &lease_entry.sha,
        &lease_entry.kind,
        stage,
        &lease_entry.builder_id,
        &lease_entry.builder_label,
        req.stage_status.as_deref().unwrap_or("running"),
        new_expiry,
        None,
        now,
    );
    if let Some(item) = lane_mut(&mut guard, kind)
        .in_flight
        .iter_mut()
        .find(|item| item.token == req.token)
    {
        item.last_heartbeat = now;
        item.lease_expires_at = new_expiry;
    }
    let lease_expires_at = new_expiry;

    let resp = HeartbeatResponse {
        ok: true,
        lease_expires_at,
    };
    persist_or_restore(&mgr, &mut guard, original).await?;
    drop(guard);
    Ok(Json(resp))
}

pub async fn finish_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<FinishRequest>,
) -> Result<Json<FinishResponse>, (StatusCode, Json<Value>)> {
    let kind = parse_kind(&req.kind)?;
    let mgr = manager(&state);
    ensure_manager_healthy(&mgr)?;
    let mut guard = mgr.inner.lock().await;
    let original = guard.clone();
    let now = now_secs();
    for lane_kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        sweep_expired(lane_mut(&mut guard, lane_kind), now);
    }
    sweep_global_expired(&mut guard, now);

    let owner = guard
        .global_publish
        .owner
        .as_ref()
        .filter(|owner| owner.token == req.token && owner.kind == kind.as_str())
        .cloned();
    let Some(owner) = owner else {
        return Err(err(
            StatusCode::CONFLICT,
            "not-publish-owner",
            "only the current global publish owner may finish this lease",
        ));
    };
    validate_finish_owner(&owner, &req)?;

    let lane_mut_ref = lane_mut(&mut guard, kind);
    let pos = lane_mut_ref
        .in_flight
        .iter()
        .position(|b| b.token == req.token);
    let removed = pos.map(|i| lane_mut_ref.in_flight.remove(i));

    let recorded = if let Some(b) = removed {
        if let Err((error_kind, error_message)) = validate_finish_identity(&b, &req) {
            lane_mut(&mut guard, kind).in_flight.push(b);
            return Err(err(StatusCode::CONFLICT, error_kind, error_message));
        }
        let final_vn = b.assigned_version_name.clone();
        let final_vc = b.assigned_version_code;
        let final_sha = b.sha.clone();
        let last = LastRelease {
            success: req.success,
            sha: final_sha,
            version_name: final_vn.clone(),
            version_code: final_vc,
            finished_at: now,
            builder_label: b.builder_label,
            error_message: req.error_message.clone(),
        };
        lane_mut_ref.last_release = Some(last);

        if req.success {
            let prev_vn = lane_mut_ref.last_published_version_name.clone();
            let new_vn = match prev_vn {
                Some(p) => {
                    if semver_ge(&final_vn, &p) {
                        final_vn
                    } else {
                        p
                    }
                }
                None => final_vn,
            };
            lane_mut_ref.last_published_version_name = Some(new_vn);
            if let Some(c) = final_vc {
                let new_c = lane_mut_ref
                    .last_published_version_code
                    .map(|prev| prev.max(c))
                    .unwrap_or(c);
                lane_mut_ref.last_published_version_code = Some(new_c);
            }
        }
        crate::release_batch::record_stage(
            &mut guard,
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
            &mut guard,
            &owner,
            req.success,
            req.error_message.clone(),
            now,
        );
        true
    } else {
        false
    };

    let resp = FinishResponse { ok: true, recorded };
    persist_or_restore(&mgr, &mut guard, original).await?;
    drop(guard);
    Ok(Json(resp))
}
