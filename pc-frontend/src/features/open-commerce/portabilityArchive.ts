export const PORTABILITY_ARCHIVE_SCHEMA =
  'open_commerce.consumer_portability_encrypted_archive.v1' as const
export const PORTABILITY_ARCHIVE_ITERATIONS = 310_000
const ARCHIVE_AAD = PORTABILITY_ARCHIVE_SCHEMA

export interface PortabilityEncryptedArchive {
  schema: typeof PORTABILITY_ARCHIVE_SCHEMA
  kdf: {
    name: 'PBKDF2'
    hash: 'SHA-256'
    iterations: typeof PORTABILITY_ARCHIVE_ITERATIONS
    salt_base64: string
  }
  cipher: {
    name: 'AES-256-GCM'
    nonce_base64: string
    auth_tag_length_bits: 128
  }
  plaintext_sha256: string
  ciphertext_base64: string
  created_at: string
}

export async function encryptPortabilityArchive(
  value: unknown,
  passphrase: string,
): Promise<PortabilityEncryptedArchive> {
  assertPassphrase(passphrase)
  const plaintext = new TextEncoder().encode(JSON.stringify(value))
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const key = await deriveKey(passphrase, salt, ['encrypt'])
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: new TextEncoder().encode(ARCHIVE_AAD),
      tagLength: 128,
    },
    key,
    plaintext,
  ))
  return {
    schema: PORTABILITY_ARCHIVE_SCHEMA,
    kdf: {
      name: 'PBKDF2',
      hash: 'SHA-256',
      iterations: PORTABILITY_ARCHIVE_ITERATIONS,
      salt_base64: bytesToBase64(salt),
    },
    cipher: {
      name: 'AES-256-GCM',
      nonce_base64: bytesToBase64(nonce),
      auth_tag_length_bits: 128,
    },
    plaintext_sha256: await sha256Hex(plaintext),
    ciphertext_base64: bytesToBase64(ciphertext),
    created_at: new Date().toISOString(),
  }
}

export async function decryptPortabilityArchive(
  archive: PortabilityEncryptedArchive,
  passphrase: string,
): Promise<unknown> {
  assertArchive(archive)
  assertPassphrase(passphrase)
  const salt = base64ToBytes(archive.kdf.salt_base64)
  const nonce = base64ToBytes(archive.cipher.nonce_base64)
  const ciphertext = base64ToBytes(archive.ciphertext_base64)
  const key = await deriveKey(passphrase, salt, ['decrypt'])
  const plaintext = new Uint8Array(await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: new TextEncoder().encode(ARCHIVE_AAD),
      tagLength: 128,
    },
    key,
    ciphertext,
  ))
  if (await sha256Hex(plaintext) !== archive.plaintext_sha256) {
    throw new Error('解密后的数据包摘要不一致')
  }
  return JSON.parse(new TextDecoder().decode(plaintext))
}

export function isPortabilityEncryptedArchive(value: unknown): value is PortabilityEncryptedArchive {
  return Boolean(
    value
    && typeof value === 'object'
    && (value as PortabilityEncryptedArchive).schema === PORTABILITY_ARCHIVE_SCHEMA,
  )
}

async function deriveKey(
  passphrase: string,
  salt: Uint8Array<ArrayBuffer>,
  usages: KeyUsage[],
) {
  const material = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(passphrase),
    'PBKDF2',
    false,
    ['deriveKey'],
  )
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', hash: 'SHA-256', salt, iterations: PORTABILITY_ARCHIVE_ITERATIONS },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    usages,
  )
}

function assertArchive(value: PortabilityEncryptedArchive) {
  if (
    value.schema !== PORTABILITY_ARCHIVE_SCHEMA
    || value.kdf?.name !== 'PBKDF2'
    || value.kdf?.hash !== 'SHA-256'
    || value.kdf?.iterations !== PORTABILITY_ARCHIVE_ITERATIONS
    || value.cipher?.name !== 'AES-256-GCM'
    || value.cipher?.auth_tag_length_bits !== 128
    || base64ToBytes(value.kdf.salt_base64).length !== 16
    || base64ToBytes(value.cipher.nonce_base64).length !== 12
    || !/^[a-f0-9]{64}$/.test(value.plaintext_sha256)
  ) {
    throw new Error('不支持的加密数据包格式')
  }
}

function assertPassphrase(value: string) {
  if (value.length < 12 || value.length > 256) throw new Error('归档口令长度必须为 12 到 256 个字符')
}

async function sha256Hex(value: Uint8Array<ArrayBuffer>) {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', value))
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

function bytesToBase64(value: Uint8Array) {
  let binary = ''
  for (let offset = 0; offset < value.length; offset += 0x8000) {
    binary += String.fromCharCode(...value.subarray(offset, offset + 0x8000))
  }
  return btoa(binary)
}

function base64ToBytes(value: string) {
  const binary = atob(value)
  return Uint8Array.from(binary, (character) => character.charCodeAt(0))
}
