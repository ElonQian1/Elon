import assert from 'node:assert/strict'
import {
  createCipheriv,
  createDecipheriv,
  createHash,
  pbkdf2Sync,
  randomBytes,
} from 'node:crypto'
import test from 'node:test'

import {
  CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
  CONSUMER_PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES,
  CONSUMER_PORTABILITY_ARCHIVE_SCHEMA,
  CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1,
  decryptConsumerPortabilityArchive,
  encryptConsumerPortabilityArchive,
} from '../src/index.js'

const PASSPHRASE = 'correct horse battery staple'
const VALUE = { user: '测试用户', preferences: ['coffee', '安静'], revision: 3 }

test('encrypts a randomized authenticated v2 archive and restores JSON', async () => {
  const first = await encryptConsumerPortabilityArchive(VALUE, PASSPHRASE)
  const second = await encryptConsumerPortabilityArchive(VALUE, PASSPHRASE)

  assert.equal(first.schema, CONSUMER_PORTABILITY_ARCHIVE_SCHEMA)
  assert.equal(first.kdf.iterations, CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS)
  assert.notEqual(first.kdf.salt_base64, second.kdf.salt_base64)
  assert.notEqual(first.cipher.nonce_base64, second.cipher.nonce_base64)
  assert.notEqual(first.ciphertext_base64, second.ciphertext_base64)
  assert.deepEqual(await decryptConsumerPortabilityArchive(first, PASSPHRASE), VALUE)
  assert.deepEqual(decryptV2Externally(first, PASSPHRASE), VALUE)
})

test('rejects wrong credentials and authenticated v2 metadata or ciphertext changes', async () => {
  const archive = await encryptConsumerPortabilityArchive(VALUE, PASSPHRASE)
  await assert.rejects(
    decryptConsumerPortabilityArchive(archive, 'another valid passphrase'),
    /authentication failed/,
  )

  const mutations = [
    ['created_at', '2026-08-10T09:00:00.000Z'],
    ['plaintext_sha256', 'b'.repeat(64)],
  ]
  for (const [field, value] of mutations) {
    await assert.rejects(
      decryptConsumerPortabilityArchive({ ...archive, [field]: value }, PASSPHRASE),
      /authentication failed/,
    )
  }

  const ciphertext = Buffer.from(archive.ciphertext_base64, 'base64')
  ciphertext[0] ^= 1
  await assert.rejects(decryptConsumerPortabilityArchive({
    ...archive,
    ciphertext_base64: ciphertext.toString('base64'),
  }, PASSPHRASE), /authentication failed/)
})

test('strictly rejects malformed or resource-amplifying archive fields', async () => {
  const archive = await encryptConsumerPortabilityArchive(VALUE, PASSPHRASE)
  const malformed = [
    { ...archive, schema: 'unsupported' },
    { ...archive, kdf: { ...archive.kdf, iterations: Number.MAX_SAFE_INTEGER } },
    { ...archive, kdf: { ...archive.kdf, salt_base64: 'AA=A' } },
    { ...archive, cipher: { ...archive.cipher, nonce_base64: 'AA==' } },
    { ...archive, ciphertext_base64: 'AA==' },
    { ...archive, created_at: 'not-a-time' },
  ]
  for (const value of malformed) {
    await assert.rejects(decryptConsumerPortabilityArchive(value, PASSPHRASE), TypeError)
  }
})

test('continues to decrypt legacy v1 archives without writing new v1 archives', async () => {
  const legacy = legacyArchive(VALUE, PASSPHRASE)
  assert.equal(legacy.schema, CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1)
  assert.deepEqual(await decryptConsumerPortabilityArchive(legacy, PASSPHRASE), VALUE)
  assert.equal(
    (await encryptConsumerPortabilityArchive(VALUE, PASSPHRASE)).schema,
    CONSUMER_PORTABILITY_ARCHIVE_SCHEMA,
  )
})

test('bounds passphrases and plaintext before expensive encryption work', async () => {
  await assert.rejects(encryptConsumerPortabilityArchive(VALUE, 'too short'), TypeError)
  await assert.rejects(encryptConsumerPortabilityArchive(undefined, PASSPHRASE), TypeError)
  await assert.rejects(
    encryptConsumerPortabilityArchive('x'.repeat(
      CONSUMER_PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES,
    ), PASSPHRASE),
    /exceeds the supported size/,
  )
  const circular = {}
  circular.self = circular
  await assert.rejects(encryptConsumerPortabilityArchive(circular, PASSPHRASE), TypeError)
})

function legacyArchive(value, passphrase) {
  const plaintext = Buffer.from(JSON.stringify(value), 'utf8')
  const salt = randomBytes(16)
  const nonce = randomBytes(12)
  const key = pbkdf2Sync(
    Buffer.from(passphrase, 'utf8'),
    salt,
    CONSUMER_PORTABILITY_ARCHIVE_ITERATIONS,
    32,
    'sha256',
  )
  const cipher = createCipheriv('aes-256-gcm', key, nonce)
  cipher.setAAD(Buffer.from(CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1, 'utf8'))
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final(), cipher.getAuthTag()])
  return {
    schema: CONSUMER_PORTABILITY_ARCHIVE_SCHEMA_V1,
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
    created_at: '2026-08-10T08:00:00.000Z',
  }
}

function decryptV2Externally(archive, passphrase) {
  const salt = Buffer.from(archive.kdf.salt_base64, 'base64')
  const nonce = Buffer.from(archive.cipher.nonce_base64, 'base64')
  const combined = Buffer.from(archive.ciphertext_base64, 'base64')
  const ciphertext = combined.subarray(0, -16)
  const authTag = combined.subarray(-16)
  const key = pbkdf2Sync(
    Buffer.from(passphrase, 'utf8'),
    salt,
    archive.kdf.iterations,
    32,
    'sha256',
  )
  const decipher = createDecipheriv('aes-256-gcm', key, nonce)
  decipher.setAAD(Buffer.from([
    'open_commerce.consumer_portability_archive_aad.v2',
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
  ].join('\n'), 'utf8'))
  decipher.setAuthTag(authTag)
  return JSON.parse(Buffer.concat([decipher.update(ciphertext), decipher.final()]).toString('utf8'))
}
