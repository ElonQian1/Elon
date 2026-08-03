const PROOF_PROTOCOL = 'open_commerce.merchant_identity_proof.v1'

export interface GeneratedMerchantIdentityProof {
  keyId: string
  publicKeyPem: string
  privateKeyPem: string
  proofSignatureBase64: string
}

export async function generateMerchantIdentityProof(
  projectId: string,
  merchantId: string,
): Promise<GeneratedMerchantIdentityProof> {
  const pair = await crypto.subtle.generateKey(
    {
      name: 'RSASSA-PKCS1-v1_5',
      modulusLength: 3072,
      publicExponent: new Uint8Array([1, 0, 1]),
      hash: 'SHA-256',
    },
    true,
    ['sign', 'verify'],
  ) as CryptoKeyPair
  const publicDer = new Uint8Array(await crypto.subtle.exportKey('spki', pair.publicKey))
  const privateDer = new Uint8Array(await crypto.subtle.exportKey('pkcs8', pair.privateKey))
  const keyId = bytesToHex(new Uint8Array(await crypto.subtle.digest('SHA-256', publicDer)))
  const message = [PROOF_PROTOCOL, projectId.trim(), merchantId.trim(), keyId].join('\n')
  const signature = new Uint8Array(await crypto.subtle.sign(
    'RSASSA-PKCS1-v1_5',
    pair.privateKey,
    new TextEncoder().encode(message),
  ))
  return {
    keyId,
    publicKeyPem: toPem('PUBLIC KEY', publicDer),
    privateKeyPem: toPem('PRIVATE KEY', privateDer),
    proofSignatureBase64: bytesToBase64(signature),
  }
}

export function downloadMerchantIdentityPrivateKey(
  merchantId: string,
  proof: GeneratedMerchantIdentityProof,
) {
  const blob = new Blob([proof.privateKeyPem], { type: 'application/x-pem-file' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `merchant-${merchantId}-${proof.keyId.slice(0, 12)}-private.pem`
  anchor.click()
  URL.revokeObjectURL(url)
}

function toPem(label: string, bytes: Uint8Array) {
  const base64 = bytesToBase64(bytes)
  const lines = base64.match(/.{1,64}/g) ?? []
  return `-----BEGIN ${label}-----\n${lines.join('\n')}\n-----END ${label}-----\n`
}

function bytesToBase64(bytes: Uint8Array) {
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

function bytesToHex(bytes: Uint8Array) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
