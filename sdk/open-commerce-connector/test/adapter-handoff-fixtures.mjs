export const adapterToken = `oc_adapter_${'a'.repeat(64)}`
export const leaseToken = `oc_claim_${'b'.repeat(64)}`

export function jsonResponse(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

export function claim(overrides = {}) {
  const now = Date.now()
  return {
    schema: 'open_commerce.adapter_business_handoff_claim.v1',
    id: 'handoffclaim-1',
    project_id: 'project-demo',
    merchant_id: 'merchant-demo',
    invocation_id: 'invocation-demo-1',
    integration_id: 'integration-demo',
    adapter_credential_id: 'adapter-credential-1',
    adapter_credential_version: 1,
    attempt_no: 1,
    status: 'active',
    lease_token_hint: '...bbbbbb',
    lease_expires_at: new Date(now + 60_000).toISOString(),
    lease_deadline_at: new Date(now + 3_600_000).toISOString(),
    release_reason_code: null,
    released_at: null,
    completion_status: null,
    retry_not_before: null,
    retry_suspended_at: null,
    retry_suspension_reason: null,
    retry_resumed_at: null,
    completed_receipt_id: null,
    created_at: '2026-08-10T00:00:00.000Z',
    updated_at: '2026-08-10T00:00:00.000Z',
    ...overrides,
  }
}

export function issue(overrides = {}) {
  const claimed = overrides.claim ?? claim()
  return {
    claim: claimed,
    lease_token: leaseToken,
    lease_token_visible_once: true,
    task: {
      evidence: {
        schema: 'open_commerce.merchant_business_evidence.v1',
        invocation_id: claimed.invocation_id,
        merchant_id: claimed.merchant_id,
        status: 'succeeded',
        receipt_state: 'valid',
        result_available: true,
      },
      result: { order: { id: 'order-1' } },
    },
    ...overrides,
    claim: claimed,
  }
}

export function receipt(sourceClaim = claim(), overrides = {}) {
  return {
    schema: 'open_commerce.business_handoff_receipt.v1',
    id: 'handoff-receipt-1',
    project_id: sourceClaim.project_id,
    merchant_id: sourceClaim.merchant_id,
    invocation_id: sourceClaim.invocation_id,
    integration_id: sourceClaim.integration_id,
    receipt_key: 'erp-order-1',
    status: 'applied',
    target_domain: 'erp',
    evidence_result_sha256: 'c'.repeat(64),
    target_reference_sha256: 'd'.repeat(64),
    error_code: null,
    confirmed_by_user: false,
    assertion_authority: 'adapter_token_authenticated',
    adapter_credential_id: sourceClaim.adapter_credential_id,
    adapter_credential_version: sourceClaim.adapter_credential_version,
    adapter_claim_id: sourceClaim.id,
    recorded_by_user_id: 'user-owner-1',
    recorded_by_app_id: `adapter-${sourceClaim.adapter_credential_id}`,
    completed_at: '2026-08-10T00:00:01.000Z',
    created_at: '2026-08-10T00:00:01.000Z',
    funds_moved: false,
    ...overrides,
  }
}
