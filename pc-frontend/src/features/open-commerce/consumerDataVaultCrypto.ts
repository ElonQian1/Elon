import type { ConsumerDataVaultEnvelope } from './openCommerceClientTypes'

export const CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA =
  'open_commerce.consumer_data_vault_envelope.v1' as const
export const CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA =
  'open_commerce.consumer_data_vault_plaintext.v1' as const
export const CONSUMER_DATA_VAULT_ITERATIONS = 310_000 as const
const MAX_PLAINTEXT_BYTES = 900 * 1024

interface ConsumerDataVaultPlaintext {
  schema: typeof CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA
  record_id: string
  revision: number
  content: string
}

export async function encryptConsumerDataVaultItem(
  recordId: string,
  revision: number,
  content: string,
  passphrase: string,
): Promise<ConsumerDataVaultEnvelope> {
  assertRecord(recordId, revision)
  assertPassphrase(passphrase)
  const plaintextValue: ConsumerDataVaultPlaintext = {
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

export async function decryptConsumerDataVaultItem(
  envelope: ConsumerDataVaultEnvelope,
  passphrase: string,
): Promise<string> {
  assertEnvelope(envelope)
  assertPassphrase(passphrase)
  const salt = base64ToBytes(envelope.kdf.salt_base64)
  const nonce = base64ToBytes(envelope.cipher.nonce_base64)
  const ciphertext = base64ToBytes(envelope.ciphertext_base64)
  const key = await deriveKey(passphrase, salt, ['decrypt'])
  const plaintext = await crypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: toArrayBuffer(nonce),
      additionalData: new TextEncoder().encode(aad(envelope.record_id, envelope.revision)),
      tagLength: 128,
    },
    key,
    toArrayBuffer(ciphertext),
  )
  const parsed = JSON.parse(new TextDecoder().decode(plaintext)) as ConsumerDataVaultPlaintext
  if (
    parsed.schema !== CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA
    || parsed.record_id !== envelope.record_id
    || parsed.revision !== envelope.revision
    || typeof parsed.content !== 'string'
  ) {
    throw new Error('保险箱明文与密文信封不匹配')
  }
  return parsed.content
}

function assertEnvelope(envelope: ConsumerDataVaultEnvelope) {
  assertRecord(envelope.record_id, envelope.revision)
  if (
    envelope.schema !== CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA
    || envelope.kdf?.name !== 'PBKDF2'
    || envelope.kdf?.hash !== 'SHA-256'
    || envelope.kdf?.iterations !== CONSUMER_DATA_VAULT_ITERATIONS
    || envelope.cipher?.name !== 'AES-256-GCM'
    || envelope.cipher?.auth_tag_length_bits !== 128
    || base64ToBytes(envelope.kdf.salt_base64).length !== 16
    || base64ToBytes(envelope.cipher.nonce_base64).length !== 12
    || base64ToBytes(envelope.ciphertext_base64).length < 17
  ) {
    throw new Error('不支持的消费者数据保险箱格式')
  }
}

function assertRecord(recordId: string, revision: number) {
  if (!/^[A-Za-z0-9_-]{8,120}$/.test(recordId) || !Number.isSafeInteger(revision) || revision < 1) {
    throw new Error('保险箱记录 ID 或修订号无效')
  }
}

function assertPassphrase(passphrase: string) {
  if (passphrase.length < 12 || passphrase.length > 256) {
    throw new Error('保险箱口令长度必须为 12 到 256 个字符')
  }
}

async function deriveKey(passphrase: string, salt: Uint8Array, usages: KeyUsage[]) {
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

function aad(recordId: string, revision: number) {
  return `${CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA}:${recordId}:${revision}`
}

function toArrayBuffer(value: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(value.byteLength)
  copy.set(value)
  return copy.buffer
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
