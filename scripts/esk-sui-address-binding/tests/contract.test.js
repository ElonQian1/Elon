const test = require('node:test')
const assert = require('node:assert/strict')
const {
  validateChallengeRequest, validateChallenge, validateWalletResponse,
  AddressBindingError, canonicalBase64, safeCode,
} = require('../contract')
const { createChallenge } = require('../challenge')
const { SUBJECT, request } = require('./fixtures')

const ADDRESS = `0x${'b'.repeat(64)}`

test('strict request accepts the only supported canonical testnet migration shape', () => {
  const value = validateChallengeRequest(request(ADDRESS))
  assert.equal(value.address, ADDRESS)
  assert.equal(value.subject_commitment, SUBJECT)
  assert.equal(value.ttl_seconds, 600)
  for (const ttl_seconds of [120, 900]) {
    assert.equal(validateChallengeRequest({ ...request(ADDRESS), ttl_seconds }).ttl_seconds,
      ttl_seconds)
  }
})

for (const [field, values] of [
  ['schema', ['', 'v2', null]],
  ['network', ['mainnet', 'devnet', '', null]],
  ['purpose', ['governance_role', 'service_payment', '', null]],
  ['subject_commitment', [`sha256:${'0'.repeat(64)}`, 'a'.repeat(64), 'sha256:AA', null]],
  ['address', ['0x0', `0x${'0'.repeat(64)}`, `0X${'b'.repeat(64)}`,
    `0x${'B'.repeat(64)}`, `0x${'g'.repeat(64)}`, null]],
  ['ttl_seconds', [119, 901, 600.5, '600', null]],
]) {
  values.forEach((value, index) => test(`rejects invalid request ${field} ${index + 1}`, () => {
    assert.throws(() => validateChallengeRequest({ ...request(ADDRESS), [field]: value }))
  }))
}

test('requests reject missing and unknown fields', () => {
  for (const field of Object.keys(request(ADDRESS))) {
    const input = request(ADDRESS)
    delete input[field]
    assert.throws(() => validateChallengeRequest(input))
  }
  assert.throws(() => validateChallengeRequest({ ...request(ADDRESS), user_id: 'private' }))
  for (const input of [null, [], 'secret', 7]) assert.throws(() => validateChallengeRequest(input))
})

test('challenge and wallet response reject all unknown or drifted fields', () => {
  const challenge = createChallenge(request(ADDRESS))
  assert.equal(validateChallenge(challenge).challenge_id, challenge.challenge_id)
  assert.throws(() => validateChallenge({ ...challenge, token: 'secret' }))
  for (const field of Object.keys(challenge)) {
    const input = { ...challenge }
    delete input[field]
    assert.throws(() => validateChallenge(input))
  }
  const response = {
    schema: 'yilong.esk.sui.address_binding_wallet_response.v1',
    challenge_id: challenge.challenge_id,
    message_base64: challenge.message_base64,
    signature: Buffer.from([0, ...Buffer.alloc(96)]).toString('base64'),
  }
  assert.equal(validateWalletResponse(response).challenge_id, challenge.challenge_id)
  assert.throws(() => validateWalletResponse({ ...response, mnemonic: 'never' }))
  for (const field of Object.keys(response)) {
    const input = { ...response }
    delete input[field]
    assert.throws(() => validateWalletResponse(input))
  }
})

test('canonical base64 rejects whitespace, padding drift and oversized values', () => {
  assert.deepEqual(canonicalBase64('AQID', 3, 3), Buffer.from([1, 2, 3]))
  for (const value of [' AQID', 'AQID\n', 'AQI', 'AQID=', '***', '']) {
    assert.throws(() => canonicalBase64(value, 1, 8))
  }
  assert.throws(() => canonicalBase64(Buffer.alloc(33).toString('base64'), 1, 32))
})

test('safe errors expose only fixed codes', () => {
  assert.equal(safeCode(new AddressBindingError('MESSAGE_MISMATCH')), 'MESSAGE_MISMATCH')
  assert.equal(safeCode(new AddressBindingError('secret')), 'INVALID_INPUT')
  assert.equal(safeCode(new Error('secret')), 'INTERNAL_ERROR')
})

test('challenge time fields remain strict UTC millisecond timestamps', () => {
  const before = Date.now()
  const challenge = createChallenge(request(ADDRESS))
  const after = Date.now()
  assert.ok(Date.parse(challenge.issued_at) >= before)
  assert.ok(Date.parse(challenge.issued_at) <= after)
  assert.equal(Date.parse(challenge.expires_at) - Date.parse(challenge.issued_at), 600000)
  for (const issued_at of ['2026-09-05T08:00:00Z', '2026-09-05 08:00:00.000Z',
    '2026-02-30T08:00:00.000Z']) {
    assert.throws(() => validateChallenge({ ...challenge, issued_at }))
  }
})
