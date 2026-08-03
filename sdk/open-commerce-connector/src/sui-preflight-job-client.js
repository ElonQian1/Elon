import {
  SUI_PREFLIGHT_MAX_RESPONSE_BYTES,
  SuiPreflightContractError,
  verifySuiAdapterHandoff,
} from './sui-preflight.js'

export const SUI_PREFLIGHT_JOB_POLL_SCHEMA = 'task_economy.sui_preflight_job_poll.v1'
export const SUI_PREFLIGHT_JOB_RENEW_SCHEMA = 'task_economy.sui_preflight_job_renew.v1'
export const SUI_PREFLIGHT_JOB_RELEASE_SCHEMA = 'task_economy.sui_preflight_job_release.v1'
export const SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA = 'task_economy.sui_preflight_job_complete.v1'

export function createSuiPreflightJobClient(options) {
  expectObject(options, 'options')
  const baseUrl = normalizeBaseUrl(options.baseUrl)
  const token = expectAdapterToken(options.token)
  const fetchImpl = options.fetch ?? globalThis.fetch
  if (typeof fetchImpl !== 'function') {
    fail('missing_fetch', 'options.fetch or globalThis.fetch is required')
  }

  async function post(path, body, expectedSchema, signal) {
    const response = await fetchImpl(new URL(path, `${baseUrl}/`), {
      method: 'POST',
      headers: {
        authorization: `Bearer ${token}`,
        'content-type': 'application/json',
      },
      body: JSON.stringify(body),
      signal,
    })
    return readJsonResponse(response, expectedSchema)
  }

  return Object.freeze({
    async claimNext({ leaseSeconds = 300, signal } = {}) {
      const payload = await post(
        '/api/economy/sui-preflight/jobs/claim',
        { lease_seconds: expectLeaseSeconds(leaseSeconds, 'leaseSeconds') },
        SUI_PREFLIGHT_JOB_POLL_SCHEMA,
        signal,
      )
      if (!payload.claimed) return payload
      const issue = validateIssue(payload.issue)
      return Object.freeze({ ...payload, issue })
    },

    async renew(jobId, leaseToken, { extendSeconds = 300, signal } = {}) {
      return post(
        `/api/economy/sui-preflight/jobs/${encodeURIComponent(expectJobId(jobId))}/renew`,
        {
          lease_token: expectLeaseToken(leaseToken),
          extend_seconds: expectLeaseSeconds(extendSeconds, 'extendSeconds'),
        },
        SUI_PREFLIGHT_JOB_RENEW_SCHEMA,
        signal,
      )
    },

    async release(jobId, leaseToken, { reason, signal } = {}) {
      return post(
        `/api/economy/sui-preflight/jobs/${encodeURIComponent(expectJobId(jobId))}/release`,
        {
          lease_token: expectLeaseToken(leaseToken),
          reason: expectText(reason, 'reason', 4, 500),
        },
        SUI_PREFLIGHT_JOB_RELEASE_SCHEMA,
        signal,
      )
    },

    async complete(jobId, leaseToken, input, { signal } = {}) {
      expectObject(input, 'completion')
      return post(
        `/api/economy/sui-preflight/jobs/${encodeURIComponent(expectJobId(jobId))}/complete`,
        {
          lease_token: expectLeaseToken(leaseToken),
          outcome: expectOneOf(input.outcome, new Set(['passed', 'rejected']), 'outcome'),
          summary: expectText(input.summary, 'summary', 4, 500),
          tool_version: expectText(input.toolVersion, 'toolVersion', 1, 100),
          idempotency_key: expectIdentifier(
            input.idempotencyKey,
            'idempotencyKey',
            8,
            128,
          ),
        },
        SUI_PREFLIGHT_JOB_COMPLETE_SCHEMA,
        signal,
      )
    },
  })
}

function validateIssue(issue) {
  expectObject(issue, 'claim.issue')
  expectObject(issue.job, 'claim.issue.job')
  const leaseToken = expectLeaseToken(issue.lease_token)
  const handoff = issue.handoff
  const verified = verifySuiAdapterHandoff(handoff)
  if (
    issue.job.id !== expectJobId(issue.job.id)
    || issue.job.project_id !== verified.projectId
    || issue.job.package_kind !== verified.packageKind
    || issue.job.projection_package_id !== verified.projectionPackageId
    || issue.job.target_network !== verified.targetNetwork
    || issue.job.projection_digest !== verified.projectionDigest
    || issue.job.handoff_digest !== verified.handoffDigest
  ) {
    fail('claim_binding_mismatch', 'claimed job does not match its deterministic handoff')
  }
  return Object.freeze({ ...issue, lease_token: leaseToken, handoff })
}

async function readJsonResponse(response, expectedSchema) {
  const contentLength = Number(response.headers?.get?.('content-length'))
  if (Number.isFinite(contentLength) && contentLength > SUI_PREFLIGHT_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'preflight job response exceeds 256 KiB', response.status)
  }
  const text = await response.text()
  if (Buffer.byteLength(text, 'utf8') > SUI_PREFLIGHT_MAX_RESPONSE_BYTES) {
    fail('response_too_large', 'preflight job response exceeds 256 KiB', response.status)
  }
  let payload
  try {
    payload = text ? JSON.parse(text) : null
  } catch {
    fail('invalid_json_response', 'preflight job API returned invalid JSON', response.status)
  }
  if (!response.ok) {
    const message = typeof payload?.error === 'string'
      ? payload.error.slice(0, 240)
      : `preflight job API returned HTTP ${response.status}`
    fail('preflight_http_error', message, response.status)
  }
  if (payload?.schema !== expectedSchema) {
    fail('invalid_job_response', 'preflight job response schema is unsupported')
  }
  return payload
}

function normalizeBaseUrl(value) {
  if (typeof value !== 'string' || !value.trim()) fail('invalid_base_url', 'baseUrl is required')
  let url
  try {
    url = new URL(value.trim())
  } catch {
    fail('invalid_base_url', 'baseUrl must be an absolute HTTP URL')
  }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    fail('invalid_base_url', 'baseUrl must be an HTTP origin without credentials, query, or hash')
  }
  if (url.protocol === 'http:' && !['localhost', '127.0.0.1', '::1'].includes(url.hostname)) {
    fail('insecure_base_url', 'non-local preflight traffic must use HTTPS')
  }
  return url.toString().replace(/\/$/, '')
}

function expectAdapterToken(value) {
  if (typeof value !== 'string' || !value.startsWith('sui_preflight_') || value.length < 52 || /\s/.test(value)) {
    fail('invalid_token', 'Sui preflight adapter token is missing or malformed')
  }
  return value
}

function expectLeaseToken(value) {
  if (typeof value !== 'string' || !value.startsWith('sui_preflight_lease_') || value.length < 58 || /\s/.test(value)) {
    fail('invalid_lease_token', 'Sui preflight lease token is missing or malformed')
  }
  return value
}

function expectJobId(value) {
  return expectIdentifier(value, 'jobId', 8, 160)
}

function expectLeaseSeconds(value, path) {
  if (!Number.isInteger(value) || value < 60 || value > 900) {
    fail('invalid_lease_seconds', `${path} must be an integer from 60 to 900`)
  }
  return value
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    fail('invalid_object', `${path} must be an object`)
  }
}

function expectOneOf(value, allowed, path) {
  if (!allowed.has(value)) fail('invalid_value', `${path} is unsupported`)
  return value
}

function expectIdentifier(value, path, min, max) {
  const text = expectText(value, path, min, max)
  if (!/^[A-Za-z0-9._:-]+$/.test(text)) {
    fail('invalid_identifier', `${path} contains unsupported characters`)
  }
  return text
}

function expectText(value, path, min, max) {
  if (typeof value !== 'string') fail('invalid_text', `${path} must be text`)
  const text = value.trim()
  if (text.length < min || text.length > max) {
    fail('invalid_text', `${path} must contain ${min}-${max} characters`)
  }
  return text
}

function fail(code, message, status = undefined) {
  throw new SuiPreflightContractError(code, message, status)
}
