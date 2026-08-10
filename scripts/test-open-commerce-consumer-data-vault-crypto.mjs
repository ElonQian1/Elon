import assert from 'node:assert/strict'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const vault = await import(pathToFileURL(resolve(
  root,
  'pc-frontend/src/features/open-commerce/consumerDataVaultCrypto.js',
)).href)

const recordId = 'vault_record_1234'
const passphrase = 'correct horse battery staple'
const content = '偏好：安静的咖啡店\n预算：¥88\nemoji: ☕'

const envelope = await vault.encryptConsumerDataVaultItem(recordId, 1, content, passphrase)
assert.equal(envelope.schema, vault.CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA)
assert.equal(envelope.kdf.iterations, 310_000)
assert.equal(await vault.decryptConsumerDataVaultItem(envelope, passphrase), content)

const second = await vault.encryptConsumerDataVaultItem(recordId, 1, content, passphrase)
assert.notEqual(second.kdf.salt_base64, envelope.kdf.salt_base64)
assert.notEqual(second.cipher.nonce_base64, envelope.cipher.nonce_base64)
assert.notEqual(second.ciphertext_base64, envelope.ciphertext_base64)

await assert.rejects(
  vault.decryptConsumerDataVaultItem(envelope, 'wrong password value'),
  /解密或认证失败/,
)
await assert.rejects(
  vault.decryptConsumerDataVaultItem({ ...envelope, record_id: 'vault_record_5678' }, passphrase),
  /解密或认证失败/,
)
await assert.rejects(
  vault.decryptConsumerDataVaultItem({ ...envelope, revision: 2 }, passphrase),
  /解密或认证失败/,
)

const tamperedBytes = Buffer.from(envelope.ciphertext_base64, 'base64')
tamperedBytes[0] ^= 1
await assert.rejects(
  vault.decryptConsumerDataVaultItem({
    ...envelope,
    ciphertext_base64: tamperedBytes.toString('base64'),
  }, passphrase),
  /解密或认证失败/,
)

for (const invalid of [
  { ...envelope, created_at: '2026-02-31T10:00:00Z' },
  { ...envelope, kdf: { ...envelope.kdf, iterations: 309_999 } },
  { ...envelope, kdf: { ...envelope.kdf, salt_base64: ` ${envelope.kdf.salt_base64}` } },
  { ...envelope, cipher: { ...envelope.cipher, nonce_base64: `${envelope.cipher.nonce_base64}=` } },
  { ...envelope, ciphertext_base64: Buffer.alloc(16).toString('base64') },
  { ...envelope, ciphertext_base64: Buffer.alloc(1024 * 1024 + 1).toString('base64') },
]) {
  await assert.rejects(vault.decryptConsumerDataVaultItem(invalid, passphrase), /不支持/)
}

await assert.rejects(
  vault.encryptConsumerDataVaultItem(recordId, 1, { secret: true }, passphrase),
  /必须为文本/,
)
await assert.rejects(
  vault.encryptConsumerDataVaultItem(recordId, 1, 'x'.repeat(900 * 1024), passphrase),
  /超过 900 KiB/,
)
await assert.rejects(
  vault.encryptConsumerDataVaultItem(recordId, 1, content, 'too short'),
  /12 到 256/,
)
await assert.rejects(
  vault.encryptConsumerDataVaultItem(recordId, 1, content, 'x'.repeat(257)),
  /12 到 256/,
)

console.log('open commerce consumer data vault crypto: ok')
