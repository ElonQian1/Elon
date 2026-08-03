import {
  createHash,
  createPrivateKey,
  createPublicKey,
  createSign,
  createVerify,
} from 'node:crypto'

export const CONSUMER_PORTABILITY_SIGNATURE_SCHEMA =
  'open_commerce.consumer_portability_signed_package.v1'
export const CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM = 'rsa-pkcs1v15-sha256'
const SIGNATURE_PROTOCOL = 'open_commerce.consumer_portability_signature.v1'

export function consumerPortabilityPublicKeyId(publicKey) {
  const key = createPublicKey(publicKey)
  if (key.asymmetricKeyType !== 'rsa') throw new TypeError('publicKey must be an RSA key')
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
  if (privateKeyObject.asymmetricKeyType !== 'rsa') {
    throw new TypeError('privateKey must be an RSA key')
  }
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
  if (signedPackage?.schema !== CONSUMER_PORTABILITY_SIGNATURE_SCHEMA) return false
  if (signedPackage.signature?.algorithm !== CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM) return false
  const expectedKeyId = consumerPortabilityPublicKeyId(publicKey)
  if (signedPackage.signature.key_id !== expectedKeyId) return false
  const message = consumerPortabilitySignatureMessage({
    sourceOperator: signedPackage.source_operator,
    keyId: expectedKeyId,
    package: signedPackage.package,
  })
  const verifier = createVerify('RSA-SHA256')
  verifier.update(message, 'utf8')
  verifier.end()
  return verifier.verify(publicKey, Buffer.from(signedPackage.signature.signature_base64, 'base64'))
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
  const fields = [
    'schema',
    'id',
    'source_project_id',
    'idempotency_key',
    'payload_sha256',
    'created_at',
  ]
  for (const field of fields) {
    if (typeof value[field] !== 'string' || value[field].length === 0) {
      throw new TypeError(`package.${field} is required`)
    }
  }
  if (!/^[a-f0-9]{64}$/.test(value.payload_sha256)) {
    throw new TypeError('package.payload_sha256 must be a lowercase SHA-256 digest')
  }
}
