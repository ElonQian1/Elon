import { createHash, pbkdf2, randomBytes, webcrypto } from 'node:crypto'
import { promisify } from 'node:util'
import { decodeStrictBase64 } from './strict-base64.js'

export const CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1 =
  'open_commerce.consumer_portability_encrypted_archive.v1'
export const CONSUMER_PORTABILITY_ARCHIVE_SCHEMA =
  'open_commerce.consumer_portability_encrypted_archive.v2'
export const CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS = 310_000
export const CONSUMER_PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES = 6 * 1024 * 1024
const LEGACY_ARCHIVE_AAD = CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1
const ARCHIVE_AAD_PROTOCOL = 'open_commerce.consumer_portability_archive_aad.v2'
const deriveBytes = promisify(pbkdf2)

export async function encryptConsumerPortabilityArchive(value, passphrase) {
  assertPassphrase(passphrase)
  const plaintext = serializeArchiveValue(value)
  const salt = randomBytes(16)
  const nonce = randomBytes(12)
  const createdAt = new Date().toISOString()
  const archive = archiveMetadata({
    schema: CONSUMER_PORTABILITY_ARCHIVE_SCHEMA,
    salt,
    nonce,
    plaintext,
    createdAt,
  })
  const key = await deriveArchiveKey(passphrase, salt, ['encrypt'])
  const ciphertext = Buffer.from(await webcrypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: archiveAad(archive),
      tagLength: 128,
    },
    key,
    plaintext,
  ))
  return {
    ...archive,
    ciphertext_base64: ciphertext.toString('base64'),
  }
}

export async function decryptConsumerPortabilityArchive(archive, passphrase) {
  assertPassphrase(passphrase)
  const { salt, nonce, ciphertext } = decodeArchive(archive)
  const key = await deriveArchiveKey(passphrase, salt, ['decrypt'])
  let plaintext
  try {
    plaintext = Buffer.from(await webcrypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv: nonce,
        additionalData: archiveAad(archive),
        tagLength: 128,
      },
      key,
      ciphertext,
    ))
  } catch (cause) {
    throw new Error('consumer portability archive authentication failed', { cause })
  }
  const digest = createHash('sha256').update(plaintext).digest('hex')
  if (digest !== archive.plaintext_sha256) {
    throw new Error('consumer portability archive authentication failed')
  }
  try {
    return JSON.parse(plaintext.toString('utf8'))
  } catch (cause) {
    throw new Error('consumer portability archive plaintext is not valid JSON', { cause })
  }
}

function assertPassphrase(value) {
  const characters = typeof value === 'string' ? Array.from(value).length : 0
  const bytes = typeof value === 'string' ? Buffer.byteLength(value, 'utf8') : 0
  if (characters < 12 || characters > 256 || bytes > 1024) {
    throw new TypeError('passphrase must contain 12-256 characters')
  }
}

function decodeArchive(value) {
  if (
    ![CONSUMER_PORTABILITY_ARCHIVE_SCHEMA, CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1]
      .includes(value?.schema)
    || value.kdf?.name !== 'PBKDF2'
    || value.kdf?.hash !== 'SHA-256'
    || value.kdf?.iterations !== CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS
    || value.cipher?.name !== 'AES-256-GCM'
    || value.cipher?.auth_tag_length_bits !== 128
    || typeof value.kdf?.salt_base64 !== 'string'
    || typeof value.cipher?.nonce_base64 !== 'string'
    || !/^[a-f0-9]{64}$/.test(value.plaintext_sha256)
    || typeof value.ciphertext_base64 !== 'string'
    || typeof value.created_at !== 'string'
    || !isRfc3339(value.created_at)
  ) {
    throw new TypeError('unsupported consumer portability archive')
  }
  const salt = decodeStrictBase64(value.kdf.salt_base64, {
    label: 'archive.kdf.salt_base64', minBytes: 16, maxBytes: 16,
  })
  const nonce = decodeStrictBase64(value.cipher.nonce_base64, {
    label: 'archive.cipher.nonce_base64', minBytes: 12, maxBytes: 12,
  })
  const ciphertext = decodeStrictBase64(value.ciphertext_base64, {
    label: 'archive.ciphertext_base64',
    minBytes: 17,
    maxBytes: CONSUMER_PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES + 16,
  })
  return { salt, nonce, ciphertext }
}

function serializeArchiveValue(value) {
  let json
  try {
    json = JSON.stringify(value)
  } catch (cause) {
    throw new TypeError('archive value must be JSON serializable', { cause })
  }
  if (json === undefined) throw new TypeError('archive value must be JSON serializable')
  const plaintext = Buffer.from(json, 'utf8')
  if (plaintext.length > CONSUMER_PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES) {
    throw new TypeError('archive plaintext exceeds the supported size')
  }
  return plaintext
}

function archiveMetadata({ schema, salt, nonce, plaintext, createdAt }) {
  return {
    schema,
    kdf: {
      name: 'PBKDF2',
      hash: 'SHA-256',
      iterations: CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
      salt_base64: salt.toString('base64'),
    },
    cipher: {
      name: 'AES-256-GCM',
      nonce_base64: nonce.toString('base64'),
      auth_tag_length_bits: 128,
    },
    plaintext_sha256: createHash('sha256').update(plaintext).digest('hex'),
    created_at: createdAt,
  }
}

function archiveAad(archive) {
  if (archive.schema === CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1) {
    return Buffer.from(LEGACY_ARCHIVE_AAD, 'utf8')
  }
  return Buffer.from([
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
  ].join('\n'), 'utf8')
}

async function deriveArchiveKey(passphrase, salt, usages) {
  const passphraseBytes = Buffer.from(passphrase, 'utf8')
  let keyBytes
  try {
    keyBytes = await deriveBytes(
      passphraseBytes,
      salt,
      CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
      32,
      'sha256',
    )
    return await webcrypto.subtle.importKey('raw', keyBytes, 'AES-GCM', false, usages)
  } finally {
    passphraseBytes.fill(0)
    keyBytes?.fill(0)
  }
}

function isRfc3339(value) {
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value)
    && Number.isFinite(Date.parse(value))
}
