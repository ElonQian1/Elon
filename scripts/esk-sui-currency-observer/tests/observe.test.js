const test = require('node:test')
const assert = require('node:assert/strict')
const { spawnSync } = require('node:child_process')
const { join } = require('node:path')
const { observeCurrency } = require('../observe')
const { CurrencyObservationError } = require('../contract')
const { ObservationError } = require('../../esk-sui-publication-observer/contract')
const { input, rawInput, observation } = require('./fixtures')

function flags(report) {
  for (const key of ['publication_certified', 'asset_identity_verified', 'balance_eligible',
    'manifest_transition_allowed']) assert.equal(report[key], false)
  assert.equal(report.trust_basis, 'rpc_reports_not_committee_signature_verification')
  assert.ok(!JSON.stringify(report).includes('https://'))
}

test('two agreeing sources observe canonical registration but never certify or enable balances', async () => {
  const seen = []
  const report = await observeCurrency(rawInput(), { read: async (url, expected) => {
    seen.push(url)
    assert.equal(expected.currency_address, input().currency_address)
    assert.equal(expected.coin_type, input().coin_type)
    return observation()
  } })
  assert.equal(report.status, 'observed')
  assert.equal(report.schema, 'yilong.esk.sui.currency_observation.v1')
  assert.equal(seen.length, 2)
  assert.equal(report.sources.length, 2)
  assert.notEqual(report.sources[0].endpoint_sha256, report.sources[1].endpoint_sha256)
  assert.deepEqual(report.evidence, report.sources[0].evidence)
  assert.equal(report.error_code, null)
  flags(report)
})

test('invalid input performs no reads and projects no expected values or injected secrets', async () => {
  let reads = 0
  for (const value of [null, { ...rawInput(), api_key: 'secret' },
    { ...rawInput(), endpoints: [rawInput().endpoints[0], 'https://user:secret@second.sui.io'] }]) {
    const report = await observeCurrency(value, { read: async () => { reads++; return observation() } })
    assert.equal(report.status, 'unverified')
    assert.equal(report.expected, null)
    assert.deepEqual(report.sources, [])
    assert.ok(!JSON.stringify(report).includes('secret'))
    flags(report)
  }
  assert.equal(reads, 0)
})

for (const [label, error, code] of [
  ['network', new Error('secret upstream error'), 'NETWORK_ERROR'],
  ['timeout', new ObservationError('TIMEOUT'), 'TIMEOUT'],
  ['supply', new CurrencyObservationError('SUPPLY_MISMATCH'), 'SUPPLY_MISMATCH'],
]) test(`one ${label} failure cannot fall back to the successful source`, async () => {
  const report = await observeCurrency(rawInput(), { read: async url => {
    if (url === rawInput().endpoints[1]) throw error
    return observation()
  } })
  assert.equal(report.status, 'unverified')
  assert.equal(report.evidence, null)
  assert.equal(report.error_code, code)
  assert.equal(report.sources[0].status, 'observed')
  assert.equal(report.sources[1].status, 'unverified')
  assert.ok(!JSON.stringify(report).includes('secret'))
  flags(report)
})

test('two valid but different latest versions remain source disagreement', async () => {
  const report = await observeCurrency(rawInput(), { read: async url => {
    const data = observation()
    if (url === rawInput().endpoints[1]) data.currentMetadata.version += 1
    return data
  } })
  assert.equal(report.status, 'unverified')
  assert.equal(report.error_code, 'SOURCE_DISAGREEMENT')
  assert.equal(report.evidence, null)
  assert.ok(report.sources.every(source => source.status === 'observed'))
  flags(report)
})

test('CLI help and malformed arguments remain no-network and redact secret input', () => {
  const cli = join(__dirname, '../../observe-esk-sui-currency.js')
  const help = spawnSync(process.execPath, [cli, '--help'], { encoding: 'utf8', timeout: 5000 })
  assert.equal(help.status, 0)
  assert.match(help.stdout, /Read-only testnet/)
  const args = [rawInput().chain_identifier, rawInput().package_id,
    rawInput().publication_digest, rawInput().registration_digest,
    rawInput().registration_version, rawInput().expected_supply_base_units,
    'https://user:secret@second.sui.io/graphql']
  for (const invalid of [[], ['secret'], args]) {
    const result = spawnSync(process.execPath, [cli, ...invalid], { encoding: 'utf8', timeout: 5000 })
    assert.equal(result.status, 1)
    assert.ok(!result.stdout.includes('secret'))
    assert.equal(result.stderr, '')
    const report = JSON.parse(result.stdout)
    assert.equal(report.status, 'unverified')
    assert.deepEqual(report.sources, [])
    flags(report)
  }
})

for (const status of ['observed', 'unverified']) {
  test(`legal CLI arguments map exactly and ${status} has the correct process exit code`, () => {
    const cli = join(__dirname, '../../observe-esk-sui-currency.js')
    const observer = require.resolve('../observe')
    const expected = rawInput()
    const args = [expected.chain_identifier, expected.package_id, expected.publication_digest,
      expected.registration_digest, expected.registration_version,
      expected.expected_supply_base_units, expected.endpoints[1]]
    // The child process replaces only the observation boundary: CLI mapping/printing/exit are real.
    const script = `const assert = require('node:assert/strict');
      require.cache[${JSON.stringify(observer)}] = { exports: { observeCurrency: async input => {
        assert.deepEqual(input, ${JSON.stringify(expected)});
        return {status: ${JSON.stringify(status)}, publication_certified: false,
          asset_identity_verified: false, balance_eligible: false, manifest_transition_allowed: false};
      } } };
      require(${JSON.stringify(cli)}).main(process.argv.slice(1)).catch(() => {process.exitCode = 7});`
    const result = spawnSync(process.execPath, ['-e', script, ...args], {
      encoding: 'utf8', timeout: 5000,
    })
    assert.equal(result.status, status === 'observed' ? 0 : 1)
    assert.equal(result.stderr, '')
    const report = JSON.parse(result.stdout)
    assert.equal(report.status, status)
    for (const flag of ['publication_certified', 'asset_identity_verified', 'balance_eligible',
      'manifest_transition_allowed']) assert.equal(report[flag], false)
  })
}
