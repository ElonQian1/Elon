import {
  createHash,
  createPrivateKey,
  createPublicKey,
  createSign,
  createVerify,
} from 'node:crypto'
import { decodeStrictBase64 } from './strict-base64.js'

export const CONSUMER_PORTABILITY_SIGNATURE_SCHEMA =
  'open_commerce.consumer_portability_signed_package.v1'
export const CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM = 'rsa-pkcs1v15-sha256'
const SIGNATURE_PROTOCOL = 'open_commerce.consumer_portability_signature.v1'

export function consumerPortabilityPublicKeyId(publicKey) {
  const key = publicKey?.type === 'public' ? publicKey : createPublicKey(publicKey)
  assertRsaKey(key, 'publicKey')
  const der = key.export({ type: 'spki', format: 'der' })
  return createHash('sha256').update(der).digest('hex')
}

export function consumerPortabilitySignatureMessage({ sourceOperator, keyId, package: value }) {
  assertSourceOperator(sourceOperator)
  assertKeyId(keyId)
  assertPackage(value)
  return [
    SIGNATURE_PROTOCOL,
    sourceOperator,
    keyId,
    value.schema,
    value.id,
    value.source_project_id,
    value.idempotency_key,
    value.payload_sha256,
    value.created_at,
  ].join('\n')
}

export function signConsumerPortabilityPackage({ sourceOperator, privateKey, package: value }) {
  const privateKeyObject = createPrivateKey(privateKey)
  assertRsaKey(privateKeyObject, 'privateKey')
  const publicKey = createPublicKey(privateKeyObject)
  const keyId = consumerPortabilityPublicKeyId(publicKey)
  const message = consumerPortabilitySignatureMessage({ sourceOperator, keyId, package: value })
  const signer = createSign('RSA-SHA256')
  signer.update(message, 'utf8')
  signer.end()
  return {
    schema: CONSUMER_PORTABILITY_SIGNATURE_SCHEMA,
    source_operator: sourceOperator,
    package: value,
    signature: {
      algorithm: CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM,
      key_id: keyId,
      signature_base64: signer.sign(privateKeyObject).toString('base64'),
    },
  }
}

export function verifyConsumerPortabilityPackageSignature({ publicKey, signedPackage }) {
  try {
    if (signedPackage?.schema !== CONSUMER_PORTABILITY_SIGNATURE_SCHEMA) return false
    if (signedPackage.signature?.algorithm !== CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM) return false
    const key = createPublicKey(publicKey)
    const modulusLength = assertRsaKey(key, 'publicKey')
    const expectedKeyId = consumerPortabilityPublicKeyId(key)
    if (signedPackage.signature.key_id !== expectedKeyId) return false
    const message = consumerPortabilitySignatureMessage({
      sourceOperator: signedPackage.source_operator,
      keyId: expectedKeyId,
      package: signedPackage.package,
    })
    const signature = decodeStrictBase64(signedPackage.signature.signature_base64, {
      label: 'signature.signature_base64',
      minBytes: 256,
      maxBytes: 1024,
    })
    if (signature.length !== modulusLength / 8) return false
    const verifier = createVerify('RSA-SHA256')
    verifier.update(message, 'utf8')
    verifier.end()
    return verifier.verify(key, signature)
  } catch {
    return false
  }
}

function assertSourceOperator(value) {
  if (
    typeof value !== 'string'
    || value.length < 1
    || value.length > 160
    || value.trim() !== value
    || /[\u0000-\u001f\u007f]/.test(value)
  ) {
    throw new TypeError('sourceOperator must be a trimmed 1-160 character label')
  }
}

function assertKeyId(value) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    throw new TypeError('keyId must be a lowercase SHA-256 digest')
  }
}

function assertPackage(value) {
  if (!value || typeof value !== 'object') throw new TypeError('package is required')
  assertBoundText(value.schema, 'package.schema', 1, 160)
  assertBoundText(value.id, 'package.id', 1, 120)
  assertBoundText(value.source_project_id, 'package.source_project_id', 1, 120)
  assertBoundText(value.idempotency_key, 'package.idempotency_key', 8, 120)
  assertBoundText(value.created_at, 'package.created_at', 1, 64)
  if (!/^[A-Za-z0-9._:-]+$/.test(value.idempotency_key)) {
    throw new TypeError('package.idempotency_key contains unsupported characters')
  }
  if (!/^[a-f0-9]{64}$/.test(value.payload_sha256)) {
    throw new TypeError('package.payload_sha256 must be a lowercase SHA-256 digest')
  }
  if (!isRfc3339(value.created_at)) {
    throw new TypeError('package.created_at must be an RFC3339 timestamp')
  }
}

function assertBoundText(value, label, min, max) {
  if (
    typeof value !== 'string'
    || value.length < min
    || value.length > max
    || value.trim() !== value
    || /[\u0000-\u001f\u007f]/.test(value)
  ) {
    throw new TypeError(`${label} must be a trimmed ${min}-${max} character value`)
  }
}

function assertRsaKey(key, label) {
  const modulusLength = key.asymmetricKeyDetails?.modulusLength
  if (
    key.asymmetricKeyType !== 'rsa'
    || !Number.isInteger(modulusLength)
    || modulusLength < 2048
    || modulusLength > 8192
  ) {
    throw new TypeError(`${label} must be a 2048-8192 bit RSA key`)
  }
  return modulusLength
}

function isRfc3339(value) {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    && Number.isFinite(Date.parse(value))
}
