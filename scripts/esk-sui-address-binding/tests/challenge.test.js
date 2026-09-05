const test = require('node:test')
const assert = require('node:assert/strict')
const { createHash } = require('node:crypto')
const { assemble, createChallenge, reconstructChallenge } = require('../challenge')
const { request } = require('./fixtures')

const ADDRESS = `0x${'c'.repeat(64)}`

test('challenge construction uses fresh system entropy and is exactly reconstructable', () => {
  const first = createChallenge(request(ADDRESS))
  const second = createChallenge(request(ADDRESS))
  assert.notEqual(first.nonce_base64, second.nonce_base64)
  assert.notEqual(first.challenge_id, second.challenge_id)
  assert.match(first.challenge_id, /^eab1_[0-9a-f]{32}$/)
  assert.equal(first.message_sha256,
    `sha256:${createHash('sha256').update(Buffer.from(first.message_base64, 'base64')).digest('hex')}`)
  assert.deepEqual(reconstructChallenge(first), first)
})

test('message uses fixed order, LF and no trailing newline', () => {
  const value = createChallenge(request(ADDRESS))
  const text = Buffer.from(value.message_base64, 'base64').toString('utf8')
  assert.deepEqual(text.split('\n').map(line => line.split('=')[0]), [
    'YILONG_ESK_SUI_ADDRESS_BINDING_V1', 'network', 'purpose',
    'subject_commitment', 'address', 'nonce_base64', 'issued_at', 'expires_at',
  ])
  assert.equal(text.includes('\r'), false)
  assert.equal(text.endsWith('\n'), false)
})

test('every bound field changes challenge identity', () => {
  const base = createChallenge(request(ADDRESS))
  const cases = [
    assemble({
      ...base,
      ttl_seconds: 601,
      expires_at: new Date(Date.parse(base.expires_at) + 1000).toISOString(),
    }),
    assemble({ ...base, address: `0x${'d'.repeat(64)}` }),
    assemble({ ...base, nonce_base64: Buffer.alloc(32, 8).toString('base64') }),
    assemble({ ...base, issued_at: new Date(Date.parse(base.issued_at) + 1).toISOString() }),
  ]
  for (const value of cases) {
    assert.notEqual(value.challenge_id, base.challenge_id)
    assert.notEqual(value.message_sha256, base.message_sha256)
  }
})
