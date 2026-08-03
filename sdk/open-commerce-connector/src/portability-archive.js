import { createHash, pbkdf2, randomBytes, webcrypto } from 'node:crypto'
import { promisify } from 'node:util'

export const CONSUMER_PORTABILITY_ARCHIVE_SCHEMA =
  'open_commerce.consumer_portability_encrypted_archive.v1'
export const CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS = 310_000
const ARCHIVE_AAD = 'open_commerce.consumer_portability_encrypted_archive.v1'
const deriveBytes = promisify(pbkdf2)

export async function encryptConsumerPortabilityArchive(value, passphrase) {
  assertPassphrase(passphrase)
  const plaintext = Buffer.from(JSON.stringify(value), 'utf8')
  const salt = randomBytes(16)
  const nonce = randomBytes(12)
  const keyBytes = await deriveBytes(
    Buffer.from(passphrase, 'utf8'),
    salt,
    CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
    32,
    'sha256',
  )
  const key = await webcrypto.subtle.importKey('raw', keyBytes, 'AES-GCM', false, ['encrypt'])
  const ciphertext = Buffer.from(await webcrypto.subtle.encrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: Buffer.from(ARCHIVE_AAD, 'utf8'),
      tagLength: 128,
    },
    key,
    plaintext,
  ))
  return {
    schema: CONSUMER_PORTABILITY_ARCHIVE_SCHEMA,
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
    ciphertext_base64: ciphertext.toString('base64'),
    created_at: new Date().toISOString(),
  }
}

export async function decryptConsumerPortabilityArchive(archive, passphrase) {
  assertArchive(archive)
  assertPassphrase(passphrase)
  const salt = Buffer.from(archive.kdf.salt_base64, 'base64')
  const nonce = Buffer.from(archive.cipher.nonce_base64, 'base64')
  const ciphertext = Buffer.from(archive.ciphertext_base64, 'base64')
  const keyBytes = await deriveBytes(
    Buffer.from(passphrase, 'utf8'),
    salt,
    archive.kdf.iterations,
    32,
    'sha256',
  )
  const key = await webcrypto.subtle.importKey('raw', keyBytes, 'AES-GCM', false, ['decrypt'])
  const plaintext = Buffer.from(await webcrypto.subtle.decrypt(
    {
      name: 'AES-GCM',
      iv: nonce,
      additionalData: Buffer.from(ARCHIVE_AAD, 'utf8'),
      tagLength: 128,
    },
    key,
    ciphertext,
  ))
  const digest = createHash('sha256').update(plaintext).digest('hex')
  if (digest !== archive.plaintext_sha256) throw new Error('archive plaintext digest mismatch')
  return JSON.parse(plaintext.toString('utf8'))
}

function assertPassphrase(value) {
  if (typeof value !== 'string' || value.length < 12 || value.length > 256) {
    throw new TypeError('passphrase must contain 12-256 characters')
  }
}

function assertArchive(value) {
  if (
    value?.schema !== CONSUMER_PORTABILITY_ARCHIVE_SCHEMA
    || value.kdf?.name !== 'PBKDF2'
    || value.kdf?.hash !== 'SHA-256'
    || value.kdf?.iterations !== CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS
    || value.cipher?.name !== 'AES-256-GCM'
    || value.cipher?.auth_tag_length_bits !== 128
    || typeof value.kdf?.salt_base64 !== 'string'
    || typeof value.cipher?.nonce_base64 !== 'string'
    || !/^[a-f0-9]{64}$/.test(value.plaintext_sha256)
    || typeof value.ciphertext_base64 !== 'string'
  ) {
    throw new TypeError('unsupported consumer portability archive')
  }
  if (Buffer.from(value.kdf.salt_base64, 'base64').length !== 16) {
    throw new TypeError('archive salt must contain 16 bytes')
  }
  if (Buffer.from(value.cipher.nonce_base64, 'base64').length !== 12) {
    throw new TypeError('archive nonce must contain 12 bytes')
  }
}
