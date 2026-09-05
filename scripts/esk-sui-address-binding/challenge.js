const { randomBytes: systemRandomBytes, createHash } = require('node:crypto')
const {
  CHALLENGE_SCHEMA, validateChallengeRequest, validateChallenge, fail,
} = require('./contract')

function sha256Hex(bytes) { return createHash('sha256').update(bytes).digest('hex') }
function sha256(bytes) { return `sha256:${sha256Hex(bytes)}` }

function messageText(fields) {
  return [
    'YILONG_ESK_SUI_ADDRESS_BINDING_V1',
    `network=${fields.network}`,
    `purpose=${fields.purpose}`,
    `subject_commitment=${fields.subject_commitment}`,
    `address=${fields.address}`,
    `nonce_base64=${fields.nonce_base64}`,
    `issued_at=${fields.issued_at}`,
    `expires_at=${fields.expires_at}`,
  ].join('\n')
}

function assemble(fields) {
  const message = Buffer.from(messageText(fields), 'utf8')
  const messageHex = sha256Hex(message)
  return {
    schema: CHALLENGE_SCHEMA,
    challenge_id: `eab1_${messageHex.slice(0, 32)}`,
    network: fields.network,
    purpose: fields.purpose,
    subject_commitment: fields.subject_commitment,
    address: fields.address,
    ttl_seconds: fields.ttl_seconds,
    nonce_base64: fields.nonce_base64,
    issued_at: fields.issued_at,
    expires_at: fields.expires_at,
    message_base64: message.toString('base64'),
    message_sha256: `sha256:${messageHex}`,
  }
}

function validClock(value) {
  if (!Number.isSafeInteger(value) || value < 0 || value > 8640000000000000) fail('INVALID_INPUT')
  return value
}

function createChallenge(raw) {
  const request = validateChallengeRequest(raw)
  const nowMs = validClock(Date.now())
  const nonce = systemRandomBytes(32)
  const fields = {
    ...request,
    nonce_base64: nonce.toString('base64'),
    issued_at: new Date(nowMs).toISOString(),
    expires_at: new Date(nowMs + request.ttl_seconds * 1000).toISOString(),
  }
  const challenge = assemble(fields)
  validateChallenge(challenge)
  return challenge
}

function reconstructChallenge(raw) {
  const challenge = validateChallenge(raw)
  const rebuilt = assemble(challenge)
  if (JSON.stringify(rebuilt) !== JSON.stringify(challenge)) fail('INVALID_CHALLENGE')
  return rebuilt
}

module.exports = {
  sha256, sha256Hex, messageText, assemble, createChallenge, reconstructChallenge,
}
