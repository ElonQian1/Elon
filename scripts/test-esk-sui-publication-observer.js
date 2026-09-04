const assert = require('node:assert/strict')
const { EventEmitter } = require('node:events')
const { spawnSync } = require('node:child_process')
const path = require('node:path')
const { OFFICIAL_TESTNET, digest32, publicAddress, validateInput } =
  require('./esk-sui-publication-observer/contract')
const { observePublication } = require('./esk-sui-publication-observer/observe')
const { QUERY, MAX_BYTES, readObservation, publicLookup, resolveAddresses } = require('./esk-sui-publication-observer/graphql')

// Synthetic fixture only. No network, wallets or real ESK deployment parameters.
const DIGEST = '11111111111111111111111111111111'
const OTHER_DIGEST = '11111111111111111111111111111112'
const PACKAGE = `0x${'1'.repeat(64)}`
const SECONDARY = 'https://reviewed-provider.org/graphql'
const input = () => ({ network: 'testnet', chain_identifier: DIGEST, package_id: PACKAGE,
  publication_digest: DIGEST, endpoints: [OFFICIAL_TESTNET, SECONDARY] })
const data = () => ({ chainIdentifier: DIGEST,
  transaction: { digest: DIGEST, effects: { status: 'SUCCESS',
    checkpoint: { sequenceNumber: 123, digest: DIGEST } } },
  object: { address: PACKAGE, version: 1, digest: DIGEST,
    asMovePackage: { address: PACKAGE, version: 1 }, previousTransaction: { digest: DIGEST } },
})
let cases = 0
async function check(name, fn) {
  await fn()
  cases += 1
}
function boundaries(report) {
  for (const field of ['publication_certified', 'asset_identity_verified', 'balance_eligible',
    'manifest_transition_allowed']) assert.equal(report[field], false)
}
async function rejectedInput(change) {
  const candidate = input()
  change(candidate)
  let calls = 0
  const result = await observePublication(candidate, { read: async () => { calls++; return data() } })
  assert.equal(result.status, 'unverified')
  assert.equal(result.expected, null)
  assert.equal(calls, 0)
  boundaries(result)
  assert.ok(!JSON.stringify(result).includes('secret'))
}
function stubRequest({ status = 200, text, headers, hang = false, error = false, aborted = false } = {}) {
  return (url, options, onResponse) => {
    assert.equal(options.method, 'POST')
    assert.equal(options.lookup, publicLookup)
    assert.equal(options.agent, false)
    assert.equal(options.autoSelectFamily, false)
    assert.equal(options.rejectUnauthorized, true)
    const req = new EventEmitter()
    req.destroy = () => {}
    req.end = body => {
      const sent = JSON.parse(body)
      assert.equal(sent.query, QUERY)
      assert.deepEqual(Object.keys(sent.variables).sort(), ['digest', 'package'])
      queueMicrotask(() => {
        if (error) return req.emit('error', new Error('secret remote message'))
        if (hang) return
        const res = new EventEmitter()
        res.destroy = () => {}
        res.statusCode = status
        res.headers = headers || { 'content-type': 'application/json' }
        onResponse(res)
        if (aborted) return res.emit('aborted')
        res.emit('data', Buffer.from(text ?? JSON.stringify({ data: data() })))
        res.emit('end')
      })
    }
    return req
  }
}
async function main() {
  await check('success: exact two-source consensus without certification', async () => {
    const result = await observePublication(input(), { read: async () => data() })
    assert.equal(result.status, 'observed')
    assert.equal(result.sources.length, 2)
    assert.equal(result.evidence.checkpoint_sequence, '123')
    assert.equal(result.evidence.package_version, '1')
    assert.equal(result.error_code, null)
    assert.ok(!JSON.stringify(result).includes(SECONDARY))
    assert.equal(result.sources[1].endpoint_sha256.length, 64)
    boundaries(result)
  })
  for (const [name, change] of [
    ['wrong chain', d => { d.chainIdentifier = OTHER_DIGEST }],
    ['missing object', d => { d.object = null }],
    ['not a package', d => { d.object.asMovePackage = null }],
    ['different package', d => { d.object.address = '0x2' }],
    ['wrong move package', d => { d.object.asMovePackage.address = '0x2' }],
    ['different version', d => { d.object.asMovePackage.version = 2 }],
    ['unsafe version', d => { d.object.version = d.object.asMovePackage.version = 1e20 }],
    ['invalid object digest', d => { d.object.digest = '123' }],
    ['wrong creation transaction', d => { d.object.previousTransaction.digest = OTHER_DIGEST }],
    ['wrong transaction', d => { d.transaction.digest = OTHER_DIGEST }],
    ['missing transaction', d => { d.transaction = null }],
    ['failed transaction', d => { d.transaction.effects.status = 'FAILURE' }],
    ['missing effects', d => { d.transaction.effects = null }],
    ['missing checkpoint', d => { d.transaction.effects.checkpoint = null }],
    ['unsafe checkpoint', d => { d.transaction.effects.checkpoint.sequenceNumber = 1e20 }],
    ['negative checkpoint', d => { d.transaction.effects.checkpoint.sequenceNumber = -1 }],
    ['decimal checkpoint', d => { d.transaction.effects.checkpoint.sequenceNumber = 1.5 }],
    ['string checkpoint', d => { d.transaction.effects.checkpoint.sequenceNumber = '123' }],
    ['invalid checkpoint digest', d => { d.transaction.effects.checkpoint.digest = '123' }],
    ['checkpoint disagreement', d => { d.transaction.effects.checkpoint.sequenceNumber = 124 }],
    ['checkpoint hash disagreement', d => { d.transaction.effects.checkpoint.digest = OTHER_DIGEST }],
    ['package hash disagreement', d => { d.object.digest = OTHER_DIGEST }],
  ]) await check(name, async () => {
    const result = await observePublication(input(), { read: async url => {
      const value = data()
      if (url === SECONDARY) change(value)
      return value
    } })
    assert.equal(result.status, 'unverified', name)
    assert.equal(result.evidence, null)
    assert.ok(result.error_code)
    if (name.endsWith('disagreement')) assert.equal(result.error_code, 'SOURCE_DISAGREEMENT')
    boundaries(result)
  })
  for (const [name, change] of [
    ['no mainnet', i => { i.network = 'mainnet' }],
    ['no legacy chain id', i => { i.chain_identifier = '4c78adac' }],
    ['invalid digest', i => { i.publication_digest = 'secret' }],
    ['no zero object', i => { i.package_id = '0x0' }],
    ['no extra credentials', i => { i.api_key = 'secret' }],
    ['no input query', i => { i.query = 'mutation secret' }],
    ['no single endpoint', i => { i.endpoints.pop() }],
    ['no same endpoint', i => { i.endpoints[1] = OFFICIAL_TESTNET }],
    ['no same host path alias', i => { i.endpoints[1] = OFFICIAL_TESTNET + '/alias' }],
    ['official primary required', i => { i.endpoints.reverse() }],
    ...['http://host.org/graphql', 'https://secret@host.org/graphql',
      'https://host.org/graphql?key=secret', 'https://host.org/graphql#secret',
      'https://host.org/graphql/secret',
      'https://127.0.0.1/graphql', 'https://[::1]/graphql', 'https://localhost/graphql',
      'https://host.local/graphql', 'https://host.org:8080/graphql',
      'https://host.org/%73ecret', 'https://host.org/with space'].map(url =>
      ['reject endpoint', i => { i.endpoints[1] = url }]),
  ]) await check(name, () => rejectedInput(change))
  await check('32 byte Base58 exactness', () => {
    assert.ok(digest32(DIGEST) && digest32(OTHER_DIGEST))
    assert.ok(!digest32('1'.repeat(31)) && !digest32('1'.repeat(33)))
    assert.ok(!digest32('z'.repeat(44)) && !digest32('0'.repeat(32)))
    assert.equal(validateInput({ ...input(), package_id: '0xa' }).package_id, `0x${'0'.repeat(63)}a`)
  })
  await check('public IP bounds', () => {
    for (const address of ['127.0.0.1', '10.0.0.1', '192.168.1.1', '172.31.1.1',
      '169.254.169.254', '100.64.0.1', '198.18.0.1', '192.0.2.1', '203.0.113.1',
      '224.0.0.1', '::1', 'fc00::1', 'fe80::1', '::ffff:127.0.0.1', '2001:db8::1',
      '2002::1', '2001:20::1', '3fff::1', '3fff:fff::1']) {
      assert.equal(publicAddress(address), false, address)
    }
    assert.ok(publicAddress('8.8.8.8') && publicAddress('2606:4700:4700::1111'))
  })
  await check('DNS validated address pin and mixed private rejection', async () => {
    const lookup = records => new Promise(resolve => publicLookup('host.org', {},
      (error, address, family) => resolve({ error, address, family }),
      (_host, _options, cb) => cb(null, records)))
    assert.deepEqual(await lookup([{ address: '8.8.8.8', family: 4 }]),
      { error: null, address: '8.8.8.8', family: 4 })
    assert.equal((await lookup([{ address: '8.8.8.8', family: 4 },
      { address: '10.0.0.1', family: 4 }])).error.code, 'PRIVATE_ADDRESS')
  })
  await check('transport success and bounded constant query', async () => {
    assert.deepEqual(await readObservation(OFFICIAL_TESTNET, input(), { request: stubRequest() }), data())
    assert.ok(!/mutation|executeTransaction|simulateTransaction/.test(QUERY))
  })
  await check('cancellable DNS deadline', async () => {
    let cancelled = false
    const resolver = { resolve4() {}, resolve6() {}, cancel() { cancelled = true } }
    const error = await new Promise(resolve => resolveAddresses('host.org', {}, resolve, resolver, 10))
    assert.equal(error.code, 'TIMEOUT')
    assert.equal(cancelled, true)
  })
  await check('DNS records and successful cleanup', async () => {
    let cancelled = false
    const resolver = {
      resolve4(_host, cb) { queueMicrotask(() => cb(null, ['8.8.8.8'])) },
      resolve6(_host, cb) { queueMicrotask(() => cb(new Error('ENODATA'))) },
      cancel() { cancelled = true },
    }
    const records = await new Promise((resolve, reject) => resolveAddresses('host.org', {},
      (error, values) => error ? reject(error) : resolve(values), resolver))
    assert.deepEqual(records, [{ address: '8.8.8.8', family: 4 }])
    assert.equal(cancelled, true)
  })
  for (const [name, options, code] of [
    ['network failure', { error: true }, 'NETWORK_ERROR'],
    ['deadline', { hang: true }, 'TIMEOUT'],
    ['partial body', { aborted: true }, 'NETWORK_ERROR'],
    ['HTTP redirect forbidden', { status: 302 }, 'HTTP_ERROR'],
    ['HTTP error', { status: 500 }, 'HTTP_ERROR'],
    ['bad JSON', { text: 'secret' }, 'INVALID_RESPONSE'],
    ['invalid UTF8', { text: Buffer.from([0xc0, 0xaf]) }, 'INVALID_RESPONSE'],
    ['HTML body', { headers: { 'content-type': 'text/html' } }, 'INVALID_RESPONSE'],
    ['compressed body', { headers: { 'content-type': 'application/json',
      'content-encoding': 'gzip' } }, 'INVALID_RESPONSE'],
    ['chunk body limit', { text: 'x'.repeat(MAX_BYTES + 1) }, 'RESPONSE_TOO_LARGE'],
    ['header body limit', { headers: { 'content-type': 'application/json',
      'content-length': MAX_BYTES + 1 } }, 'RESPONSE_TOO_LARGE'],
    ['GraphQL partial errors', { text: JSON.stringify({ data: data(), errors: [{ message: 'secret' }] }) }, 'GRAPHQL_ERROR'],
    ['GraphQL malformed errors', { text: JSON.stringify({ data: data(), errors: 'secret' }) }, 'GRAPHQL_ERROR'],
  ]) await check(name, async () => {
    const result = await observePublication(input(), { read: (url, expected) =>
      readObservation(url, expected, { request: stubRequest(options), timeoutMs: 10 }) })
    assert.equal(result.status, 'unverified', name)
    assert.equal(result.error_code, code, name)
    assert.ok(!JSON.stringify(result).includes('secret'))
    boundaries(result)
  })
  await check('CLI usage succeeds, malformed call fails without echo', () => {
    const cli = path.join(__dirname, 'observe-esk-sui-publication.js')
    const help = spawnSync(process.execPath, [cli, '--help'], { encoding: 'utf8' })
    assert.equal(help.status, 0)
    const invalid = spawnSync(process.execPath, [cli, 'secret'], { encoding: 'utf8' })
    assert.equal(invalid.status, 1)
    assert.equal(JSON.parse(invalid.stdout).status, 'unverified')
    assert.ok(!invalid.stdout.includes('secret') && !invalid.stderr.includes('secret'))
  })
  console.log(`ESK_SUI_OBSERVER_TESTS=${cases}_passed`)
  console.log('ESK_SUI_NETWORK_WRITES=none')
  console.log('ESK_SUI_PUBLICATION_CERTIFICATION=not_performed')
}

main().catch(error => { console.error(error); process.exitCode = 1 })
