export const CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA =
  'open_commerce.consumer_data_vault_envelope.v1'
export const CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA =
  'open_commerce.consumer_data_vault_plaintext.v1'
export const CONSUMER_DATA_VAULT_ITERATIONS = 310_000

const MAX_PLAINTEXT_BYTES = 900 * 1024
const MAX_CIPHERTEXT_BYTES = 1024 * 1024
const MAX_PASSPHRASE_BYTES = 1024
const BASE64_PATTERN = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/
const RFC3339_PATTERN = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/

export async function encryptConsumerDataVaultItem(recordId, revision, content, passphrase) {
  assertRecord(recordId, revision)
  assertPassphrase(passphrase)
  if (typeof content !== 'string') {
    throw new Error('保险箱明文必须为文本')
  }

  const plaintextValue = {
    schema: CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA,
    record_id: recordId,
    revision,
    content,
  }
  const plaintext = new TextEncoder().encode(JSON.stringify(plaintextValue))
  if (plaintext.byteLength > MAX_PLAINTEXT_BYTES) {
    throw new Error('保险箱明文超过 900 KiB 本地加密上限')
  }

  const salt = crypto.getRandomValues(new Uint8Array(16))
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const key = await deriveKey(passphrase, salt, ['encrypt'])
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: toArrayBuffer(nonce),
      additionalData: new TextEncoder().encode(aad(recordId, revision)),
      tagLength: 128,
    },
    key,
    toArrayBuffer(plaintext),
  ))

  return {
    schema: CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA,
    record_id: recordId,
    revision,
    kdf: {
      name: 'PBKDF2',
      hash: 'SHA-256',
      iterations: CONSUMER_DATA_VAULT_ITERATIONS,
      salt_base64: bytesToBase64(salt),
    },
    cipher: {
      name: 'AES-256-GCM',
      nonce_base64: bytesToBase64(nonce),
      auth_tag_length_bits: 128,
    },
    ciphertext_base64: bytesToBase64(ciphertext),
    created_at: new Date().toISOString(),
  }
}

export async function decryptConsumerDataVaultItem(envelope, passphrase) {
  const decoded = assertEnvelope(envelope)
  assertPassphrase(passphrase)
  const key = await deriveKey(passphrase, decoded.salt, ['decrypt'])

  try {
    const plaintext = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: toArrayBuffer(decoded.nonce),
        additionalData: new TextEncoder().encode(aad(envelope.record_id, envelope.revision)),
        tagLength: 128,
      },
      key,
      toArrayBuffer(decoded.ciphertext),
    )
    const decodedPlaintext = new TextDecoder('utf-8', { fatal: true }).decode(plaintext)
    const parsed = JSON.parse(decodedPlaintext)
    if (
      !isObject(parsed)
      || parsed.schema !== CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA
      || parsed.record_id !== envelope.record_id
      || parsed.revision !== envelope.revision
      || typeof parsed.content !== 'string'
    ) {
      throw new Error('plaintext mismatch')
    }
    return parsed.content
  } catch {
    throw new Error('保险箱解密或认证失败')
  }
}

function assertEnvelope(envelope) {
  if (!isObject(envelope) || !isObject(envelope.kdf) || !isObject(envelope.cipher)) {
    throw new Error('不支持的消费者数据保险箱格式')
  }
  assertRecord(envelope.record_id, envelope.revision)
  if (
    envelope.schema !== CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA
    || envelope.kdf.name !== 'PBKDF2'
    || envelope.kdf.hash !== 'SHA-256'
    || envelope.kdf.iterations !== CONSUMER_DATA_VAULT_ITERATIONS
    || envelope.cipher.name !== 'AES-256-GCM'
    || envelope.cipher.auth_tag_length_bits !== 128
    || !isRfc3339(envelope.created_at)
  ) {
    throw new Error('不支持的消费者数据保险箱格式')
  }

  const salt = base64ToBytes(envelope.kdf.salt_base64, 16)
  const nonce = base64ToBytes(envelope.cipher.nonce_base64, 12)
  const ciphertext = base64ToBytes(envelope.ciphertext_base64, MAX_CIPHERTEXT_BYTES)
  if (salt.length !== 16 || nonce.length !== 12 || ciphertext.length < 17) {
    throw new Error('不支持的消费者数据保险箱格式')
  }
  return { salt, nonce, ciphertext }
}

function assertRecord(recordId, revision) {
  if (
    typeof recordId !== 'string'
    || !/^[A-Za-z0-9_-]{8,120}$/.test(recordId)
    || !Number.isSafeInteger(revision)
    || revision < 1
  ) {
    throw new Error('保险箱记录 ID 或修订号无效')
  }
}

function assertPassphrase(passphrase) {
  if (typeof passphrase !== 'string') {
    throw new Error('保险箱口令长度必须为 12 到 256 个字符')
  }
  const characters = Array.from(passphrase).length
  const bytes = new TextEncoder().encode(passphrase).byteLength
  if (characters < 12 || characters > 256 || bytes > MAX_PASSPHRASE_BYTES) {
    throw new Error('保险箱口令长度必须为 12 到 256 个字符')
  }
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
      iterations: CONSUMER_DATA_VAULT_ITERATIONS,
    },
    material,
    { name: 'AES-GCM', length: 256 },
    false,
    usages,
  )
}

function aad(recordId, revision) {
  return `${CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA}:${recordId}:${revision}`
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

function base64ToBytes(value, maxDecodedBytes) {
  const maxEncodedLength = Math.ceil(maxDecodedBytes / 3) * 4
  if (
    typeof value !== 'string'
    || value.length === 0
    || value.length > maxEncodedLength
    || !BASE64_PATTERN.test(value)
  ) {
    throw new Error('不支持的消费者数据保险箱格式')
  }
  let decoded
  try {
    const binary = atob(value)
    decoded = Uint8Array.from(binary, (character) => character.charCodeAt(0))
  } catch {
    throw new Error('不支持的消费者数据保险箱格式')
  }
  if (decoded.byteLength > maxDecodedBytes || bytesToBase64(decoded) !== value) {
    throw new Error('不支持的消费者数据保险箱格式')
  }
  return decoded
}

function isRfc3339(value) {
  if (typeof value !== 'string') return false
  const match = RFC3339_PATTERN.exec(value)
  if (!match) return false
  const [, year, month, day, hour, minute, second, , offsetHour, offsetMinute] = match
  const numericYear = Number(year)
  const numericMonth = Number(month)
  const numericDay = Number(day)
  const daysInMonth = new Date(Date.UTC(numericYear, numericMonth, 0)).getUTCDate()
  return numericYear >= 1
    && numericMonth >= 1
    && numericMonth <= 12
    && numericDay >= 1
    && numericDay <= daysInMonth
    && Number(hour) <= 23
    && Number(minute) <= 59
    && Number(second) <= 59
    && (offsetHour === undefined || (Number(offsetHour) <= 23 && Number(offsetMinute) <= 59))
    && Number.isFinite(Date.parse(value))
}

function isObject(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
