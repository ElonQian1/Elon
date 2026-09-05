const test = require('node:test')
const assert = require('node:assert/strict')
require('./no-network-guard')
const { webcrypto } = require('node:crypto')
const { assemble } = require('../challenge')
const { canonicalJson } = require('../verify')

async function webSha256(bytes) {
  const digest = await webcrypto.subtle.digest('SHA-256', bytes)
  return Buffer.from(digest).toString('hex')
}

test('challenge ID and message digest match the fixed cross-implementation vector', async () => {
  const challenge = assemble({
    network: 'testnet',
    purpose: 'user_asset_migration',
    subject_commitment: `sha256:${'a'.repeat(64)}`,
    address: `0x${'b'.repeat(64)}`,
    ttl_seconds: 600,
    nonce_base64: 'BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=',
    issued_at: '2026-09-05T08:00:00.000Z',
    expires_at: '2026-09-05T08:10:00.000Z',
  })
  assert.equal(challenge.challenge_id, 'eab1_fcdc075cbdf1d5b7f484161218766d57')
  assert.equal(challenge.message_sha256,
    'sha256:fcdc075cbdf1d5b7f484161218766d57efd0604697e53ebfb0f075e0a4c9d4ce')
  const expectedMessage = [
    'YILONG_ESK_SUI_ADDRESS_BINDING_V1',
    'network=testnet',
    'purpose=user_asset_migration',
    `subject_commitment=sha256:${'a'.repeat(64)}`,
    `address=0x${'b'.repeat(64)}`,
    'nonce_base64=BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=',
    'issued_at=2026-09-05T08:00:00.000Z',
    'expires_at=2026-09-05T08:10:00.000Z',
  ].join('\n')
  assert.equal(Buffer.from(challenge.message_base64, 'base64').toString('utf8'), expectedMessage)
  assert.equal(await webSha256(Buffer.from(challenge.message_base64, 'base64')),
    challenge.message_sha256.slice('sha256:'.length))
})

test('canonical evidence JSON ordering and digest match the fixed vector', async () => {
  const input = { z: [3, { b: true, a: null }], a: 'ESK', m: { y: 2, x: 'testnet' } }
  const expected = '{"a":"ESK","m":{"x":"testnet","y":2},"z":[3,{"a":null,"b":true}]}'
  assert.equal(canonicalJson(input), expected)
  assert.equal(await webSha256(Buffer.from(expected, 'utf8')),
    '37d91eff51091d142eb476b0e20ac523a67080e0efd0e57cb390e3e5e30c5d1f')
})
