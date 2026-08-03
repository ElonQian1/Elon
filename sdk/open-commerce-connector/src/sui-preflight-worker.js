import { createHash } from 'node:crypto'

import { createSuiPreflightJobClient } from './sui-preflight-job-client.js'

export const SUI_PREFLIGHT_WORKER_SCHEMA = 'task_economy.sui_preflight_worker_result.v1'

const OUTCOMES = new Set(['passed', 'rejected'])

export class SuiPreflightReleaseError extends Error {
  constructor(reason, message = reason) {
    super(message)
    this.name = 'SuiPreflightReleaseError'
    this.reason = expectText(reason, 'reason', 4, 500)
  }
}

export function createSuiPreflightWorker(options) {
  expectObject(options, 'options')
  const client = options.client ?? createSuiPreflightJobClient(options)
  expectClient(client)
  if (typeof options.handler !== 'function') throw new TypeError('options.handler must be a function')
  const toolVersion = expectText(options.toolVersion, 'options.toolVersion', 1, 100)
  const leaseSeconds = boundedInteger(options.leaseSeconds ?? 300, 60, 900, 'leaseSeconds')
  const extendSeconds = boundedInteger(
    options.extendSeconds ?? leaseSeconds,
    60,
    900,
    'extendSeconds',
  )
  const renewBeforeSeconds = boundedInteger(
    options.renewBeforeSeconds ?? Math.min(60, Math.floor(leaseSeconds / 3)),
    10,
    Math.max(10, leaseSeconds - 10),
    'renewBeforeSeconds',
  )
  const idleDelayMs = boundedInteger(options.idleDelayMs ?? 5_000, 250, 60_000, 'idleDelayMs')
  const errorDelayMs = boundedInteger(options.errorDelayMs ?? 5_000, 250, 60_000, 'errorDelayMs')
  const completionAttempts = boundedInteger(
    options.completionAttempts ?? 3,
    1,
    5,
    'completionAttempts',
  )

  async function runOnce({ signal } = {}) {
    throwIfAborted(signal)
    const poll = await client.claimNext({ leaseSeconds, signal })
    if (!poll.claimed) {
      return Object.freeze({
        schema: SUI_PREFLIGHT_WORKER_SCHEMA,
        claimed: false,
        retryAfterMs: Math.max(idleDelayMs, Number(poll.retry_after_seconds ?? 0) * 1_000),
      })
    }
    return processIssue(poll.issue, signal)
  }

  async function processIssue(initialIssue, outerSignal) {
    let issue = initialIssue
    let renewalFailure
    const stopRenewal = new AbortController()
    const stopHandler = new AbortController()
    const abortHandler = () => stopHandler.abort(outerSignal?.reason)
    outerSignal?.addEventListener('abort', abortHandler, { once: true })
    const renewal = renewUntilStopped(
      () => issue,
      (job) => { issue = { ...issue, job } },
      stopRenewal.signal,
    ).catch((error) => {
      renewalFailure = error
      stopHandler.abort(error)
    })

    try {
      const rawOutcome = await options.handler(issue.handoff, {
        job: issue.job,
        attemptNo: issue.job.attempt_no,
        idempotencyKey: stableIdempotencyKey(issue.job),
        signal: stopHandler.signal,
      })
      stopRenewal.abort()
      await renewal
      if (renewalFailure) throw renewalFailure
      const outcome = normalizeOutcome(rawOutcome)
      const completion = await completeWithRetry(issue, outcome)
      return Object.freeze({
        schema: SUI_PREFLIGHT_WORKER_SCHEMA,
        claimed: true,
        jobId: completion.job.id,
        status: completion.job.status,
        outcome: completion.report.outcome,
        reportId: completion.report.id,
        retryAfterMs: 0,
      })
    } catch (error) {
      stopRenewal.abort()
      await renewal
      const failure = renewalFailure ?? error
      const reason = outerSignal?.aborted
        ? 'preflight worker shutdown before completion'
        : failure instanceof SuiPreflightReleaseError
          ? failure.reason
          : `transient preflight failure: ${safeMessage(failure)}`
      await releaseBestEffort(issue, reason)
      if (outerSignal?.aborted) throw abortError(outerSignal.reason)
      throw failure
    } finally {
      outerSignal?.removeEventListener('abort', abortHandler)
    }
  }

  async function renewUntilStopped(getIssue, setJob, signal) {
    while (!signal.aborted) {
      const current = getIssue()
      const renewAt = timestamp(current.job.lease_expires_at, 'lease_expires_at')
        - renewBeforeSeconds * 1_000
      try {
        await delay(Math.max(250, renewAt - Date.now()), signal)
      } catch (error) {
        if (signal.aborted) return
        throw error
      }
      if (signal.aborted) return
      const renewed = await client.renew(current.job.id, current.lease_token, {
        extendSeconds,
        signal,
      })
      setJob(renewed.job)
      if (timestamp(renewed.job.lease_deadline_at, 'lease_deadline_at') <= Date.now()) {
        throw new Error('Sui preflight hard lease deadline reached')
      }
    }
  }

  async function completeWithRetry(issue, outcome) {
    let lastError
    for (let attempt = 1; attempt <= completionAttempts; attempt += 1) {
      try {
        return await client.complete(issue.job.id, issue.lease_token, {
          ...outcome,
          toolVersion,
          idempotencyKey: stableIdempotencyKey(issue.job),
        })
      } catch (error) {
        lastError = error
        if (attempt < completionAttempts) await delay(250 * (2 ** (attempt - 1)))
      }
    }
    throw lastError
  }

  async function releaseBestEffort(issue, reason) {
    try {
      await client.release(issue.job.id, issue.lease_token, {
        reason: reason.slice(0, 500),
        signal: AbortSignal.timeout(5_000),
      })
    } catch {
      // The bounded lease becomes retryable even when shutdown cannot reach the server.
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

function normalizeOutcome(outcome) {
  expectObject(outcome, 'handler outcome')
  if (!OUTCOMES.has(outcome.outcome)) {
    throw new TypeError('handler outcome.outcome must be passed or rejected')
  }
  return {
    outcome: outcome.outcome,
    summary: expectText(outcome.summary, 'outcome.summary', 4, 500),
  }
}

function stableIdempotencyKey(job) {
  const digest = createHash('sha256')
    .update(`${job.id}:${job.attempt_no}`, 'utf8')
    .digest('hex')
    .slice(0, 40)
  return `sui-preflight-${digest}`
}

function expectClient(client) {
  expectObject(client, 'options.client')
  for (const method of ['claimNext', 'renew', 'release', 'complete']) {
    if (typeof client[method] !== 'function') throw new TypeError(`options.client.${method} is required`)
  }
}

function expectObject(value, path) {
  if (!value || Array.isArray(value) || typeof value !== 'object') {
    throw new TypeError(`${path} must be an object`)
  }
}

function expectText(value, path, min, max) {
  if (typeof value !== 'string' || value.trim().length < min || value.trim().length > max) {
    throw new TypeError(`${path} must contain ${min}-${max} characters`)
  }
  return value.trim()
}

function boundedInteger(value, min, max, path) {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new TypeError(`${path} must be an integer between ${min} and ${max}`)
  }
  return value
}

function timestamp(value, path) {
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) throw new TypeError(`${path} must be a valid timestamp`)
  return parsed
}

function throwIfAborted(signal) {
  if (signal?.aborted) throw abortError(signal.reason)
}

function abortError(reason) {
  const error = new Error(reason ? String(reason) : 'operation aborted')
  error.name = 'AbortError'
  return error
}

function safeMessage(error) {
  return error instanceof Error ? error.message.slice(0, 440) : 'operation failed'
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
