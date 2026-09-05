const test = require('node:test')
const assert = require('node:assert/strict')
const { readFileSync, readdirSync } = require('node:fs')
const { join } = require('node:path')

const ROOT = join(__dirname, '..')
const REPO = join(ROOT, '../..')

test('official SDK version and npm integrity are pinned to public registry artifacts', () => {
  const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8'))
  const lock = JSON.parse(readFileSync(join(ROOT, 'package-lock.json'), 'utf8'))
  const npmrc = readFileSync(join(ROOT, '.npmrc'), 'utf8')
  assert.equal(pkg.dependencies['@mysten/sui'], '2.29.0')
  assert.match(npmrc, /^registry=https:\/\/registry\.npmjs\.org\/$/m)
  assert.match(npmrc, /^ignore-scripts=true$/m)
  assert.equal(lock.packages['node_modules/@mysten/sui'].version, '2.29.0')
  assert.equal(lock.packages['node_modules/@mysten/sui'].integrity,
    'sha512-k7q22+AFQ5SZXOH+a28M1J8iFVbMcWro9mt0Bb7GI1HZNsxJIyQT5Q3iZdrAM0hTwM4XQvzm5Y0f32f0nfiGxw==')
  for (const [path, entry] of Object.entries(lock.packages)) {
    if (!path) continue
    assert.match(entry.resolved, /^https:\/\/registry\.npmjs\.org\//)
    assert.match(entry.integrity, /^sha512-/)
  }
})

test('runtime source contains no wallet, signer, RPC, network, process or environment access', () => {
  const files = readdirSync(ROOT).filter(name => name.endsWith('.js'))
  assert.ok(files.length >= 4)
  const source = files.map(name => readFileSync(join(ROOT, name), 'utf8')).join('\n')
  for (const forbidden of [
    /@mysten\/sui\/(client|graphql|grpc|jsonRpc)/,
    /\b(fetch|XMLHttpRequest|WebSocket|axios)\b/,
    /\b(SuiClient|SuiGrpcClient|Transaction)\b/,
    /\b(signPersonalMessage|signTransaction|signAndExecuteTransaction)\b/,
    /\b(decodeSuiPrivateKey|fromSecretKey|getSecretKey)\b/,
    /\bprocess\.env\b/,
    /node:child_process/,
  ]) assert.doesNotMatch(source, forbidden)
  assert.doesNotMatch(source, /readFileSync/)
  assert.match(source, /Buffer\.alloc\(MAX_FILE_BYTES \+ 1\)/)
  assert.match(source, /sameSnapshot\(opened, finished\)/)
})

test('versioned schemas are strict, bounded and keep every promotion flag false', async () => {
  const challenge = JSON.parse(readFileSync(
    join(REPO, 'contracts/sui/esk-address-binding-challenge-v1.schema.json'), 'utf8'))
  const evidence = JSON.parse(readFileSync(
    join(REPO, 'contracts/sui/esk-address-control-evidence-v1.schema.json'), 'utf8'))
  assert.equal(challenge.additionalProperties, false)
  assert.equal(challenge.properties.network.const, 'testnet')
  assert.equal(challenge.properties.purpose.const, 'user_asset_migration')
  assert.equal(challenge.properties.issued_at.format, 'date-time')
  assert.equal(challenge.properties.expires_at.format, 'date-time')
  assert.match(Buffer.alloc(32, 7).toString('base64'),
    new RegExp(challenge.properties.nonce_base64.pattern))
  assert.doesNotMatch('A'.repeat(42) + 'B=',
    new RegExp(challenge.properties.nonce_base64.pattern))
  assert.doesNotMatch('AB==', new RegExp(challenge.properties.message_base64.pattern))
  assert.equal(evidence.additionalProperties, false)
  assert.equal(evidence.properties.challenge.$ref, '#/$defs/challenge')
  for (const key of ['type', 'additionalProperties', 'required', 'properties']) {
    assert.deepEqual(evidence.$defs.challenge[key], challenge[key])
  }
  assert.equal(evidence.properties.verified_at.format, 'date-time')
  const signatureSchema = evidence.properties.wallet_response.properties.signature
  assert.ok('AA=='.length < signatureSchema.minLength)
  const [{ Ed25519Keypair }, { Secp256k1Keypair }, { Secp256r1Keypair }] = await Promise.all([
    import('@mysten/sui/keypairs/ed25519'),
    import('@mysten/sui/keypairs/secp256k1'),
    import('@mysten/sui/keypairs/secp256r1'),
  ])
  for (const keypair of [new Ed25519Keypair(), new Secp256k1Keypair(), new Secp256r1Keypair()]) {
    const { signature } = await keypair.signPersonalMessage(Buffer.from('schema-vector'))
    assert.ok(signature.length >= signatureSchema.minLength)
    assert.ok(signature.length <= signatureSchema.maxLength)
    assert.match(signature, new RegExp(signatureSchema.pattern))
  }
  assert.equal(evidence.properties.address_control_verified.const, true)
  for (const key of [
    'platform_subject_authenticated', 'challenge_single_use_recorded',
    'chain_finality_verified', 'asset_identity_verified', 'balance_eligible',
    'manifest_transition_allowed',
  ]) assert.equal(evidence.properties[key].const, false, key)
})
