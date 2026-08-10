export const PORTABILITY_ARCHIVE_SCHEMA_V1 =
  'open_commerce.consumer_portability_encrypted_archive.v1'
export const PORTABILITY_ARCHIVE_SCHEMA =
  'open_commerce.consumer_portability_encrypted_archive.v2'
export const PORTABILITY_ARCHIVE_ITERATIONS = 310_000
export const PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES = 6 * 1024 * 1024
const LEGACY_ARCHIVE_AAD = PORTABILITY_ARCHIVE_SCHEMA_V1
const ARCHIVE_AAD_PROTOCOL = 'open_commerce.consumer_portability_archive_aad.v2'

export async function encryptPortabilityArchive(value, passphrase) {
  assertPassphrase(passphrase)
  const plaintext = serializeArchiveValue(value)
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const archive = await archiveMetadata(salt, nonce, plaintext)
  const key = await deriveKey(passphrase, salt, ['encrypt'])
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: toArrayBuffer(nonce),
      additionalData: toArrayBuffer(archiveAad(archive)),
      tagLength: 128,
    },
    key,
    toArrayBuffer(plaintext),
  ))
  return { ...archive, ciphertext_base64: bytesToBase64(ciphertext) }
}

export async function decryptPortabilityArchive(archive, passphrase) {
  assertPassphrase(passphrase)
  const { salt, nonce, ciphertext } = decodeArchive(archive)
  const key = await deriveKey(passphrase, salt, ['decrypt'])
  let plaintext
  try {
    plaintext = new Uint8Array(await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: toArrayBuffer(nonce),
        additionalData: toArrayBuffer(archiveAad(archive)),
        tagLength: 128,
      },
      key,
      toArrayBuffer(ciphertext),
    ))
  } catch {
    throw new Error('消费者数据归档认证失败')
  }
  if (await sha256Hex(plaintext) !== archive.plaintext_sha256) {
    throw new Error('消费者数据归档认证失败')
  }
  try {
    return JSON.parse(new TextDecoder().decode(plaintext))
  } catch {
    throw new Error('消费者数据归档明文不是有效 JSON')
  }
}

export function isPortabilityEncryptedArchive(value) {
  const schema = value && typeof value === 'object' ? value.schema : undefined
  return schema === PORTABILITY_ARCHIVE_SCHEMA || schema === PORTABILITY_ARCHIVE_SCHEMA_V1
}

async function deriveKey(passphrase, salt, usages) {
  const material = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(passphrase),
    'PBKDF2',
    false,
    ['deriveKey'],
  )
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      hash: 'SHA-256',
      salt: toArrayBuffer(salt),
      iterations: PORTABILITY_ARCHIVE_ITERATIONS,
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    usages,
  )
}

function decodeArchive(value) {
  if (
    ![PORTABILITY_ARCHIVE_SCHEMA, PORTABILITY_ARCHIVE_SCHEMA_V1].includes(value?.schema)
    || value.kdf?.name !== 'PBKDF2'
    || value.kdf?.hash !== 'SHA-256'
    || value.kdf?.iterations !== PORTABILITY_ARCHIVE_ITERATIONS
    || value.cipher?.name !== 'AES-256-GCM'
    || value.cipher?.auth_tag_length_bits !== 128
    || !/^[a-f0-9]{64}$/.test(value.plaintext_sha256)
    || !isRfc3339(value.created_at)
  ) {
    throw new Error('不支持的加密数据包格式')
  }
  return {
    salt: strictBase64(value.kdf.salt_base64, '归档盐', 16, 16),
    nonce: strictBase64(value.cipher.nonce_base64, '归档 Nonce', 12, 12),
    ciphertext: strictBase64(
      value.ciphertext_base64,
      '归档密文',
      17,
      PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES + 16,
    ),
  }
}

function serializeArchiveValue(value) {
  let json
  try {
    json = JSON.stringify(value)
  } catch {
    throw new Error('归档内容必须可以序列化为 JSON')
  }
  if (json === undefined) throw new Error('归档内容必须可以序列化为 JSON')
  const plaintext = new TextEncoder().encode(json)
  if (plaintext.length > PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES) {
    throw new Error('归档明文超过允许大小')
  }
  return plaintext
}

async function archiveMetadata(salt, nonce, plaintext) {
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
    ciphertext_base64: '',
    created_at: new Date().toISOString(),
  }
}

function archiveAad(archive) {
  if (archive.schema === PORTABILITY_ARCHIVE_SCHEMA_V1) {
    return new TextEncoder().encode(LEGACY_ARCHIVE_AAD)
  }
  return new TextEncoder().encode([
    ARCHIVE_AAD_PROTOCOL,
    archive.schema,
    archive.kdf.name,
    archive.kdf.hash,
    String(archive.kdf.iterations),
    archive.kdf.salt_base64,
    archive.cipher.name,
    archive.cipher.nonce_base64,
    String(archive.cipher.auth_tag_length_bits),
    archive.plaintext_sha256,
    archive.created_at,
  ].join('\n'))
}

function assertPassphrase(value) {
  const characters = typeof value === 'string' ? Array.from(value).length : 0
  const bytes = typeof value === 'string' ? new TextEncoder().encode(value).length : 0
  if (characters < 12 || characters > 256 || bytes > 1024) {
    throw new Error('归档口令长度必须为 12 到 256 个字符')
  }
}

async function sha256Hex(value) {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', toArrayBuffer(value)))
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('')
}

function strictBase64(value, label, minBytes, maxBytes) {
  if (
    typeof value !== 'string'
    || value.length % 4 !== 0
    || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)
    || value.length > Math.ceil(maxBytes / 3) * 4
  ) {
    throw new Error(`${label}不是规范 Base64`)
  }
  const decoded = base64ToBytes(value)
  if (decoded.length < minBytes || decoded.length > maxBytes || bytesToBase64(decoded) !== value) {
    throw new Error(`${label}长度无效`)
  }
  return decoded
}

function isRfc3339(value) {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    && Number.isFinite(Date.parse(value))
}

function toArrayBuffer(value) {
  const copy = new Uint8Array(value.byteLength)
  copy.set(value)
  return copy.buffer
}

function bytesToBase64(value) {
  let binary = ''
  for (let offset = 0; offset < value.length; offset += 0x8000) {
    binary += String.fromCharCode(...value.subarray(offset, offset + 0x8000))
  }
  return btoa(binary)
}

function base64ToBytes(value) {
  const binary = atob(value)
  return Uint8Array.from(binary, (character) => character.charCodeAt(0))
}
