'use strict'

const { fingerprint, sourceFingerprint } = require('./identity')

const MAX_KEYS = 10000
const CODES = new Set([
  'INVALID_PLATFORM_SNAPSHOT', 'PLATFORM_SNAPSHOT_DIGEST_MISMATCH',
  'PLATFORM_SNAPSHOT_SOURCE_MISMATCH', 'PLATFORM_SNAPSHOT_FROM_FUTURE',
  'PLATFORM_SNAPSHOT_STALE', 'COMBINED_HISTORY_TOO_LARGE',
])

class SnapshotError extends Error {
  constructor(code = 'INVALID_PLATFORM_SNAPSHOT') {
    const safe = CODES.has(code) ? code : 'INVALID_PLATFORM_SNAPSHOT'
    super(safe)
    this.code = safe
  }
}

function ensure(condition, code) {
  if (!condition) throw new SnapshotError(code)
}

function exactObject(value, names) {
  ensure(value !== null && typeof value === 'object' && !Array.isArray(value))
  ensure([Object.prototype, null].includes(Object.getPrototypeOf(value)))
  const keys = Reflect.ownKeys(value)
  ensure(keys.length === names.length && keys.every(key => names.includes(key)))
  for (const key of names) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    ensure(descriptor && Object.hasOwn(descriptor, 'value') && descriptor.enumerable)
  }
}

const isDigest = value => typeof value === 'string' && value.length === 64 && /^[0-9a-f]+$/.test(value)

function count(value) {
  ensure(typeof value === 'string' && value.length <= 5)
  const number = Number(value)
  ensure(Number.isInteger(number) && number >= 0 && number <= MAX_KEYS && String(number) === value)
  return number
}

function keys(value) {
  ensure(Array.isArray(value) && value.length <= MAX_KEYS)
  ensure(Reflect.ownKeys(value).length === value.length + 1)
  for (let index = 0; index < value.length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index))
    ensure(descriptor && Object.hasOwn(descriptor, 'value') && descriptor.enumerable)
    ensure(isDigest(descriptor.value))
    if (index) ensure(value[index - 1] < descriptor.value)
  }
}

function validatePlatformSnapshot(snapshot, reconciliation) {
  exactObject(snapshot, [
    'schema', 'scope', 'source_fingerprint', 'policy_digest', 'observed_at',
    'used_payment_keys', 'prepared_count', 'recorded_count', 'key_count',
    'platform_history_complete', 'external_history_complete', 'funds_moved',
    'balances_written', 'external_payment_verified', 'snapshot_digest',
  ])
  ensure(snapshot.schema === 'yilong.esk.platform_payment_snapshot.v1'
    && snapshot.scope === 'platform_recorded_allocations_only'
    && snapshot.platform_history_complete === true && snapshot.external_history_complete === false
    && snapshot.funds_moved === false && snapshot.balances_written === false
    && snapshot.external_payment_verified === false)
  for (const key of ['source_fingerprint', 'policy_digest', 'snapshot_digest']) ensure(isDigest(snapshot[key]))
  keys(snapshot.used_payment_keys)
  ensure(count(snapshot.prepared_count) + count(snapshot.recorded_count) === count(snapshot.key_count))
  ensure(Number(snapshot.key_count) === snapshot.used_payment_keys.length)
  const observed = snapshot.observed_at
  ensure(typeof observed === 'string' && observed.length === 24
    && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(observed))
  const observedMs = Date.parse(observed)
  ensure(Number.isFinite(observedMs) && new Date(observedMs).toISOString() === observed)
  ensure(fingerprint({ ...snapshot, snapshot_digest: null }) === snapshot.snapshot_digest,
    'PLATFORM_SNAPSHOT_DIGEST_MISMATCH')
  ensure(sourceFingerprint(reconciliation.source) === snapshot.source_fingerprint,
    'PLATFORM_SNAPSHOT_SOURCE_MISMATCH')
  const age = Date.parse(reconciliation.as_of) - observedMs
  ensure(age >= 0, 'PLATFORM_SNAPSHOT_FROM_FUTURE')
  ensure(age <= 24 * 60 * 60 * 1000, 'PLATFORM_SNAPSHOT_STALE')
  return snapshot
}

function joinHistory(input, platform) {
  // Keep duplicate keys in the operator snapshot so the original validator
  // still reports HISTORY_DUPLICATE_KEYS; only cross-source overlap is normal.
  const used = [...input.snapshot.used_payment_keys]
  const seen = new Set(used)
  for (const key of platform.used_payment_keys) {
    if (!seen.has(key)) { seen.add(key); used.push(key) }
  }
  ensure(used.length <= MAX_KEYS, 'COMBINED_HISTORY_TOO_LARGE')
  const manualTime = input.snapshot.observed_at
  // A future manual timestamp must remain visible to the original algorithm.
  const observedAt = Date.parse(manualTime) > Date.parse(input.as_of)
    ? manualTime : manualTime < platform.observed_at ? manualTime : platform.observed_at
  return {
    ...input,
    snapshot: { ...input.snapshot, snapshot_id: `platform-join:${fingerprint(input.snapshot)}`,
      observed_at: observedAt, used_payment_keys: used.sort() },
  }
}

module.exports = { MAX_KEYS, SnapshotError, exactObject, validatePlatformSnapshot, joinHistory }
