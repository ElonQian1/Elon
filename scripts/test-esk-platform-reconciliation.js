'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { fingerprint, paymentKey } = require('./esk-paid-reconciliation/identity')
const { previewWithPlatformSnapshot } = require('./esk-paid-reconciliation/platform-preview')

let passed = 0
function test(name, run) {
  try { run(); passed += 1 } catch (error) { error.message = `${name}: ${error.message}`; throw error }
}
const clone = value => JSON.parse(JSON.stringify(value))
const sign = value => { value.snapshot_digest = fingerprint({ ...value, snapshot_digest: null }); return value }
function fixture() {
  const reconciliation = JSON.parse(fs.readFileSync(path.join(__dirname,
    '../contracts/assets/esk-paid-reconciliation-v1.fixture.json'), 'utf8'))
  return { schema: 'yilong.esk.platform_reconciliation_input.v1', reconciliation,
    platform_snapshot: sign({
      schema: 'yilong.esk.platform_payment_snapshot.v1', scope: 'platform_recorded_allocations_only',
      source_fingerprint: reconciliation.snapshot.source_fingerprint, policy_digest: 'd'.repeat(64),
      observed_at: reconciliation.as_of, used_payment_keys: [], prepared_count: '0', recorded_count: '0',
      key_count: '0', platform_history_complete: true, external_history_complete: false,
      funds_moved: false, balances_written: false, external_payment_verified: false,
    }) }
}
function occupied(input, status = 'prepared') {
  input.platform_snapshot.used_payment_keys = [paymentKey(input.reconciliation.source, input.reconciliation.rows[0])]
  input.platform_snapshot[`${status}_count`] = '1'
  input.platform_snapshot.key_count = '1'
  sign(input.platform_snapshot)
  return input
}
function reject(input, code) {
  const result = previewWithPlatformSnapshot(input)
  assert.equal(result.status, 'invalid_input')
  assert.equal(result.preview, null)
  assert.equal(result.commit_eligible, false)
  if (code) assert.equal(result.error_code, code)
}
function reason(input, expected) {
  const result = previewWithPlatformSnapshot(input)
  assert.equal(result.status, 'needs_review')
  assert(result.preview.rows[0].reasons.includes(expected), JSON.stringify(result))
  assert.deepEqual(result.preview.proposed_totals, [])
}

test('unused payment is reviewable without becoming money or authenticated evidence', () => {
  const input = fixture()
  const before = JSON.stringify(input)
  const result = previewWithPlatformSnapshot(input)
  assert.equal(result.status, 'review_ready')
  assert.equal(result.preview.proposed_totals[0].esk_base_units, '10000000')
  for (const flag of ['funds_moved', 'balances_written', 'commit_eligible', 'platform_snapshot_authenticity_verified']) {
    assert.equal(result[flag], false)
  }
  assert.equal(result.input_digest, fingerprint(input))
  assert.equal(result.platform_snapshot_digest, input.platform_snapshot.snapshot_digest)
  assert.equal(result.report_digest, fingerprint({ ...result, report_digest: null }))
  assert.equal(result.preview.report_digest, fingerprint({ ...result.preview, report_digest: null }))
  assert.equal(JSON.stringify(input), before)
  assert(!JSON.stringify(result).includes(input.reconciliation.rows[0].external_payment_reference))
})
for (const status of ['prepared', 'recorded']) test(`${status} payment blocks the original algorithm`, () => {
  reason(occupied(fixture(), status), 'PAYMENT_ALREADY_USED')
})
test('payment key is independent of user and amount', () => {
  const input = occupied(fixture())
  input.reconciliation.users[0].target_user_ref = 'b'.repeat(64)
  input.reconciliation.rows[0].payment_amount = '22'
  input.reconciliation.rows[0].esk_base_units = '11000000'
  reason(input, 'PAYMENT_ALREADY_USED')
})
test('same transaction different actual event is not the same identity', () => {
  const input = occupied(fixture())
  input.reconciliation.rows[0].transfer_index = 1
  assert.equal(previewWithPlatformSnapshot(input).status, 'review_ready')
})
test('manual and platform overlap is not a duplicate manual history error', () => {
  const input = occupied(fixture())
  input.reconciliation.snapshot.used_payment_keys = [...input.platform_snapshot.used_payment_keys]
  const result = previewWithPlatformSnapshot(input)
  assert(result.preview.rows[0].reasons.includes('PAYMENT_ALREADY_USED'))
  assert(!result.preview.rows[0].reasons.includes('HISTORY_DUPLICATE_KEYS'))
})
test('manual duplicates remain errors after joining', () => {
  const input = occupied(fixture())
  input.reconciliation.snapshot.used_payment_keys = Array(2).fill(input.platform_snapshot.used_payment_keys[0])
  reason(input, 'HISTORY_DUPLICATE_KEYS')
})
for (const [name, mutate, expected] of [
  ['incomplete', s => { s.history_complete = false }, 'HISTORY_INCOMPLETE'],
  ['stale', s => { s.observed_at = '2026-09-02T00:00:00.000Z' }, 'SNAPSHOT_STALE'],
  ['future', s => { s.observed_at = '2026-09-04T07:00:00.000Z' }, 'SNAPSHOT_FROM_FUTURE'],
  ['source', s => { s.source_fingerprint = '0'.repeat(64) }, 'SNAPSHOT_SOURCE_MISMATCH'],
]) test(`platform coverage does not replace manual ${name} check`, () => {
  const input = fixture(); mutate(input.reconciliation.snapshot); reason(input, expected)
})
for (const [name, mutate, expected] of [
  ['stale', s => { s.observed_at = '2026-09-02T00:00:00.000Z' }, 'PLATFORM_SNAPSHOT_STALE'],
  ['future', s => { s.observed_at = '2026-09-04T06:00:00.001Z' }, 'PLATFORM_SNAPSHOT_FROM_FUTURE'],
  ['source', s => { s.source_fingerprint = '0'.repeat(64) }, 'PLATFORM_SNAPSHOT_SOURCE_MISMATCH'],
  ['calendar', s => { s.observed_at = '2026-02-30T00:00:00.000Z' }, 'INVALID_PLATFORM_SNAPSHOT'],
  ['precision', s => { s.observed_at = '2026-09-04T06:00:00Z' }, 'INVALID_PLATFORM_SNAPSHOT'],
  ['zone', s => { s.observed_at = '2026-09-04T06:00:00.000+00:00' }, 'INVALID_PLATFORM_SNAPSHOT'],
]) test(`reject platform ${name}`, () => {
  const input = fixture(); mutate(input.platform_snapshot); sign(input.platform_snapshot); reject(input, expected)
})
test('one-day boundary is included', () => {
  const input = fixture(); input.platform_snapshot.observed_at = '2026-09-03T06:00:00.000Z'
  sign(input.platform_snapshot); assert.equal(previewWithPlatformSnapshot(input).status, 'review_ready')
})
test('tampered snapshot is rejected without returning its data', () => {
  const input = fixture(); input.platform_snapshot.policy_digest = 'c'.repeat(64)
  reject(input, 'PLATFORM_SNAPSHOT_DIGEST_MISMATCH')
})
for (const [field, value] of [
  ['schema', 'legacy'], ['scope', 'all_payments'], ['platform_history_complete', false],
  ['external_history_complete', true], ['funds_moved', true], ['balances_written', true],
  ['external_payment_verified', true], ['key_count', 0], ['key_count', '00'],
  ['key_count', '1'], ['prepared_count', '-1'], ['recorded_count', '10001'],
  ['source_fingerprint', 'a'.repeat(63)], ['policy_digest', 'A'.repeat(64)],
]) test(`strict ${field}=${value}`, () => {
  const input = fixture(); input.platform_snapshot[field] = value; sign(input.platform_snapshot); reject(input)
})
test('duplicate and unsorted platform keys are rejected', () => {
  for (const keys of [['a'.repeat(64), 'a'.repeat(64)], ['b'.repeat(64), 'a'.repeat(64)]]) {
    const input = fixture(); Object.assign(input.platform_snapshot,
      { used_payment_keys: keys, prepared_count: '2', key_count: '2' })
    sign(input.platform_snapshot); reject(input)
  }
})
test('ten thousand keys supported; larger snapshot or combined history rejected', () => {
  const input = fixture()
  input.platform_snapshot.used_payment_keys = Array.from({ length: 10000 }, (_, i) => i.toString(16).padStart(64, '0'))
  input.platform_snapshot.prepared_count = input.platform_snapshot.key_count = '10000'
  sign(input.platform_snapshot)
  assert.equal(previewWithPlatformSnapshot(input).status, 'review_ready')
  input.reconciliation.snapshot.used_payment_keys.push('f'.repeat(64))
  reject(input, 'COMBINED_HISTORY_TOO_LARGE')
  input.reconciliation.snapshot.used_payment_keys = []
  input.platform_snapshot.used_payment_keys.push('f'.repeat(64))
  sign(input.platform_snapshot); reject(input)
})
test('unknown, missing, accessor, sparse and extended fields fail without executing code', () => {
  const extra = fixture(); extra.platform_snapshot.user_id = 'never-return'; reject(extra)
  const missing = fixture(); delete missing.platform_snapshot; reject(missing)
  const accessor = fixture()
  Object.defineProperty(accessor.platform_snapshot, 'policy_digest', { enumerable: true, get() { throw Error('not-executed') } })
  reject(accessor)
  const sparse = fixture(); sparse.platform_snapshot.used_payment_keys = Array(1); reject(sparse)
  const extended = fixture(); extended.platform_snapshot.used_payment_keys.extra = true; reject(extended)
})
test('source overlap cannot authorize services, QSHARE, or approval', () => {
  const input = fixture()
  Object.assign(input.reconciliation.rows[0], { commercial_purpose: 'quant_subscription', esk_base_units: '0', sale_batch_id: null })
  assert.equal(previewWithPlatformSnapshot(input).status, 'routed_only')
  occupied(input); reason(input, 'PAYMENT_ALREADY_USED')
})
test('missing consent/approval still blocks', () => {
  const input = fixture(); input.reconciliation.rows[0].consent_digest = null
  input.reconciliation.rows[0].approval_digest = null
  reason(input, 'CONSENT_MISSING'); reason(input, 'APPROVAL_MISSING')
})

const cli = path.join(__dirname, 'preview-esk-platform-reconciliation.js')
function runCli(input, args = []) {
  const result = spawnSync(process.execPath, [cli, ...args], { input, timeout: 5000, maxBuffer: 2 * 1048576, windowsHide: true })
  assert.equal(result.error, undefined)
  assert.equal(result.stderr.toString(), '')
  return { exit: result.status, report: JSON.parse(result.stdout.toString('utf8')) }
}
test('actual CLI stdin handles reviewable and occupied cases', () => {
  const open = runCli(JSON.stringify(fixture())); assert.equal(open.exit, 0)
  assert.equal(open.report.preview.status, 'review_ready')
  const held = runCli(JSON.stringify(occupied(fixture()))); assert.equal(held.exit, 2)
  assert(held.report.preview.rows[0].reasons.includes('PAYMENT_ALREADY_USED'))
})
test('CLI strict encoding, duplicates, size, legacy and commit arguments rejected', () => {
  for (const input of [Buffer.from([0xff]), Buffer.from([0xef, 0xbb, 0xbf, 0x7b, 0x7d]),
    '{"schema":"x","schema":"y"}', Buffer.alloc(1048577, 32), JSON.stringify(fixture().reconciliation)]) {
    const result = runCli(input); assert.equal(result.exit, 1); assert.equal(result.report.preview, null)
  }
  assert.equal(runCli(JSON.stringify(fixture()), ['--commit']).exit, 1)
})
test('recomputing a digest never turns the supplied snapshot into authenticated evidence', () => {
  const input = clone(fixture()); input.platform_snapshot.policy_digest = 'f'.repeat(64); sign(input.platform_snapshot)
  const result = previewWithPlatformSnapshot(input)
  assert.equal(result.status, 'review_ready')
  assert.equal(result.platform_snapshot_authenticity_verified, false)
})

console.log(`ESK_PLATFORM_RECONCILIATION_TESTS=${passed}_passed`)
console.log('ESK_PLATFORM_RECONCILIATION_NETWORK=none;BALANCE_WRITES=none')
