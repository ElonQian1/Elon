const REQUEST_SCHEMA = 'yilong.esk.sui.address_binding_challenge_request.v1'
const CHALLENGE_SCHEMA = 'yilong.esk.sui.address_binding_challenge.v1'
const RESPONSE_SCHEMA = 'yilong.esk.sui.address_binding_wallet_response.v1'
const EVIDENCE_SCHEMA = 'yilong.esk.sui.address_control_evidence.v1'
const MAX_FILE_BYTES = 64 * 1024
const MIN_TTL_SECONDS = 120
const MAX_TTL_SECONDS = 900
const MAX_MESSAGE_BYTES = 2048
const MAX_SIGNATURE_BYTES = 2048

const SAFE_CODES = new Set([
  'INVALID_INPUT', 'INVALID_CHALLENGE', 'INVALID_RESPONSE', 'INVALID_EVIDENCE',
  'MESSAGE_MISMATCH', 'CHALLENGE_ID_MISMATCH', 'CHALLENGE_EXPIRED',
  'CHALLENGE_NOT_YET_VALID', 'UNSUPPORTED_SIGNATURE_SCHEME',
  'SIGNATURE_INVALID', 'SDK_UNAVAILABLE', 'FILE_TOO_LARGE', 'USAGE',
])

class AddressBindingError extends Error {
  constructor(code) {
    super(code)
    this.name = 'AddressBindingError'
    this.code = SAFE_CODES.has(code) ? code : 'INVALID_INPUT'
  }
}

function fail(code) { throw new AddressBindingError(code) }

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function exactKeys(value, expected, code) {
  if (!isPlainObject(value)) fail(code)
  const actual = Object.keys(value).sort()
  const wanted = [...expected].sort()
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(code)
  }
}

function literal(value, expected, code) {
  if (value !== expected) fail(code)
  return value
}

function subjectCommitment(value, code = 'INVALID_INPUT') {
  if (typeof value !== 'string' || !/^sha256:[0-9a-f]{64}$/.test(value) ||
      value === `sha256:${'0'.repeat(64)}`) fail(code)
  return value
}

function address(value, { canonical = false, code = 'INVALID_INPUT' } = {}) {
  if (typeof value !== 'string' || !/^0x[0-9a-fA-F]{64}$/.test(value)) fail(code)
  const normalized = value.toLowerCase()
  if (normalized === `0x${'0'.repeat(64)}` || (canonical && value !== normalized)) fail(code)
  return normalized
}

function nativeInteger(value, minimum, maximum, code) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) fail(code)
  return value
}

function strictTimestamp(value, code) {
  if (typeof value !== 'string' ||
      !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value)) fail(code)
  const milliseconds = Date.parse(value)
  if (!Number.isFinite(milliseconds) || new Date(milliseconds).toISOString() !== value) fail(code)
  return milliseconds
}

function canonicalBase64(value, minimumBytes, maximumBytes, code = 'INVALID_INPUT') {
  if (typeof value !== 'string' || value.length === 0 || value.length > maximumBytes * 2 ||
      value.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(value)) fail(code)
  let bytes
  try { bytes = Buffer.from(value, 'base64') } catch { fail(code) }
  if (bytes.length < minimumBytes || bytes.length > maximumBytes ||
      bytes.toString('base64') !== value) fail(code)
  return bytes
}

function digest(value, code) {
  if (typeof value !== 'string' || !/^sha256:[0-9a-f]{64}$/.test(value)) fail(code)
  return value
}

function challengeId(value, code) {
  if (typeof value !== 'string' || !/^eab1_[0-9a-f]{32}$/.test(value)) fail(code)
  return value
}

function validateChallengeRequest(value) {
  const code = 'INVALID_INPUT'
  exactKeys(value, ['schema', 'network', 'purpose', 'subject_commitment', 'address',
    'ttl_seconds'], code)
  return {
    schema: literal(value.schema, REQUEST_SCHEMA, code),
    network: literal(value.network, 'testnet', code),
    purpose: literal(value.purpose, 'user_asset_migration', code),
    subject_commitment: subjectCommitment(value.subject_commitment, code),
    address: address(value.address, { canonical: true, code }),
    ttl_seconds: nativeInteger(value.ttl_seconds, MIN_TTL_SECONDS, MAX_TTL_SECONDS, code),
  }
}

function validateChallenge(value) {
  const code = 'INVALID_CHALLENGE'
  exactKeys(value, ['schema', 'challenge_id', 'network', 'purpose', 'subject_commitment',
    'address', 'ttl_seconds', 'nonce_base64', 'issued_at', 'expires_at',
    'message_base64', 'message_sha256'], code)
  const issuedMs = strictTimestamp(value.issued_at, code)
  const expiresMs = strictTimestamp(value.expires_at, code)
  const ttl = nativeInteger(value.ttl_seconds, MIN_TTL_SECONDS, MAX_TTL_SECONDS, code)
  if (expiresMs - issuedMs !== ttl * 1000) fail(code)
  canonicalBase64(value.nonce_base64, 32, 32, code)
  canonicalBase64(value.message_base64, 1, MAX_MESSAGE_BYTES, code)
  return {
    schema: literal(value.schema, CHALLENGE_SCHEMA, code),
    challenge_id: challengeId(value.challenge_id, code),
    network: literal(value.network, 'testnet', code),
    purpose: literal(value.purpose, 'user_asset_migration', code),
    subject_commitment: subjectCommitment(value.subject_commitment, code),
    address: address(value.address, { canonical: true, code }),
    ttl_seconds: ttl,
    nonce_base64: value.nonce_base64,
    issued_at: value.issued_at,
    expires_at: value.expires_at,
    message_base64: value.message_base64,
    message_sha256: digest(value.message_sha256, code),
  }
}

function validateWalletResponse(value) {
  const code = 'INVALID_RESPONSE'
  exactKeys(value, ['schema', 'challenge_id', 'message_base64', 'signature'], code)
  canonicalBase64(value.message_base64, 1, MAX_MESSAGE_BYTES, code)
  canonicalBase64(value.signature, 2, MAX_SIGNATURE_BYTES, code)
  return {
    schema: literal(value.schema, RESPONSE_SCHEMA, code),
    challenge_id: challengeId(value.challenge_id, code),
    message_base64: value.message_base64,
    signature: value.signature,
  }
}

function validateEvidenceShape(value) {
  const code = 'INVALID_EVIDENCE'
  exactKeys(value, ['schema', 'status', 'challenge', 'wallet_response', 'address',
    'subject_commitment', 'signature_scheme', 'message_sha256', 'signature_sha256',
    'verified_at', 'address_control_verified', 'platform_subject_authenticated',
    'challenge_single_use_recorded', 'chain_finality_verified', 'asset_identity_verified',
    'balance_eligible', 'manifest_transition_allowed', 'evidence_sha256'], code)
  const schemes = new Set(['ed25519', 'secp256k1', 'secp256r1'])
  if (!schemes.has(value.signature_scheme)) fail(code)
  if (value.schema !== EVIDENCE_SCHEMA || value.status !== 'verified_candidate' ||
      value.address_control_verified !== true) fail(code)
  for (const key of ['platform_subject_authenticated', 'challenge_single_use_recorded',
    'chain_finality_verified', 'asset_identity_verified', 'balance_eligible',
    'manifest_transition_allowed']) if (value[key] !== false) fail(code)
  validateChallenge(value.challenge)
  validateWalletResponse(value.wallet_response)
  address(value.address, { canonical: true, code })
  subjectCommitment(value.subject_commitment, code)
  digest(value.message_sha256, code)
  digest(value.signature_sha256, code)
  strictTimestamp(value.verified_at, code)
  digest(value.evidence_sha256, code)
  return value
}

function safeCode(error) {
  if (error instanceof AddressBindingError) return error.code
  return 'INTERNAL_ERROR'
}

module.exports = {
  REQUEST_SCHEMA, CHALLENGE_SCHEMA, RESPONSE_SCHEMA, EVIDENCE_SCHEMA,
  MAX_FILE_BYTES, MIN_TTL_SECONDS, MAX_TTL_SECONDS, MAX_MESSAGE_BYTES,
  MAX_SIGNATURE_BYTES, AddressBindingError, fail, exactKeys, literal,
  subjectCommitment, address, nativeInteger, strictTimestamp, canonicalBase64,
  digest, challengeId, validateChallengeRequest, validateChallenge,
  validateWalletResponse, validateEvidenceShape, safeCode,
}
