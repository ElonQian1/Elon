const { createHash, timingSafeEqual } = require('node:crypto')
const { isDeepStrictEqual } = require('node:util')
const {
  EVIDENCE_SCHEMA, canonicalBase64, validateWalletResponse, validateEvidenceShape,
  strictTimestamp, fail,
} = require('./contract')
const { reconstructChallenge } = require('./challenge')

const SCHEMES = new Map([[0, 'ed25519'], [1, 'secp256k1'], [2, 'secp256r1']])

function sha256(bytes) { return `sha256:${createHash('sha256').update(bytes).digest('hex')}` }

function canonicalJson(value) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean' ||
      typeof value === 'number') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  if (typeof value !== 'object') fail('INVALID_EVIDENCE')
  return `{${Object.keys(value).sort().map(key =>
    `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`
}

async function sdkVerify(message, signature, address) {
  let verifyPersonalMessageSignature
  try { ({ verifyPersonalMessageSignature } = await import('@mysten/sui/verify')) } catch {
    fail('SDK_UNAVAILABLE')
  }
  try {
    const publicKey = await verifyPersonalMessageSignature(message, signature, { address })
    if (!publicKey || typeof publicKey.verifyAddress !== 'function' ||
        !publicKey.verifyAddress(address)) fail('SIGNATURE_INVALID')
    return publicKey
  } catch (error) {
    if (error && error.code === 'SIGNATURE_INVALID') throw error
    fail('SIGNATURE_INVALID')
  }
}

function timeForVerification(value) {
  if (!Number.isSafeInteger(value) || value < 0 || value > 8640000000000000) fail('INVALID_INPUT')
  return value
}

async function verifyAddressControlAt(challengeRaw, responseRaw, nowMs) {
  const challenge = reconstructChallenge(challengeRaw)
  const response = validateWalletResponse(responseRaw)
  if (response.challenge_id !== challenge.challenge_id) fail('CHALLENGE_ID_MISMATCH')
  const expectedMessage = canonicalBase64(challenge.message_base64, 1, 2048, 'INVALID_CHALLENGE')
  const actualMessage = canonicalBase64(response.message_base64, 1, 2048, 'INVALID_RESPONSE')
  if (expectedMessage.length !== actualMessage.length ||
      !timingSafeEqual(expectedMessage, actualMessage)) fail('MESSAGE_MISMATCH')
  timeForVerification(nowMs)
  const issuedMs = strictTimestamp(challenge.issued_at, 'INVALID_CHALLENGE')
  const expiresMs = strictTimestamp(challenge.expires_at, 'INVALID_CHALLENGE')
  if (nowMs < issuedMs) fail('CHALLENGE_NOT_YET_VALID')
  if (nowMs >= expiresMs) fail('CHALLENGE_EXPIRED')
  const signatureBytes = canonicalBase64(response.signature, 2, 2048, 'INVALID_RESPONSE')
  const scheme = SCHEMES.get(signatureBytes[0])
  if (!scheme) fail('UNSUPPORTED_SIGNATURE_SCHEME')
  await sdkVerify(expectedMessage, response.signature, challenge.address)
  const unsigned = {
    schema: EVIDENCE_SCHEMA,
    status: 'verified_candidate',
    challenge,
    wallet_response: response,
    address: challenge.address,
    subject_commitment: challenge.subject_commitment,
    signature_scheme: scheme,
    message_sha256: sha256(expectedMessage),
    signature_sha256: sha256(signatureBytes),
    verified_at: new Date(nowMs).toISOString(),
    address_control_verified: true,
    platform_subject_authenticated: false,
    challenge_single_use_recorded: false,
    chain_finality_verified: false,
    asset_identity_verified: false,
    balance_eligible: false,
    manifest_transition_allowed: false,
  }
  const evidence = { ...unsigned, evidence_sha256: sha256(Buffer.from(canonicalJson(unsigned), 'utf8')) }
  validateEvidenceShape(evidence)
  return evidence
}

async function verifyAddressControl(challengeRaw, responseRaw) {
  return verifyAddressControlAt(challengeRaw, responseRaw, Date.now())
}

async function reverifyEvidence(raw) {
  const evidence = validateEvidenceShape(raw)
  const { evidence_sha256: expectedDigest, ...unsigned } = evidence
  if (sha256(Buffer.from(canonicalJson(unsigned), 'utf8')) !== expectedDigest) fail('INVALID_EVIDENCE')
  const verifiedMs = strictTimestamp(evidence.verified_at, 'INVALID_EVIDENCE')
  const issuedMs = strictTimestamp(evidence.challenge.issued_at, 'INVALID_EVIDENCE')
  const expiresMs = strictTimestamp(evidence.challenge.expires_at, 'INVALID_EVIDENCE')
  if (verifiedMs < issuedMs || verifiedMs >= expiresMs) fail('INVALID_EVIDENCE')
  const rebuilt = await verifyAddressControlAt(
    evidence.challenge, evidence.wallet_response, Date.now())
  const { verified_at: ignoredStoredTime, evidence_sha256: ignoredStoredDigest,
    ...storedStable } = evidence
  const { verified_at: ignoredFreshTime, evidence_sha256: ignoredFreshDigest,
    ...freshStable } = rebuilt
  if (!isDeepStrictEqual(freshStable, storedStable)) fail('INVALID_EVIDENCE')
  return evidence
}

module.exports = { SCHEMES, canonicalJson, verifyAddressControl, reverifyEvidence }
