import { createHash } from 'node:crypto'

import { createAdapterHandoffClient } from './adapter-handoff-client.js'

export const ADAPTER_HANDOFF_WORKER_SCHEMA = 'open_commerce.adapter_handoff_worker_result.v1'

const RELEASE_REASONS = new Set([
  'adapter_shutdown',
  'capacity_pressure',
  'transient_failure',
  'manual_release',
])
const COMPLETION_STATUSES = new Set(['applied', 'ignored', 'rejected'])

export class AdapterHandoffReleaseError extends Error {
  constructor(reasonCode, message = reasonCode) {
    super(message)
    this.name = 'AdapterHandoffReleaseError'
    this.reasonCode = expectReleaseReason(reasonCode)
  }
}

export class AdapterHandoffRejectError extends Error {
  constructor(errorCode, message = errorCode) {
    super(message)
    this.name = 'AdapterHandoffRejectError'
    this.errorCode = expectIdentifier(errorCode, 'errorCode', 2, 96)
  }
}

export function createAdapterHandoffWorker(options) {
  expectObject(options, 'options')
  const client = options.client ?? createAdapterHandoffClient(options)
  expectClient(client)
  if (typeof options.handler !== 'function') {
    throw new TypeError('options.handler must be a function')
  }
  const targetDomain = expectIdentifier(options.targetDomain, 'options.targetDomain', 2, 96)
  const leaseSeconds = boundedInteger(options.leaseSeconds ?? 300, 60, 900, 'options.leaseSeconds')
  const extendSeconds = boundedInteger(
    options.extendSeconds ?? leaseSeconds,
    60,
    900,
    'options.extendSeconds',
  )
  const renewBeforeSeconds = boundedInteger(
    options.renewBeforeSeconds ?? Math.min(60, Math.floor(leaseSeconds / 3)),
    10,
    Math.max(10, leaseSeconds - 10),
    'options.renewBeforeSeconds',
  )
  const idleDelayMs = boundedInteger(options.idleDelayMs ?? 5_000, 250, 60_000, 'options.idleDelayMs')
  const errorDelayMs = boundedInteger(options.errorDelayMs ?? 5_000, 250, 60_000, 'options.errorDelayMs')
  const completionAttempts = boundedInteger(
    options.completionAttempts ?? 3,
    1,
    5,
    'options.completionAttempts',
  )

  async function runOnce({ signal } = {}) {
    throwIfAborted(signal)
    const poll = await client.claimNext({ leaseSeconds, signal })
    if (!poll.claimed) {
      return {
        schema: ADAPTER_HANDOFF_WORKER_SCHEMA,
        claimed: false,
        retryAfterMs: Math.max(idleDelayMs, Number(poll.retry_after_seconds ?? 0) * 1_000),
      }
    }
    return processIssue(poll.issue, signal)
  }

  async function processIssue(initialIssue, outerSignal) {
    let currentIssue = initialIssue
    let renewalFailure
    const renewalStop = new AbortController()
    const handlerAbort = new AbortController()
    const abortHandler = () => handlerAbort.abort(outerSignal?.reason)
    outerSignal?.addEventListener('abort', abortHandler, { once: true })
    const renewal = renewUntilStopped(
      () => currentIssue,
      (issue) => { currentIssue = issue },
      renewalStop.signal,
      handlerAbort,
    ).catch((error) => {
      renewalFailure = error
      handlerAbort.abort(error)
    })

    try {
      const outcome = await options.handler(currentIssue.task, {
        claim: currentIssue.claim,
        idempotencyKey: currentIssue.claim.invocation_id,
        attemptNo: currentIssue.claim.attempt_no,
        signal: handlerAbort.signal,
      })
      renewalStop.abort()
      await renewal
      if (renewalFailure) throw renewalFailure
      const completion = normalizeOutcome(outcome, currentIssue, targetDomain)
      const receipt = await completeWithRetry(currentIssue, completion, completionAttempts)
      return {
        schema: ADAPTER_HANDOFF_WORKER_SCHEMA,
        claimed: true,
        claimId: currentIssue.claim.id,
        invocationId: currentIssue.claim.invocation_id,
        status: completion.status,
        receipt,
        retryAfterMs: 0,
      }
    } catch (error) {
      renewalStop.abort()
      await renewal
      if (renewalFailure && error !== renewalFailure) throw renewalFailure
      if (error instanceof AdapterHandoffRejectError) {
        const completion = normalizeOutcome({
          status: 'rejected',
          errorCode: error.errorCode,
        }, currentIssue, targetDomain)
        const receipt = await completeWithRetry(currentIssue, completion, completionAttempts)
        return {
          schema: ADAPTER_HANDOFF_WORKER_SCHEMA,
          claimed: true,
          claimId: currentIssue.claim.id,
          invocationId: currentIssue.claim.invocation_id,
          status: 'rejected',
          receipt,
          retryAfterMs: 0,
        }
      }
      const reasonCode = outerSignal?.aborted
        ? 'adapter_shutdown'
        : error instanceof AdapterHandoffReleaseError
          ? error.reasonCode
          : 'transient_failure'
      await releaseBestEffort(currentIssue, reasonCode)
      if (outerSignal?.aborted) throw abortError(outerSignal.reason)
      throw error
    } finally {
      outerSignal?.removeEventListener('abort', abortHandler)
    }
  }

  async function renewUntilStopped(getIssue, setIssue, signal, handlerAbort) {
    while (!signal.aborted) {
      const issue = getIssue()
      const renewAt = Date.parse(issue.claim.lease_expires_at) - renewBeforeSeconds * 1_000
      await delay(Math.max(250, renewAt - Date.now()), signal)
      if (signal.aborted) return
      const renewed = await client.renew(issue, { extendSeconds, signal })
      setIssue({ ...issue, claim: renewed.claim })
      if (Date.parse(renewed.claim.lease_deadline_at) <= Date.now()) {
        handlerAbort.abort(new Error('adapter handoff hard lease deadline reached'))
        return
      }
    }
  }

  async function completeWithRetry(issue, completion, attempts) {
    let lastError
    for (let attempt = 1; attempt <= attempts; attempt += 1) {
      try {
        return await client.complete(issue, completion)
      } catch (error) {
        lastError = error
        if (attempt < attempts) await delay(250 * (2 ** (attempt - 1)))
      }
    }
    throw lastError
  }

  async function releaseBestEffort(issue, reasonCode) {
    try {
      await client.release(issue, reasonCode, { signal: AbortSignal.timeout(5_000) })
    } catch {
      // The lease will expire and become retryable even if shutdown cannot reach the server.
    }
  }

  async function run({ signal, onResult, onError } = {}) {
    let claimed = 0
    let completed = 0
    let failed = 0
    while (!signal?.aborted) {
      try {
        const result = await runOnce({ signal })
        if (result.claimed) {
          claimed += 1
          completed += 1
        }
        await onResult?.(result)
        if (result.retryAfterMs > 0) await delay(result.retryAfterMs, signal)
      } catch (error) {
        if (signal?.aborted || error?.name === 'AbortError') break
        failed += 1
        await onError?.(error)
        await delay(errorDelayMs, signal)
      }
    }
    return Object.freeze({ claimed, completed, failed })
  }

  return Object.freeze({ run, runOnce })
}

function normalizeOutcome(outcome, issue, targetDomain) {
  expectObject(outcome, 'handler outcome')
  if (!COMPLETION_STATUSES.has(outcome.status)) {
    throw new TypeError('handler outcome.status must be applied, ignored, or rejected')
  }
  const targetReference = optionalIdentifier(
    outcome.targetReference,
    'outcome.targetReference',
    1,
    160,
  )
  const errorCode = optionalIdentifier(outcome.errorCode, 'outcome.errorCode', 2, 96)
  if (outcome.status === 'applied' && (!targetReference || errorCode)) {
    throw new TypeError('an applied outcome requires targetReference and forbids errorCode')
  }
  if (outcome.status !== 'applied' && (targetReference || !errorCode)) {
    throw new TypeError('an ignored or rejected outcome requires errorCode and forbids targetReference')
  }
  return {
    receiptKey: outcome.receiptKey
      ? expectIdentifier(outcome.receiptKey, 'outcome.receiptKey', 3, 128)
      : stableReceiptKey(issue.claim.id),
    status: outcome.status,
    targetDomain,
    targetReference,
    errorCode,
    completedAt: outcome.completedAt ?? new Date().toISOString(),
  }
}

function stableReceiptKey(claimId) {
  return `adapter-${createHash('sha256').update(claimId, 'utf8').digest('hex').slice(0, 40)}`
}

function expectClient(client) {
  expectObject(client, 'options.client')
  for (const method of ['claimNext', 'renew', 'complete', 'release']) {
    if (typeof client[method] !== 'function') throw new TypeError(`options.client.${method} is required`)
  }
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${path} must be an object`)
  }
}

function expectIdentifier(value, path, min, max) {
  if (typeof value !== 'string' || value.trim().length < min || value.trim().length > max) {
    throw new TypeError(`${path} must contain ${min}-${max} characters`)
  }
  if (!/^[A-Za-z0-9._:-]+$/.test(value.trim())) {
    throw new TypeError(`${path} contains unsupported characters`)
  }
  return value.trim()
}

function optionalIdentifier(value, path, min, max) {
  return value === undefined || value === null || value === ''
    ? undefined
    : expectIdentifier(value, path, min, max)
}

function expectReleaseReason(value) {
  if (!RELEASE_REASONS.has(value)) throw new TypeError('unsupported adapter release reason')
  return value
}

function boundedInteger(value, min, max, path) {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new TypeError(`${path} must be an integer between ${min} and ${max}`)
  }
  return value
}

function throwIfAborted(signal) {
  if (signal?.aborted) throw abortError(signal.reason)
}

function abortError(reason) {
  const error = new Error(reason ? String(reason) : 'operation aborted')
  error.name = 'AbortError'
  return error
}

function delay(milliseconds, signal) {
  if (signal?.aborted) return Promise.reject(abortError(signal.reason))
  return new Promise((resolve, reject) => {
    let timer
    const abort = () => {
      clearTimeout(timer)
      signal?.removeEventListener('abort', abort)
      reject(abortError(signal.reason))
    }
    const complete = () => {
      signal?.removeEventListener('abort', abort)
      resolve()
    }
    timer = setTimeout(complete, Math.max(0, milliseconds))
    signal?.addEventListener('abort', abort, { once: true })
  })
}
