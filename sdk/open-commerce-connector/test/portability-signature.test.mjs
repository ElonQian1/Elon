import assert from 'node:assert/strict'
import { generateKeyPairSync } from 'node:crypto'
import test from 'node:test'

import {
  CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM,
  CONSUMER_PORTABILITY_SIGNATURE_SCHEMA,
  consumerPortabilityPublicKeyId,
  consumerPortabilitySignatureMessage,
  signConsumerPortabilityPackage,
  verifyConsumerPortabilityPackageSignature,
} from '../src/index.js'
import {
  portabilityPackage,
  primaryKeys,
  secondaryKeys,
  weakRsaKeys,
} from './portability-signature-fixtures.mjs'

const SOURCE_OPERATOR = 'fixture.operator'

test('signs and verifies the fixed consumer portability identity fields', () => {
  const value = portabilityPackage()
  const signed = signConsumerPortabilityPackage({
    sourceOperator: SOURCE_OPERATOR,
    privateKey: primaryKeys.privateKey,
    package: value,
  })

  assert.equal(signed.schema, CONSUMER_PORTABILITY_SIGNATURE_SCHEMA)
  assert.equal(signed.signature.algorithm, CONSUMER_PORTABILITY_SIGNATURE_ALGORITHM)
  assert.equal(signed.signature.key_id, consumerPortabilityPublicKeyId(primaryKeys.publicKey))
  assert.equal(signed.package, value)
  assert.equal(verifyConsumerPortabilityPackageSignature({
    publicKey: primaryKeys.publicKey,
    signedPackage: signed,
  }), true)
})

test('uses the server-compatible newline-delimited signature message', () => {
  const value = portabilityPackage()
  const keyId = consumerPortabilityPublicKeyId(primaryKeys.publicKey)
  assert.equal(consumerPortabilitySignatureMessage({
    sourceOperator: SOURCE_OPERATOR,
    keyId,
    package: value,
  }), [
    'open_commerce.consumer_portability_signature.v1',
    SOURCE_OPERATOR,
    keyId,
    value.schema,
    value.id,
    value.source_project_id,
    value.idempotency_key,
    value.payload_sha256,
    value.created_at,
  ].join('\n'))
})

test('rejects mutation of every field bound by the signature', () => {
  const signed = signConsumerPortabilityPackage({
    sourceOperator: SOURCE_OPERATOR,
    privateKey: primaryKeys.privateKey,
    package: portabilityPackage(),
  })
  const mutations = {
    schema: 'open_commerce.consumer_portability_export.v4',
    id: 'portability-fixture-2',
    source_project_id: 'consumer-project-other',
    idempotency_key: 'portable:fixture:2',
    payload_sha256: 'b'.repeat(64),
    created_at: '2026-08-10T08:01:00.000Z',
  }
  for (const [field, value] of Object.entries(mutations)) {
    const tampered = structuredClone(signed)
    tampered.package[field] = value
    assert.equal(verifyConsumerPortabilityPackageSignature({
      publicKey: primaryKeys.publicKey,
      signedPackage: tampered,
    }), false, field)
  }
  const tamperedOperator = structuredClone(signed)
  tamperedOperator.source_operator = 'other.operator'
  assert.equal(verifyConsumerPortabilityPackageSignature({
    publicKey: primaryKeys.publicKey,
    signedPackage: tamperedOperator,
  }), false)
})

test('fails closed for key, algorithm, identifier, and Base64 substitution', () => {
  const signed = signConsumerPortabilityPackage({
    sourceOperator: SOURCE_OPERATOR,
    privateKey: primaryKeys.privateKey,
    package: portabilityPackage(),
  })
  const cases = [
    { publicKey: secondaryKeys.publicKey, signedPackage: signed },
    { publicKey: primaryKeys.publicKey, signedPackage: { ...signed, schema: 'wrong' } },
    {
      publicKey: primaryKeys.publicKey,
      signedPackage: { ...signed, signature: { ...signed.signature, algorithm: 'wrong' } },
    },
    {
      publicKey: primaryKeys.publicKey,
      signedPackage: { ...signed, signature: { ...signed.signature, key_id: 'b'.repeat(64) } },
    },
    {
      publicKey: primaryKeys.publicKey,
      signedPackage: { ...signed, signature: { ...signed.signature, signature_base64: 'AA=A' } },
    },
    { publicKey: primaryKeys.publicKey, signedPackage: null },
  ]
  for (const value of cases) {
    assert.doesNotThrow(() => {
      assert.equal(verifyConsumerPortabilityPackageSignature(value), false)
    })
  }
})

test('rejects ambiguous signed fields and RSA keys outside the trust policy', () => {
  const weak = weakRsaKeys()
  assert.throws(() => consumerPortabilityPublicKeyId(weak.publicKey), /2048-8192 bit RSA/)
  assert.throws(() => signConsumerPortabilityPackage({
    sourceOperator: SOURCE_OPERATOR,
    privateKey: weak.privateKey,
    package: portabilityPackage(),
  }), /2048-8192 bit RSA/)
  assert.throws(() => signConsumerPortabilityPackage({
    sourceOperator: SOURCE_OPERATOR,
    privateKey: primaryKeys.privateKey,
    package: portabilityPackage({ id: 'line\nbreak' }),
  }), /package.id/)

  const ed25519 = generateKeyPairSync('ed25519')
  assert.throws(() => consumerPortabilityPublicKeyId(ed25519.publicKey), /RSA key/)
})
