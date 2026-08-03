export const ADAPTER_HANDOFF_CLAIM_SCHEMA = 'open_commerce.adapter_business_handoff_claim.v1'
export const ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA = 'open_commerce.adapter_business_handoff_claim_poll.v1'
export const ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA = 'open_commerce.adapter_business_handoff_claim_release.v1'
export const ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA = 'open_commerce.adapter_business_handoff_claim_renew.v1'
export const ADAPTER_HANDOFF_MAX_RESPONSE_BYTES = 256 * 1024

const RELEASE_REASONS = new Set([
  'adapter_shutdown',
  'capacity_pressure',
  'transient_failure',
  'manual_release',
])

export class AdapterHandoffClientError extends Error {
  constructor(code, message, status = undefined) {
    super(message)
    this.name = 'AdapterHandoffClientError'
    this.code = code
    this.status = status
  }
}

export function createAdapterHandoffClient(options) {
  expectObject(options, 'options')
  const baseUrl = normalizeBaseUrl(options.baseUrl)
  const token = expectSecret(options.token)
  const fetchImpl = options.fetch ?? globalThis.fetch
  if (typeof fetchImpl !== 'function') {
    fail('missing_fetch', 'options.fetch or globalThis.fetch is required')
  }

  const request = (path, body, signal) =>
    requestJson(fetchImpl, baseUrl, token, path, body, signal)

  return Object.freeze({
    async claimNext({ leaseSeconds = 300, signal } = {}) {
      if (!Number.isInteger(leaseSeconds) || leaseSeconds < 60 || leaseSeconds > 900) {
        fail('invalid_lease_seconds', 'leaseSeconds must be an integer between 60 and 900')
      }
      const poll = await request(
        '/api/open-commerce/adapter/business-handoff-claims',
        { lease_seconds: leaseSeconds },
        signal,
      )
      return validateClaimPoll(poll)
    },

    async complete(issue, receipt, { signal } = {}) {
      validateClaimIssue(issue)
      expectObject(receipt, 'receipt')
      return request(
        `/api/open-commerce/adapter/business-handoff-claims/${encodeURIComponent(issue.claim.id)}/complete`,
        {
          lease_token: issue.lease_token,
          receipt_key: expectIdentifier(receipt.receiptKey, 'receipt.receiptKey', 3, 128),
          status: expectOneOf(receipt.status, ['applied', 'ignored', 'rejected'], 'receipt.status'),
          target_domain: expectIdentifier(receipt.targetDomain, 'receipt.targetDomain', 2, 96),
          target_reference: optionalIdentifier(
            receipt.targetReference,
            'receipt.targetReference',
            1,
            240,
          ),
          error_code: optionalIdentifier(receipt.errorCode, 'receipt.errorCode', 2, 96),
          completed_at: expectTimestamp(receipt.completedAt, 'receipt.completedAt'),
        },
        signal,
      )
    },

    async release(issue, reasonCode, { signal } = {}) {
      validateClaimIssue(issue)
      if (!RELEASE_REASONS.has(reasonCode)) {
        fail(
          'invalid_release_reason',
          'reasonCode must be adapter_shutdown, capacity_pressure, transient_failure, or manual_release',
        )
      }
      const released = await request(
        `/api/open-commerce/adapter/business-handoff-claims/${encodeURIComponent(issue.claim.id)}/release`,
        { lease_token: issue.lease_token, reason_code: reasonCode },
        signal,
      )
      if (released?.schema !== ADAPTER_HANDOFF_CLAIM_RELEASE_SCHEMA || released.retryable !== true) {
        fail('invalid_release_response', 'adapter release response does not match the V1 contract')
      }
      return released
    },

    async renew(issue, { extendSeconds = 300, signal } = {}) {
      validateClaimIssue(issue)
      if (!Number.isInteger(extendSeconds) || extendSeconds < 60 || extendSeconds > 900) {
        fail('invalid_extend_seconds', 'extendSeconds must be an integer between 60 and 900')
      }
      const renewed = await request(
        `/api/open-commerce/adapter/business-handoff-claims/${encodeURIComponent(issue.claim.id)}/renew`,
        { lease_token: issue.lease_token, extend_seconds: extendSeconds },
        signal,
      )
      if (renewed?.schema !== ADAPTER_HANDOFF_CLAIM_RENEW_SCHEMA || renewed.renewed !== true) {
        fail('invalid_renew_response', 'adapter renew response does not match the V1 contract')
      }
      return renewed
    },
  })
}

function validateClaimPoll(poll) {
  expectObject(poll, 'poll')
  if (poll.schema !== ADAPTER_HANDOFF_CLAIM_POLL_SCHEMA || typeof poll.claimed !== 'boolean') {
    fail('invalid_claim_poll', 'adapter claim response does not match the V1 poll contract')
  }
  if (!poll.claimed) {
    if (poll.issue !== null && poll.issue !== undefined) {
      fail('invalid_claim_poll', 'an empty poll must not contain a claim issue')
    }
    return poll
  }
  validateClaimIssue(poll.issue)
  return poll
}

function validateClaimIssue(issue) {
  expectObject(issue, 'issue')
  expectObject(issue.claim, 'issue.claim')
  if (issue.claim.schema !== ADAPTER_HANDOFF_CLAIM_SCHEMA) {
    fail('invalid_claim_issue', 'claim schema is unsupported')
  }
  expectIdentifier(issue.claim.id, 'issue.claim.id', 3, 160)
  if (typeof issue.lease_token !== 'string' || !issue.lease_token.startsWith('oc_claim_')) {
    fail('invalid_claim_issue', 'claim lease token is missing or malformed')
  }
  expectObject(issue.task, 'issue.task')
  expectObject(issue.task.evidence, 'issue.task.evidence')
  if (!Object.hasOwn(issue.task, 'result')) {
    fail('invalid_claim_issue', 'claim task result is missing')
  }
  return issue
}

async function requestJson(fetchImpl, baseUrl, token, path, body, signal) {
  const response = await fetchImpl(new URL(path, `${baseUrl}/`), {
    method: 'POST',
    headers: {
      authorization: `Bearer ${token}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify(body),
    signal,
  })
  const contentLength = Number(response.headers?.get?.('content-length'))
  if (Number.isFinite(contentLength) && contentLength > ADAPTER_HANDOFF_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'adapter API response exceeds the 256 KiB SDK limit', response.status)
  }
  const text = await response.text()
  if (Buffer.byteLength(text, 'utf8') > ADAPTER_HANDOFF_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'adapter API response exceeds the 256 KiB SDK limit', response.status)
  }
  let payload
  try {
    payload = text ? JSON.parse(text) : null
  } catch {
    fail('invalid_json_response', 'adapter API returned invalid JSON', response.status)
  }
  if (!response.ok) {
    const message = typeof payload?.error === 'string'
      ? payload.error.slice(0, 240)
      : `adapter API returned HTTP ${response.status}`
    fail('adapter_http_error', message, response.status)
  }
  return payload
}

function normalizeBaseUrl(value) {
  if (typeof value !== 'string' || !value.trim()) {
    fail('invalid_base_url', 'options.baseUrl is required')
  }
  let url
  try {
    url = new URL(value.trim())
  } catch {
    fail('invalid_base_url', 'options.baseUrl must be an absolute HTTP URL')
  }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    fail('invalid_base_url', 'options.baseUrl must be an HTTP origin without credentials, query, or hash')
  }
  if (url.protocol === 'http:' && !['localhost', '127.0.0.1', '::1'].includes(url.hostname)) {
    fail('insecure_base_url', 'non-local adapter traffic must use HTTPS')
  }
  return url.toString().replace(/\/$/, '')
}

function expectSecret(value) {
  if (typeof value !== 'string' || value.length < 32 || /\s/.test(value)) {
    fail('invalid_token', 'options.token is missing or malformed')
  }
  return value
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    fail('invalid_object', `${path} must be an object`)
  }
}

function expectIdentifier(value, path, min, max) {
  if (typeof value !== 'string' || value.trim().length < min || value.trim().length > max) {
    fail('invalid_identifier', `${path} must contain ${min}-${max} characters`)
  }
  if (!/^[A-Za-z0-9._:-]+$/.test(value.trim())) {
    fail('invalid_identifier', `${path} contains unsupported characters`)
  }
  return value.trim()
}

function optionalIdentifier(value, path, min, max) {
  return value === undefined || value === null || value === ''
    ? undefined
    : expectIdentifier(value, path, min, max)
}

function expectOneOf(value, allowed, path) {
  if (!allowed.includes(value)) fail('invalid_value', `${path} is unsupported`)
  return value
}

function expectTimestamp(value, path) {
  if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) {
    fail('invalid_timestamp', `${path} must be an RFC3339 timestamp`)
  }
  return new Date(value).toISOString()
}

function fail(code, message, status = undefined) {
  throw new AdapterHandoffClientError(code, message, status)
}
