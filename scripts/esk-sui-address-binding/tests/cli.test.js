const test = require('node:test')
const assert = require('node:assert/strict')
const { mkdtempSync, writeFileSync, rmSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { spawnSync } = require('node:child_process')
const { localPath } = require('../cli')

const CLI = join(__dirname, '../../prepare-esk-sui-address-binding.js')
const NETWORK_GUARD = join(__dirname, 'no-network-guard.js')
const ADDRESS = `0x${'d'.repeat(64)}`

function spawnCli(args) {
  return spawnSync(process.execPath, ['--require', NETWORK_GUARD, CLI, ...args], {
    encoding: 'utf8', timeout: 5000, env: {},
  })
}

function writeRequest(path, address = ADDRESS) {
  writeFileSync(path, JSON.stringify({
    schema: 'yilong.esk.sui.address_binding_challenge_request.v1',
    network: 'testnet', purpose: 'user_asset_migration',
    subject_commitment: `sha256:${'a'.repeat(64)}`, address, ttl_seconds: 120,
  }))
}

test('CLI help is explicit and does not require files', () => {
  const result = spawnCli(['--help'])
  assert.equal(result.status, 0)
  assert.match(result.stdout, /offline/i)
  assert.match(result.stdout, /does not bind a platform account/i)
})

test('CLI challenge emits strict machine JSON and no prose', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-address-cli-'))
  try {
    const path = join(dir, 'request.json')
    writeRequest(path)
    const result = spawnCli(['challenge', path])
    assert.equal(result.status, 0, result.stderr)
    const output = JSON.parse(result.stdout)
    assert.equal(output.schema, 'yilong.esk.sui.address_binding_challenge.v1')
    assert.equal(result.stderr, '')
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI verifies a real Sui personal-message response end to end', async () => {
  const { Ed25519Keypair } = await import('@mysten/sui/keypairs/ed25519')
  const keypair = new Ed25519Keypair()
  const dir = mkdtempSync(join(tmpdir(), 'esk-address-cli-'))
  try {
    const requestPath = join(dir, 'request.json')
    const challengePath = join(dir, 'challenge.json')
    const responsePath = join(dir, 'response.json')
    writeRequest(requestPath, keypair.toSuiAddress())
    const challengeResult = spawnCli(['challenge', requestPath])
    assert.equal(challengeResult.status, 0, challengeResult.stderr)
    const challenge = JSON.parse(challengeResult.stdout)
    writeFileSync(challengePath, challengeResult.stdout)
    const signed = await keypair.signPersonalMessage(
      Buffer.from(challenge.message_base64, 'base64'))
    writeFileSync(responsePath, JSON.stringify({
      schema: 'yilong.esk.sui.address_binding_wallet_response.v1',
      challenge_id: challenge.challenge_id,
      message_base64: signed.bytes,
      signature: signed.signature,
    }))
    const result = spawnCli(['verify', challengePath, responsePath])
    assert.equal(result.status, 0, result.stderr)
    const evidence = JSON.parse(result.stdout)
    assert.equal(evidence.address_control_verified, true)
    assert.equal(evidence.platform_subject_authenticated, false)
    assert.equal(evidence.challenge_single_use_recorded, false)
    assert.equal(result.stderr, '')
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects oversized and secret-bearing invalid input without echoing it', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-address-cli-'))
  try {
    const path = join(dir, 'secret-input.json')
    writeFileSync(path, JSON.stringify({ secret: 'do-not-print', padding: 'x'.repeat(65536) }))
    const result = spawnCli(['challenge', path])
    assert.equal(result.status, 1)
    assert.match(result.stderr, /^ESK_SUI_ADDRESS_BINDING_ERROR=[A-Z_]+\r?\n$/)
    assert.equal((result.stdout + result.stderr).includes('do-not-print'), false)
    assert.equal((result.stdout + result.stderr).includes(path), false)
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI permits the 64 KiB file boundary before strict content validation', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-address-cli-'))
  try {
    const path = join(dir, 'boundary.json')
    writeFileSync(path, `"${'x'.repeat(65534)}"`)
    const result = spawnCli(['challenge', path])
    assert.equal(result.status, 1)
    assert.match(result.stderr, /ESK_SUI_ADDRESS_BINDING_ERROR=INVALID_INPUT/)
    assert.doesNotMatch(result.stderr, /FILE_TOO_LARGE/)
  } finally { rmSync(dir, { recursive: true, force: true }) }
})

test('CLI rejects every unsupported command and file count', () => {
  for (const args of [[], ['unknown'], ['challenge'], ['challenge', 'a', 'b'], ['verify', 'a']]) {
    const result = spawnCli(args)
    assert.equal(result.status, 1)
    assert.match(result.stderr, /ESK_SUI_ADDRESS_BINDING_ERROR=USAGE/)
  }
})

test('CLI rejects UNC and device namespace paths before file access', () => {
  for (const path of [
    '\\\\example.invalid\\share\\secret.json',
    '//example.invalid/share/secret.json',
    '\\\\?\\UNC\\example.invalid\\share\\secret.json',
    '\\\\.\\pipe\\secret',
  ]) assert.throws(() => localPath(path), /INVALID_INPUT/)
})

test('CLI rejects duplicate protocol keys before last-wins parsing', () => {
  const dir = mkdtempSync(join(tmpdir(), 'esk-address-cli-'))
  try {
    const path = join(dir, 'duplicate.json')
    writeFileSync(path, [
      '{"schema":"yilong.esk.sui.address_binding_challenge_request.v1",',
      '"network":"mainnet","\\u006eetwork":"testnet",',
      '"purpose":"user_asset_migration",',
      `"subject_commitment":"sha256:${'a'.repeat(64)}",`,
      `"address":"${ADDRESS}","ttl_seconds":120}`,
    ].join(''))
    const result = spawnCli(['challenge', path])
    assert.equal(result.status, 1)
    assert.match(result.stderr, /ESK_SUI_ADDRESS_BINDING_ERROR=INVALID_INPUT/)
  } finally { rmSync(dir, { recursive: true, force: true }) }
})
