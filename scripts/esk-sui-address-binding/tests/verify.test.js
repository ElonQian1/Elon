const test = require('node:test')
const assert = require('node:assert/strict')
require('./no-network-guard')
const { assemble, createChallenge } = require('../challenge')
const { verifyAddressControl, reverifyEvidence } = require('../verify')
const { request } = require('./fixtures')

async function keypairs() {
  const [{ Ed25519Keypair }, { Secp256k1Keypair }, { Secp256r1Keypair }] = await Promise.all([
    import('@mysten/sui/keypairs/ed25519'),
    import('@mysten/sui/keypairs/secp256k1'),
    import('@mysten/sui/keypairs/secp256r1'),
  ])
  return [new Ed25519Keypair(), new Secp256k1Keypair(), new Secp256r1Keypair()]
}

async function signedFixture(keypair) {
  const challenge = createChallenge(request(keypair.toSuiAddress()))
  const message = Buffer.from(challenge.message_base64, 'base64')
  const signed = await keypair.signPersonalMessage(message)
  return {
    challenge,
    response: {
      schema: 'yilong.esk.sui.address_binding_wallet_response.v1',
      challenge_id: challenge.challenge_id,
      message_base64: signed.bytes,
      signature: signed.signature,
    },
  }
}

test('all three supported Sui single-key schemes verify offline', async () => {
  const expected = ['ed25519', 'secp256k1', 'secp256r1']
  const pairs = await keypairs()
  for (let index = 0; index < pairs.length; index += 1) {
    const { challenge, response } = await signedFixture(pairs[index])
    const evidence = await verifyAddressControl(challenge, response)
    assert.equal(evidence.signature_scheme, expected[index])
    assert.equal(evidence.address, challenge.address)
    assert.equal(evidence.address_control_verified, true)
    for (const key of [
      'platform_subject_authenticated', 'challenge_single_use_recorded',
      'chain_finality_verified', 'asset_identity_verified', 'balance_eligible',
      'manifest_transition_allowed',
    ]) assert.equal(evidence[key], false, key)
    assert.deepEqual(await reverifyEvidence(evidence), evidence)
  }
})

test('wrong address, message, signature and transaction intent fail closed', async () => {
  const [first, second] = await keypairs()
  const { challenge, response } = await signedFixture(first)
  const wrongAddress = { ...challenge, address: second.toSuiAddress() }
  await assert.rejects(() => verifyAddressControl(wrongAddress, response))
  const wrongMessage = { ...response, message_base64: Buffer.from('wrong').toString('base64') }
  await assert.rejects(() => verifyAddressControl(challenge, wrongMessage))
  const wrongChallengeId = { ...response, challenge_id: `eab1_${'f'.repeat(32)}` }
  await assert.rejects(() => verifyAddressControl(challenge, wrongChallengeId),
    /CHALLENGE_ID_MISMATCH/)
  const wrongSignature = { ...response, signature: (await second.signPersonalMessage(
    Buffer.from(challenge.message_base64, 'base64'))).signature }
  await assert.rejects(() => verifyAddressControl(challenge, wrongSignature))
  const transactionSignature = { ...response, signature: (await first.signTransaction(
    Buffer.from(challenge.message_base64, 'base64'))).signature }
  await assert.rejects(() => verifyAddressControl(challenge, transactionSignature))
})

test('expired and not-yet-valid challenges fail closed', async () => {
  const [keypair] = await keypairs()
  const now = Date.now()
  async function signWindow(issuedAt, expiresAt) {
    const challenge = assemble({
      ...request(keypair.toSuiAddress()),
      nonce_base64: Buffer.alloc(32, 4).toString('base64'),
      issued_at: new Date(issuedAt).toISOString(),
      expires_at: new Date(expiresAt).toISOString(),
      ttl_seconds: 120,
    })
    const signed = await keypair.signPersonalMessage(Buffer.from(challenge.message_base64, 'base64'))
    return { challenge, response: {
      schema: 'yilong.esk.sui.address_binding_wallet_response.v1',
      challenge_id: challenge.challenge_id,
      message_base64: signed.bytes,
      signature: signed.signature,
    } }
  }
  const expired = await signWindow(now - 121000, now - 1000)
  const future = await signWindow(now + 1000, now + 121000)
  await assert.rejects(() => verifyAddressControl(expired.challenge, expired.response),
    /CHALLENGE_EXPIRED/)
  await assert.rejects(() => verifyAddressControl(future.challenge, future.response),
    /CHALLENGE_NOT_YET_VALID/)
})

test('issued_at is inclusive and expires_at is exclusive', async () => {
  const [keypair] = await keypairs()
  const { challenge, response } = await signedFixture(keypair)
  const originalNow = Date.now
  try {
    Date.now = () => Date.parse(challenge.issued_at)
    assert.equal((await verifyAddressControl(challenge, response)).address_control_verified, true)
    Date.now = () => Date.parse(challenge.expires_at)
    await assert.rejects(() => verifyAddressControl(challenge, response), /CHALLENGE_EXPIRED/)
  } finally {
    Date.now = originalNow
  }
})

test('network-dependent and composite schemes are rejected before SDK verification', async () => {
  const [keypair] = await keypairs()
  const { challenge, response } = await signedFixture(keypair)
  for (const flag of [3, 4, 5, 6, 255]) {
    const invalid = { ...response, signature: Buffer.from([flag, 0, 0]).toString('base64') }
    await assert.rejects(() => verifyAddressControl(challenge, invalid),
      /UNSUPPORTED_SIGNATURE_SCHEME/)
  }
})

test('evidence drift cannot be silently reverified', async () => {
  const [keypair] = await keypairs()
  const { challenge, response } = await signedFixture(keypair)
  const evidence = await verifyAddressControl(challenge, response)
  for (const mutation of [
    { ...evidence, address_control_verified: false },
    { ...evidence, balance_eligible: true },
    { ...evidence, verified_at: new Date(Date.parse(evidence.verified_at) + 1).toISOString() },
    { ...evidence, signature_sha256: `sha256:${'f'.repeat(64)}` },
  ]) await assert.rejects(() => reverifyEvidence(mutation))
})

test('evidence recheck uses the current clock, never an untrusted stored verification time', async () => {
  const [keypair] = await keypairs()
  const { challenge, response } = await signedFixture(keypair)
  const evidence = await verifyAddressControl(challenge, response)
  const originalNow = Date.now
  try {
    Date.now = () => Date.parse(challenge.expires_at)
    await assert.rejects(() => reverifyEvidence(evidence), /CHALLENGE_EXPIRED/)
  } finally {
    Date.now = originalNow
  }
})
