const test = require('node:test')
const assert = require('node:assert/strict')
const { spawnSync } = require('node:child_process')
const { join } = require('node:path')
const { mkdtempSync, writeFileSync, rmSync } = require('node:fs')
const { tmpdir } = require('node:os')
const { observeAllocation } = require('../observe')
const { AllocationObservationError } = require('../contract')
const { rawInput, observation } = require('./fixtures')

function replaceVestingAmounts(data, claimed, remaining) {
  const contents = data.vestingAtObservation.asMoveObject.contents
  const bytes = Buffer.from(contents.bcs, 'base64')
  bytes.writeBigUInt64LE(BigInt(claimed), 72)
  bytes.writeBigUInt64LE(BigInt(remaining), 104)
  contents.bcs = bytes.toString('base64')
}

test('two identical public sources produce observed evidence but no certification', async () => {
  const input = rawInput()
  let calls = 0
  const report = await observeAllocation(input, { read: async (url, expected) => {
    calls += 1
    assert.ok(input.endpoints.includes(url))
    return observation(expected)
  } })
  assert.equal(calls, 2)
  assert.equal(report.status, 'observed')
  assert.equal(report.allocation_observed, true)
  assert.equal(report.team_vesting_observed, true)
  assert.equal(report.error_code, null)
  assert.deepEqual(report.evidence, report.sources[0].evidence)
  assert.deepEqual(report.sources[0].evidence, report.sources[1].evidence)
  for (const field of [
    'publication_certified', 'source_verified', 'allocation_certified',
    'address_control_verified', 'finality_certified', 'asset_identity_verified',
    'balance_eligible', 'manifest_transition_allowed',
  ]) assert.equal(report[field], false, field)
  assert.equal(report.expected.endpoints, undefined)
  assert.equal(report.sources[0].source, 'official_testnet')
  assert.equal(report.sources[1].source, 'secondary')
  assert.match(report.sources[0].endpoint_sha256, /^[0-9a-f]{64}$/)
  assert.doesNotMatch(JSON.stringify(report), /reviewed-provider\.org|graphql\.testnet\.sui\.io/)
})

test('one failed source cannot fall back to the successful source', async () => {
  const input = rawInput()
  const report = await observeAllocation(input, { read: async (url, expected) => {
    if (url === input.endpoints[1]) throw new AllocationObservationError('CAP_MISMATCH')
    return observation(expected)
  } })
  assert.equal(report.status, 'unverified')
  assert.equal(report.error_code, 'CAP_MISMATCH')
  assert.equal(report.evidence, null)
  assert.equal(report.sources[0].status, 'observed')
  assert.equal(report.sources[1].status, 'unverified')
  assert.equal(report.sources[1].evidence, null)
})

test('the first failed source and two failed sources both remain unverified', async () => {
  const input = rawInput()
  const first = await observeAllocation(input, { read: async url => {
    if (url === input.endpoints[0]) throw new AllocationObservationError('BCS_MISMATCH')
    return observation(input)
  } })
  assert.equal(first.status, 'unverified')
  assert.equal(first.error_code, 'BCS_MISMATCH')
  assert.equal(first.sources[1].status, 'observed')

  const both = await observeAllocation(input, { read: async () => {
    throw new Error('network unavailable')
  } })
  assert.equal(both.status, 'unverified')
  assert.equal(both.error_code, 'NETWORK_ERROR')
  assert.ok(both.sources.every(source => source.status === 'unverified'))
})

test('valid but different snapshots fail with SOURCE_DISAGREEMENT', async () => {
  const input = rawInput()
  const report = await observeAllocation(input, { read: async (url, expected) => {
    const data = observation(expected)
    if (url === input.endpoints[1]) replaceVestingAmounts(data, '60', '140')
    return data
  } })
  assert.equal(report.status, 'unverified')
  assert.equal(report.error_code, 'SOURCE_DISAGREEMENT')
  assert.equal(report.evidence, null)
  assert.equal(report.sources[0].status, 'observed')
  assert.equal(report.sources[1].status, 'observed')
})

test('invalid input and unknown failures remain bounded and do not perform extra reads', async () => {
  let calls = 0
  const invalid = await observeAllocation({ private_key: 'do-not-read' }, {
    read: async () => { calls += 1 },
  })
  assert.equal(calls, 0)
  assert.equal(invalid.status, 'unverified')
  assert.equal(invalid.error_code, 'INVALID_INPUT')
  assert.equal(invalid.expected, null)
  assert.doesNotMatch(JSON.stringify(invalid), /do-not-read/)

  const network = await observeAllocation(rawInput(), { read: async () => {
    throw new Error('https://secret.example/graphql?api_key=secret')
  } })
  assert.equal(network.error_code, 'NETWORK_ERROR')
  assert.doesNotMatch(JSON.stringify(network), /secret\.example|api_key/)
})

test('CLI help and malformed public input are machine-readable and make no network request', () => {
  const cli = join(__dirname, '../../observe-esk-sui-allocation.js')
  const help = spawnSync(process.execPath, [cli, '--help'], { encoding: 'utf8', timeout: 10000 })
  assert.equal(help.status, 0)
  assert.match(help.stdout, /Read-only.*no wallet, signing, broadcast or balance update/i)

  const missing = spawnSync(process.execPath, [cli], { encoding: 'utf8', timeout: 10000 })
  assert.equal(missing.status, 1)
  assert.equal(JSON.parse(missing.stdout).error_code, 'INVALID_INPUT')

  const directory = mkdtempSync(join(tmpdir(), 'esk-allocation-observer-'))
  try {
    const path = join(directory, 'invalid.json')
    writeFileSync(path, JSON.stringify({ private_key: 'never echo' }))
    const invalid = spawnSync(process.execPath, [cli, path], {
      encoding: 'utf8', timeout: 10000,
    })
    assert.equal(invalid.status, 1)
    assert.equal(JSON.parse(invalid.stdout).error_code, 'INVALID_INPUT')
    assert.doesNotMatch(invalid.stdout, /never echo|private_key/)

    const badUtf8Path = join(directory, 'bad-utf8.json')
    writeFileSync(badUtf8Path, Buffer.from([0xff, 0xfe, 0xfd]))
    const oversizedPath = join(directory, 'oversized.json')
    writeFileSync(oversizedPath, Buffer.alloc((64 * 1024) + 1, 0x20))
    for (const candidate of [directory, badUtf8Path, oversizedPath]) {
      const rejected = spawnSync(process.execPath, [cli, candidate], {
        encoding: 'utf8', timeout: 10000,
      })
      assert.equal(rejected.status, 1)
      assert.equal(JSON.parse(rejected.stdout).error_code, 'INVALID_INPUT')
    }

    const extra = spawnSync(process.execPath, [cli, path, 'unexpected'], {
      encoding: 'utf8', timeout: 10000,
    })
    assert.equal(extra.status, 1)
    assert.equal(JSON.parse(extra.stdout).error_code, 'INVALID_INPUT')
  } finally { rmSync(directory, { recursive: true, force: true }) }
})
