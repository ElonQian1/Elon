import assert from 'node:assert/strict'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const pc = await import(pathToFileURL(resolve(
  root,
  'pc-frontend/src/features/open-commerce/portabilityArchive.js',
)).href)
const sdk = await import(pathToFileURL(resolve(
  root,
  'sdk/open-commerce-connector/src/index.js',
)).href)

const passphrase = 'cross runtime archive passphrase'
const value = { consumer: '跨端用户', preferences: ['coffee', 'quiet'], revision: 7 }

assert.equal(pc.PORTABILITY_ARCHIVE_SCHEMA, sdk.CONSUMER_PORTABILITY_ARCHIVE_SCHEMA)
const pcArchive = await pc.encryptPortabilityArchive(value, passphrase)
assert.deepEqual(await sdk.decryptConsumerPortabilityArchive(pcArchive, passphrase), value)

const sdkArchive = await sdk.encryptConsumerPortabilityArchive(value, passphrase)
assert.deepEqual(await pc.decryptPortabilityArchive(sdkArchive, passphrase), value)

const tampered = { ...pcArchive, created_at: '2026-08-10T09:00:00.000Z' }
await assert.rejects(pc.decryptPortabilityArchive(tampered, passphrase), /认证失败/)
await assert.rejects(sdk.decryptConsumerPortabilityArchive(tampered, passphrase), /authentication failed/)

console.log('open commerce portability crypto interop: ok')
