import {
  createHash,
  createPrivateKey,
  createPublicKey,
  createSign,
  createVerify,
} from 'node:crypto'

export const MERCHANT_IDENTITY_ALGORITHM = 'rsa-pkcs1v15-sha256'
export const MERCHANT_IDENTITY_PROOF_PROTOCOL = 'open_commerce.merchant_identity_proof.v1'

export function merchantIdentityPublicKeyId(publicKey) {
  const key = createPublicKey(publicKey)
  if (key.asymmetricKeyType !== 'rsa') throw new TypeError('publicKey must be an RSA key')
  const der = key.export({ type: 'spki', format: 'der' })
  return createHash('sha256').update(der).digest('hex')
}

export function merchantIdentityProofMessage({ projectId, merchantId, keyId }) {
  assertIdentifier(projectId, 'projectId')
  assertIdentifier(merchantId, 'merchantId')
  if (typeof keyId !== 'string' || !/^[a-f0-9]{64}$/.test(keyId)) {
    throw new TypeError('keyId must be a lowercase SHA-256 digest')
  }
  return [MERCHANT_IDENTITY_PROOF_PROTOCOL, projectId, merchantId, keyId].join('\n')
}

export function createMerchantIdentityProof({ projectId, merchantId, privateKey }) {
  const privateKeyObject = createPrivateKey(privateKey)
  if (privateKeyObject.asymmetricKeyType !== 'rsa') {
    throw new TypeError('privateKey must be an RSA key')
  }
  const publicKey = createPublicKey(privateKeyObject)
  const publicKeyPem = publicKey.export({ type: 'spki', format: 'pem' }).toString()
  const keyId = merchantIdentityPublicKeyId(publicKey)
  const message = merchantIdentityProofMessage({ projectId, merchantId, keyId })
  const signer = createSign('RSA-SHA256')
  signer.update(message, 'utf8')
  signer.end()
  return {
    key_id: keyId,
    algorithm: MERCHANT_IDENTITY_ALGORITHM,
    public_key_pem: publicKeyPem,
    proof_signature_base64: signer.sign(privateKeyObject).toString('base64'),
  }
}

export function verifyMerchantIdentityProof({ projectId, merchantId, proof }) {
  if (proof?.algorithm !== MERCHANT_IDENTITY_ALGORITHM) return false
  if (merchantIdentityPublicKeyId(proof.public_key_pem) !== proof.key_id) return false
  const message = merchantIdentityProofMessage({ projectId, merchantId, keyId: proof.key_id })
  const verifier = createVerify('RSA-SHA256')
  verifier.update(message, 'utf8')
  verifier.end()
  return verifier.verify(
    proof.public_key_pem,
    Buffer.from(proof.proof_signature_base64, 'base64'),
  )
}

function assertIdentifier(value, name) {
  if (
    typeof value !== 'string'
    || value.length < 1
    || value.length > 120
    || value.trim() !== value
    || /[\u0000-\u001f\u007f]/.test(value)
  ) {
    throw new TypeError(`${name} must be a trimmed 1-120 character identifier`)
  }
}
