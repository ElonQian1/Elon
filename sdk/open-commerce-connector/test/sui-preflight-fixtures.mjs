import { createHash } from 'node:crypto'

export const adapterToken = `sui_preflight_${'a'.repeat(64)}`
export const leaseToken = `sui_preflight_lease_${'b'.repeat(64)}`

export function jsonResponse(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

export function handoff(overrides = {}) {
  const bundle = {
    schema: 'task_economy.sui_adapter_handoff.v1',
    package_kind: 'standard',
    project_id: 'project-demo',
    projection_package_id: 'projection-package-1',
    source_id: 'receipt-source-1',
    target_network: 'testnet',
    package_schema: 'task_economy.sui_projection_package.v1',
    projection_digest: 'c'.repeat(64),
    source_digest: 'd'.repeat(64),
    envelope: { operation: 'offline_preflight' },
    shadow_only: true,
    atomic_bundle: false,
    network_submission: 'not_submitted',
    submission_attempts: 0,
    package_created_at: '2026-08-10T00:00:00.000Z',
    constraints: {
      allowed_adapter_action: 'offline_preflight_only',
      signature_present: false,
      transaction_broadcast: false,
      finality_verified: false,
      funds_moved: false,
    },
    ...overrides,
  }
  bundle.constraints = { ...bundle.constraints, ...overrides.constraints }
  bundle.atomic_bundle = bundle.package_kind === 'correction'
  bundle.handoff_digest = sha256(JSON.stringify(handoffPayload(bundle)))
  if (overrides.handoff_digest !== undefined) {
    bundle.handoff_digest = overrides.handoff_digest
  }
  return bundle
}

export function job(bundle = handoff(), overrides = {}) {
  const now = Date.now()
  return {
    schema: 'task_economy.sui_preflight_job.v1',
    id: 'preflight-job-1',
    project_id: bundle.project_id,
    package_kind: bundle.package_kind,
    projection_package_id: bundle.projection_package_id,
    target_network: bundle.target_network,
    handoff_digest: bundle.handoff_digest,
    projection_digest: bundle.projection_digest,
    status: 'leased',
    adapter_id: 'preflight-adapter-1',
    credential_version: 1,
    attempt_no: 1,
    lease_token_hint: '...bbbbbb',
    lease_started_at: new Date(now - 1_000).toISOString(),
    lease_expires_at: new Date(now + 60_000).toISOString(),
    lease_deadline_at: new Date(now + 3_600_000).toISOString(),
    report_id: null,
    last_error: null,
    created_by_user_id: 'user-owner-1',
    completed_at: null,
    canceled_at: null,
    created_at: '2026-08-10T00:00:00.000Z',
    updated_at: '2026-08-10T00:00:00.000Z',
    ...overrides,
  }
}

export function issue(bundle = handoff(), overrides = {}) {
  return {
    schema: 'task_economy.sui_preflight_job_issue.v1',
    job: job(bundle),
    lease_token: leaseToken,
    lease_token_visible_once: true,
    handoff: bundle,
    ...overrides,
  }
}

export function report(bundle = handoff(), overrides = {}) {
  return {
    schema: 'task_economy.sui_preflight_report.v1',
    id: 'preflight-report-1',
    project_id: bundle.project_id,
    adapter_id: 'preflight-adapter-1',
    credential_version: 1,
    package_kind: bundle.package_kind,
    projection_package_id: bundle.projection_package_id,
    target_network: bundle.target_network,
    handoff_digest: bundle.handoff_digest,
    projection_digest: bundle.projection_digest,
    outcome: 'passed',
    summary: 'deterministic offline preflight passed',
    tool_version: 'test-adapter/1.0.0',
    idempotency_key: 'preflight-test-001',
    report_digest: 'e'.repeat(64),
    created_at: '2026-08-10T00:00:01.000Z',
    ...overrides,
  }
}

function handoffPayload(bundle) {
  return {
    schema: bundle.schema,
    package_kind: bundle.package_kind,
    project_id: bundle.project_id,
    projection_package_id: bundle.projection_package_id,
    source_id: bundle.source_id,
    target_network: bundle.target_network,
    package_schema: bundle.package_schema,
    projection_digest: bundle.projection_digest,
    source_digest: bundle.source_digest,
    envelope: bundle.envelope,
    shadow_only: bundle.shadow_only,
    atomic_bundle: bundle.atomic_bundle,
    network_submission: bundle.network_submission,
    submission_attempts: bundle.submission_attempts,
    package_created_at: bundle.package_created_at,
    constraints: bundle.constraints,
  }
}

function sha256(value) {
  return createHash('sha256').update(value, 'utf8').digest('hex')
}
