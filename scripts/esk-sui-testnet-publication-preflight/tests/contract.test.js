'use strict'

const test = require('node:test')
const assert = require('node:assert/strict')
const {
  appendFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync,
} = require('node:fs')
const { tmpdir } = require('node:os')
const { join } = require('node:path')
const { canonicalJson, canonicalDigest } = require('../canonical')
const { parseStrictJson } = require('../strict-json')
const { createTemplate, preflightCandidate } = require('../index')
const {
  APPROVED_BASELINE_COMMIT, TOOLCHAIN_CONTRACT_SHA256, FIXED, loadAndVerifyRepository,
} = require('../repository')
const {
  clone, templateCandidate, syntheticCandidate, releaseCandidate, reverseObjectKeys,
} = require('./fixtures')

const REPO = join(__dirname, '../../..')
const CONTRACTS = join(REPO, 'contracts/sui')
const MAX_SCHEMA_BYTES = 64 * 1024

function withoutPlanDigest(plan) {
  const copy = clone(plan)
  delete copy.plan_sha256
  return copy
}

function copyRepositoryFixture() {
  const root = mkdtempSync(join(tmpdir(), 'esk-preflight-repository-'))
  mkdirSync(join(root, 'scripts/esk-sui-toolchain-ci'), { recursive: true })
  mkdirSync(join(root, 'contracts/sui'), { recursive: true })
  cpSync(join(REPO, 'scripts/esk-sui-toolchain-ci/toolchain-v1.json'),
    join(root, 'scripts/esk-sui-toolchain-ci/toolchain-v1.json'))
  for (const name of ['esk_currency', 'yilong_participation']) {
    cpSync(join(REPO, 'contracts/sui', name), join(root, 'contracts/sui', name),
      { recursive: true })
  }
  return root
}

test('candidate schema and checked-in template are strict JSON and runtime-identical', () => {
  const schemaBytes = readFileSync(
    join(CONTRACTS, 'esk-testnet-publication-candidate-v1.schema.json'))
  const preflightSchemaBytes = readFileSync(
    join(CONTRACTS, 'esk-testnet-publication-preflight-v1.schema.json'))
  const templateBytes = readFileSync(
    join(CONTRACTS, 'esk-testnet-publication-candidate-v1.template.json'))
  const schema = parseStrictJson(schemaBytes, MAX_SCHEMA_BYTES)
  const preflightSchema = parseStrictJson(preflightSchemaBytes, MAX_SCHEMA_BYTES)
  const artifact = parseStrictJson(templateBytes, MAX_SCHEMA_BYTES)
  assert.equal(schema.additionalProperties, false)
  assert.equal(preflightSchema.additionalProperties, false)
  assert.equal(canonicalJson(createTemplate()), canonicalJson(artifact))
  assert.equal(artifact.schema, 'yilong.esk.sui.testnet_publication_candidate.v1')
  assert.equal(artifact.scope.mode, 'template')
  assert.equal(artifact.scope.synthetic, false)
  assert.equal(artifact.commercial_policy_revision.status, 'clarification_required')
  assert.equal(artifact.commercial_policy_revision.intended_term_months, 24)
  assert.equal(artifact.commercial_policy_revision.public_sale_activation_allowed, false)
  assert.equal(artifact.commercial_policy_revision.technical_testnet_preflight_allowed, true)
})

test('canonical JSON recursively sorts ASCII object keys and preserves array order', () => {
  assert.equal(canonicalJson({ z: 1, a: { y: true, b: null }, items: [2, 1] }),
    '{"a":{"b":null,"y":true},"items":[2,1],"z":1}')
  assert.throws(() => canonicalJson({ '非ascii': 1 }), { code: 'INVALID_CANDIDATE' })
  assert.throws(() => canonicalJson({ value: 1.5 }), { code: 'INVALID_CANDIDATE' })
})

test('template, synthetic and complete release candidate have distinct inert states', () => {
  const template = preflightCandidate(templateCandidate())
  assert.equal(template.candidate_status, 'user_action_required')
  assert.ok(template.blocking_reasons.length > 0)
  assert.ok(template.user_actions_required.length > 0)

  const synthetic = preflightCandidate(syntheticCandidate())
  assert.equal(synthetic.candidate_status, 'synthetic_verified')
  assert.match(synthetic.next_safe_action, /synthetic|test/i)

  const release = preflightCandidate(releaseCandidate())
  assert.equal(release.candidate_status, 'prepared_not_authorized')
  assert.ok(release.blocking_reasons.length > 0,
    'execution remains blocked even when candidate parameters are complete')
  assert.ok(release.user_actions_required.length > 0,
    'a prepared candidate still requires separate execution authorization')
  assert.ok(release.blocking_reasons.includes(
    'APPROVAL_AUTHENTICITY_AND_SUBJECT_BINDING_UNVERIFIED'))
  assert.ok(release.blocking_reasons.includes(
    'CAPABILITY_HANDOFF_VERIFIER_IMPLEMENTATION_REQUIRED'))
  assert.ok(release.blocking_reasons.includes(
    'COMMERCIAL_POLICY_REVISION_REQUIRED_BEFORE_PUBLIC_SALE'))

  for (const plan of [template, synthetic, release]) {
    assert.equal(plan.schema, 'yilong.esk.sui.testnet_publication_preflight.v1')
    assert.equal(plan.plan_sha256, canonicalDigest(withoutPlanDigest(plan)))
  }
})

test('candidate key order and JSON whitespace do not change the plan digest', () => {
  const candidate = releaseCandidate()
  const reordered = reverseObjectKeys(candidate)
  const spaced = parseStrictJson(Buffer.from(JSON.stringify(reordered, null, 4)), 128 * 1024)
  const compact = parseStrictJson(Buffer.from(JSON.stringify(candidate)), 128 * 1024)
  assert.equal(preflightCandidate(spaced).plan_sha256, preflightCandidate(compact).plan_sha256)
})

test('a valid semantic parameter change changes the plan digest', () => {
  const original = releaseCandidate()
  const changed = releaseCandidate()
  changed.gas_budgets.currency_publish += 1
  assert.notEqual(preflightCandidate(original).plan_sha256,
    preflightCandidate(changed).plan_sha256)
})

test('fixed repository and package digests match the reviewed source baseline', () => {
  assert.equal(APPROVED_BASELINE_COMMIT, 'aebbfc41b910887725179bca46ceb2b0d793458f')
  assert.equal(TOOLCHAIN_CONTRACT_SHA256,
    'sha256:c7226dcb2e707a5c48d2f469b0b611296e5b65c1cc786dfd832b3b78e22065b6')
  assert.equal(FIXED.currency_production_bytecode_digest,
    'sha256:314273ecd53a54793c8b70f35e4a1e853fdc7c6751c20dc0baf0628907b03ca7')
  assert.equal(FIXED.participation_local_production_bytecode_digest,
    'sha256:fa691e2e7d7c1c347b8fd88a2dc9f3ca2590ee56813c0bb313ef2ea8d477d3ef')
  assert.deepEqual(loadAndVerifyRepository(REPO), {
    baseline_commit: APPROVED_BASELINE_COMMIT,
    toolchain_contract_sha256: TOOLCHAIN_CONTRACT_SHA256,
    repository_sources_verified: true,
    currency_package_input_digest: FIXED.currency_package_input_digest,
    participation_package_input_digest: FIXED.participation_package_input_digest,
  })
})

test('repository source drift fails closed before a plan can be prepared', () => {
  const root = copyRepositoryFixture()
  try {
    assert.equal(loadAndVerifyRepository(root).repository_sources_verified, true)
    appendFileSync(join(root, 'contracts/sui/esk_currency/sources/esk.move'), '\n// drift\n')
    assert.throws(() => loadAndVerifyRepository(root), { code: 'REPOSITORY_DRIFT' })
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('nested Move sources fail closed instead of escaping the fixed package inventory', () => {
  const root = copyRepositoryFixture()
  try {
    const nested = join(root, 'contracts/sui/esk_currency/sources/nested')
    mkdirSync(nested, { recursive: true })
    appendFileSync(join(nested, 'hidden.move'), '// unexpected nested source\n')
    assert.throws(() => loadAndVerifyRepository(root), { code: 'REPOSITORY_DRIFT' })
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('nested Move manifests and locks fail closed', () => {
  for (const manifest of ['Move.toml', 'Move.lock']) {
    const root = copyRepositoryFixture()
    try {
      const nested = join(root, 'contracts/sui/esk_currency/sources/nested')
      mkdirSync(nested, { recursive: true })
      appendFileSync(join(nested, manifest), '// unexpected nested package metadata\n')
      assert.throws(() => loadAndVerifyRepository(root), { code: 'REPOSITORY_DRIFT' })
    } finally { rmSync(root, { recursive: true, force: true }) }
  }
})

test('only the package-root build output is excluded from source inventory', () => {
  const root = copyRepositoryFixture()
  try {
    const rootBuild = join(root, 'contracts/sui/esk_currency/build/generated')
    mkdirSync(rootBuild, { recursive: true })
    appendFileSync(join(rootBuild, 'generated.move'), '// excluded build output\n')
    assert.equal(loadAndVerifyRepository(root).repository_sources_verified, true)

    const nestedBuild = join(root, 'contracts/sui/esk_currency/sources/build')
    mkdirSync(nestedBuild, { recursive: true })
    appendFileSync(join(nestedBuild, 'hidden.move'), '// build path inside sources\n')
    assert.throws(() => loadAndVerifyRepository(root), { code: 'REPOSITORY_DRIFT' })
  } finally { rmSync(root, { recursive: true, force: true }) }
})

test('symbolic-link or junction entries under Move source roots fail closed', (t) => {
  const root = copyRepositoryFixture()
  try {
    const external = join(root, 'external-sources')
    const linked = join(root, 'contracts/sui/esk_currency/sources/linked')
    mkdirSync(external, { recursive: true })
    appendFileSync(join(external, 'linked.move'), '// linked source\n')
    try {
      symlinkSync(external, linked, process.platform === 'win32' ? 'junction' : 'dir')
    } catch (error) {
      if (['EACCES', 'EINVAL', 'ENOTSUP', 'EPERM'].includes(error && error.code)) {
        t.skip(`symbolic links are unavailable on this host (${error.code})`)
        return
      }
      throw error
    }
    assert.throws(() => loadAndVerifyRepository(root), { code: 'REPOSITORY_DRIFT' })
  } finally { rmSync(root, { recursive: true, force: true }) }
})
