const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { spawnSync } = require('node:child_process')
const { preview } = require('./esk-paid-reconciliation/preview')
const { canonical, fingerprint, sourceFingerprint, paymentKey } = require('./esk-paid-reconciliation/identity')
const { parseInput, parseAmount, MAX_BYTES, I64_MAX } = require('./esk-paid-reconciliation/input')

const fixturePath = path.join(__dirname, '../contracts/assets/esk-paid-reconciliation-v1.fixture.json')
const fixtureBytes = fs.readFileSync(fixturePath)
const fixture = () => JSON.parse(fixtureBytes)
const hash = character => character.repeat(64)
let cases = 0
function check(name, fn) { fn(); cases += 1 }
function boundaries(report) {
  for (const field of ['funds_moved', 'balances_written', 'commit_eligible', 'payment_authenticity_verified',
    'identity_verified', 'approvals_verified']) assert.equal(report[field], false)
}
function changed(mutate) { const input = fixture(); mutate(input); return input }
function block(name, mutate, reason) {
  check(name, () => {
    const report = preview(changed(mutate))
    assert.equal(report.status, 'needs_review', name)
    assert.ok(report.rows.some(row => row.reasons.includes(reason)), `${name}: ${JSON.stringify(report.rows)}`)
    boundaries(report)
  })
}
function invalid(name, mutate) {
  check(name, () => {
    const report = preview(changed(mutate))
    assert.equal(report.status, 'invalid_input', name)
    assert.equal(report.input_digest, null)
    assert.equal(report.rows.length, 0)
    assert.ok(!JSON.stringify(report).includes('secret'))
    boundaries(report)
  })
}

check('review ready is not payment validation or credit', () => {
  const input = fixture()
  const before = JSON.stringify(input)
  const report = preview(input)
  assert.equal(JSON.stringify(input), before)
  assert.equal(report.status, 'review_ready')
  assert.equal(report.rows[0].payment_base_units, '20000000')
  assert.equal(report.proposed_totals[0].esk_base_units, '10000000')
  assert.equal(report.report_digest, fingerprint({ ...report, report_digest: null }))
  assert.deepEqual(preview(input), report)
  assert.ok(!JSON.stringify(report).includes(input.rows[0].external_payment_reference))
  assert.ok(!JSON.stringify(report).includes(input.source.namespace))
  assert.ok(!JSON.stringify(report).includes(input.batch_id))
  assert.ok(!('entries' in report) && !('user_id' in report.rows[0]))
  boundaries(report)
})
check('canonical object order and exact pricing are deterministic', () => {
  assert.equal(canonical({ z: 2, a: 1 }), canonical({ a: 1, z: 2 }))
  const input = fixture()
  input.sale_batches[0].payment_base_units_per_lot = '1000000'
  input.rows[0].esk_base_units = '20000000'
  assert.equal(preview(input).status, 'review_ready')
})
check('stable payment identity excludes batch, row, user, amount and terms', () => {
  const input = fixture()
  const row = { ...input.rows[0], row_id: 'other', opaque_subject: hash('9'), payment_amount: '5',
    esk_base_units: '2500000', sale_batch_id: 'other' }
  assert.equal(paymentKey(input.source, row), paymentKey(input.source, input.rows[0]))
  row.external_payment_reference = '0X' + row.external_payment_reference.toUpperCase()
  assert.equal(paymentKey(input.source, row), paymentKey(input.source, input.rows[0]))
})
check('hex asset address aliases normalize, opaque payment refs retain case', () => {
  const input = fixture()
  const a = { ...input.source, asset_reference: '0xAb' }
  const b = { ...input.source, asset_reference: '0x' + '0'.repeat(62) + 'ab' }
  assert.equal(sourceFingerprint(a), sourceFingerprint(b))
  const source = { ...input.source, reference_format: 'opaque' }
  assert.notEqual(paymentKey(source, { ...input.rows[0], external_payment_reference: 'Credit-A' }),
    paymentKey(source, { ...input.rows[0], external_payment_reference: 'credit-a' }))
})
check('same transaction different event stays separate', () => {
  const input = changed(i => i.rows.push({ ...i.rows[0], row_id: 'second', transfer_index: 1 }))
  const report = preview(input)
  assert.equal(report.counts.review_ready, 2)
  assert.equal(report.proposed_totals[0].esk_base_units, '20000000')
  assert.notEqual(report.rows[0].payment_key, report.rows[1].payment_key)
})
block('same payment with other row ID conflicts', i => i.rows.push({ ...i.rows[0], row_id: 'another' }), 'DUPLICATE_BATCH_PAYMENT')
block('case/0x cannot hide duplicate', i => i.rows.push({ ...i.rows[0], row_id: 'another',
  external_payment_reference: '0X' + i.rows[0].external_payment_reference.toUpperCase() }), 'DUPLICATE_BATCH_PAYMENT')
block('different subject and amount cannot hide duplicate', i => {
  i.users.push({ opaque_subject: hash('6'), target_user_ref: hash('7') })
  i.rows.push({ ...i.rows[0], row_id: 'another', opaque_subject: hash('6'), payment_amount: '2', esk_base_units: '1000000' })
}, 'DUPLICATE_BATCH_PAYMENT')
block('changed batch cannot replay historical payment', i => {
  i.snapshot.used_payment_keys.push(paymentKey(i.source, i.rows[0]))
  i.batch_id = 'new-batch'
}, 'PAYMENT_ALREADY_USED')
block('used payment cannot move to a different purpose', i => {
  i.snapshot.used_payment_keys.push(paymentKey(i.source, i.rows[0]))
  Object.assign(i.rows[0], { commercial_purpose: 'service_purchase', esk_base_units: '0', sale_batch_id: null })
}, 'PAYMENT_ALREADY_USED')
block('asset address aliases still match history', i => {
  i.source.asset_reference = '0xAb'
  i.snapshot.source_fingerprint = sourceFingerprint(i.source)
  i.snapshot.used_payment_keys.push(paymentKey({ ...i.source,
    asset_reference: '0x' + '0'.repeat(62) + 'ab' }, i.rows[0]))
}, 'PAYMENT_ALREADY_USED')
block('cross-purpose duplicate blocks both rows', i => {
  i.rows.push({ ...i.rows[0], row_id: 'service-row', commercial_purpose: 'service_purchase',
    esk_base_units: '0', sale_batch_id: null })
}, 'DUPLICATE_BATCH_PAYMENT')
block('duplicate row identifiers', i => i.rows.push({ ...i.rows[0], transfer_index: 1 }), 'DUPLICATE_ROW_ID')
block('missing mapping', i => { i.users = [] }, 'SUBJECT_MAPPING_MISSING')
block('duplicate subject', i => { i.users.push({ ...i.users[0] }) }, 'SUBJECT_MAPPING_AMBIGUOUS')
block('shared target', i => { i.users.push({ ...i.users[0], opaque_subject: hash('6') }) }, 'TARGET_MAPPING_AMBIGUOUS')
block('pending payment', i => { i.rows[0].payment_status = 'pending' }, 'PAYMENT_PENDING')
block('reversed payment', i => { i.rows[0].payment_status = 'reversed' }, 'PAYMENT_REVERSED')
block('missing consent', i => { i.rows[0].consent_digest = null }, 'CONSENT_MISSING')
block('missing approval', i => { i.rows[0].approval_digest = null }, 'APPROVAL_MISSING')
block('unconfirmed purpose', i => { i.rows[0].commercial_purpose = 'unconfirmed' }, 'PURPOSE_UNCONFIRMED')
block('missing sale terms', i => { i.rows[0].sale_batch_id = null }, 'SALE_BATCH_MISSING')
block('ambiguous sale terms', i => { i.sale_batches.push({ ...i.sale_batches[0] }) }, 'SALE_BATCH_AMBIGUOUS')
block('different disclosure', i => { i.rows[0].disclosure_revision = 'old' }, 'DISCLOSURE_MISMATCH')
block('wrong quote', i => { i.rows[0].esk_base_units = '1' }, 'ESK_QUOTE_MISMATCH')
block('no automatic rounding', i => { i.sale_batches[0].payment_base_units_per_lot = '3' }, 'NON_INTEGRAL_ESK_QUOTE')
block('no empty ESK allocation', i => { i.rows[0].esk_base_units = '0' }, 'ESK_AMOUNT_REQUIRED')
block('quote overflow', i => { i.rows[0].payment_amount = (I64_MAX * 4n).toString() }, 'QUOTE_OVERFLOW')
block('per user proposed sum overflow', i => {
  i.source.decimals = 0
  i.snapshot.source_fingerprint = sourceFingerprint(i.source)
  i.sale_batches[0].payment_base_units_per_lot = '1'
  i.sale_batches[0].esk_base_units_per_lot = '1'
  i.rows[0].payment_amount = I64_MAX.toString()
  i.rows[0].esk_base_units = I64_MAX.toString()
  i.rows.push({ ...i.rows[0], row_id: 'second', transfer_index: 1, payment_amount: '1', esk_base_units: '1' })
}, 'USER_TOTAL_OVERFLOW')
for (const purpose of ['service_purchase', 'quant_subscription']) check(`route ${purpose} without ESK`, () => {
  const input = changed(i => Object.assign(i.rows[0], { commercial_purpose: purpose,
    esk_base_units: '0', sale_batch_id: null, disclosure_revision: null }))
  const report = preview(input)
  assert.equal(report.status, 'routed_only')
  assert.equal(report.proposed_totals.length, 0)
  assert.equal(report.rows[0].route, purpose === 'service_purchase' ? 'service_orders' : 'qshare_subscription')
  boundaries(report)
})
block('one payment cannot be service and ESK', i => { i.rows[0].commercial_purpose = 'service_purchase' }, 'NON_ESK_ALLOCATION_FORBIDDEN')
block('snapshot different source', i => { i.source.network = 'other-network' }, 'SNAPSHOT_SOURCE_MISMATCH')
block('incomplete history', i => { i.snapshot.history_complete = false }, 'HISTORY_INCOMPLETE')
block('history duplicates', i => { i.snapshot.used_payment_keys = [hash('8'), hash('8')] }, 'HISTORY_DUPLICATE_KEYS')
block('stale snapshot', i => { i.snapshot.observed_at = '2026-09-03T05:59:59.999Z' }, 'SNAPSHOT_STALE')
block('future snapshot', i => { i.snapshot.observed_at = '2026-09-04T06:00:00.001Z' }, 'SNAPSHOT_FROM_FUTURE')
check('snapshot exactly 24 hours and partial review isolation', () => {
  const input = changed(i => { i.snapshot.observed_at = '2026-09-03T06:00:00.000Z' })
  assert.equal(preview(input).status, 'review_ready')
  input.rows.push({ ...input.rows[0], row_id: 'second', transfer_index: 1, consent_digest: null })
  const report = preview(input)
  assert.equal(report.status, 'needs_review')
  assert.equal(report.counts.blocked, 1)
  assert.equal(report.proposed_totals[0].row_count, 1)
})
for (const amount of ['0', '-1', '+1', '01', '1e2', 'NaN', '1.0000001', '1.', '.1', ' 1', '1 ', '9'.repeat(40)]) {
  invalid(`invalid payment ${amount}`, i => { i.rows[0].payment_amount = amount })
}
for (const [name, mutate] of [
  ['unknown secret', i => { i.secret = 'secret' }],
  ['nested secret', i => { i.rows[0].secret = 'secret' }],
  ['line terminator alias', i => { i.rows[0].row_id += '\n' }],
  ['missing field', i => { delete i.rows[0].consent_digest }],
  ['unsupported schema', i => { i.schema = 'v0' }],
  ['wrong asset', i => { i.source.asset_symbol = 'ESK' }],
  ['invalid decimals', i => { i.source.decimals = 19 }],
  ['float amount', i => { i.rows[0].payment_amount = 20 }],
  ['ESK overflow', i => { i.rows[0].esk_base_units = (I64_MAX + 1n).toString() }],
  ['ESK negative', i => { i.rows[0].esk_base_units = '-1' }],
  ['invalid timestamp', i => { i.as_of = '2026-02-31T06:00:00.000Z' }],
  ['null row', i => { i.rows = [null] }],
  ['empty rows', i => { i.rows = [] }],
  ['too many rows', i => { i.rows = Array(1001).fill(i.rows[0]) }],
  ['too many users', i => { i.users = Array(1001).fill(i.users[0]) }],
  ['too much history', i => { i.snapshot.used_payment_keys = Array(10001).fill(hash('1')) }],
  ['unsafe event index', i => { i.rows[0].transfer_index = 1e20 }],
  ['bad hex reference', i => { i.rows[0].external_payment_reference = 'secret' }],
]) invalid(name, mutate)
check('exact amounts retain base-unit precision', () => {
  assert.equal(parseAmount('0.000001', 6), 1n)
  assert.equal(parseAmount('9007199254.740993', 6), 9007199254740993n)
  assert.equal(parseAmount('0.000000000000000001', 18), 1n)
})
for (const [name, bytes] of [
  ['duplicate key', Buffer.from('{"schema":1,"schema":2}')],
  ['escaped duplicate key', Buffer.from('{"schema":1,"sch\\u0065ma":2}')],
  ['nested duplicate key', Buffer.from('{"a":{"b":1,"b":2}}')],
  ['invalid UTF8', Buffer.from([0xc0, 0xaf])],
  ['BOM', Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), fixtureBytes])],
  ['oversized input', Buffer.alloc(MAX_BYTES + 1, 0x20)],
  ['deep content', Buffer.from('['.repeat(14) + '0' + ']'.repeat(14))],
  ['unsafe JSON integer', Buffer.from('{"x":9007199254740993}')],
  ['trailing garbage', Buffer.from('{}secret')],
  ['invalid numeric syntax', Buffer.from('{"x":NaN}')],
]) check(name, () => {
  assert.throws(() => parseInput(bytes), error => !error.message.includes('secret') && /^[A-Z0-9_]+$/.test(error.code))
})
check('CLI stdin output is deterministic, redacted and non-submittable', () => {
  const cli = path.join(__dirname, 'preview-esk-paid-reconciliation.js')
  const run = (input, args = []) => spawnSync(process.execPath, [cli, ...args], { input, encoding: 'utf8', timeout: 5000 })
  const valid = run(fixtureBytes)
  assert.equal(valid.status, 0, valid.stderr)
  assert.deepEqual(JSON.parse(valid.stdout), preview(parseInput(fixtureBytes)))
  const missing = changed(i => { i.rows[0].approval_digest = null })
  assert.equal(run(JSON.stringify(missing)).status, 2)
  const invalid = run('{"secret":"secret"}')
  assert.equal(invalid.status, 1)
  assert.ok(!invalid.stdout.includes('secret') && !invalid.stderr.includes('secret'))
  assert.equal(run(fixtureBytes, ['--commit']).status, 1)
  assert.equal(run(null, ['--help']).status, 0)
  assert.deepEqual(fs.readFileSync(fixturePath), fixtureBytes)
})
console.log(`ESK_PAID_RECONCILIATION_TESTS=${cases}_passed`)
console.log('ESK_PAID_RECONCILIATION_BALANCE_WRITES=none')
console.log('ESK_PAID_RECONCILIATION_REAL_PAYMENT_VERIFICATION=not_performed')
