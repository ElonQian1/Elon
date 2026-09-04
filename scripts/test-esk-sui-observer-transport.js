const assert = require('node:assert/strict')
const { EventEmitter } = require('node:events')
const transport = require('./esk-sui-publication-observer/transport')
const graphql = require('./esk-sui-publication-observer/graphql')
const { OFFICIAL_TESTNET, ObservationError } = require('./esk-sui-publication-observer/contract')

const EXPECTED_QUERY = `query PublicationObservation($package: SuiAddress!, $digest: String!) {
  chainIdentifier
  transaction(digest: $digest) {
    digest
    effects { status checkpoint { sequenceNumber digest } }
  }
  object(address: $package) {
    address version digest
    asMovePackage { address version }
    previousTransaction { digest }
  }
}`
const expectedInput = { package_id: `0x${'1'.repeat(64)}`, publication_digest: '1'.repeat(32) }
const payload = () => ({ query: 'query SyntheticTransport { chainIdentifier }', variables: {} })
let cases = 0
async function check(name, action) { await action(); cases += 1 }
const code = expected => error => error instanceof ObservationError && error.code === expected

// All traffic is an in-memory event stream; no sockets, DNS or credentials are used.
function wire({ text = '{"data":{"value":"ok"}}', chunks, status = 200, headers, deliver } = {}) {
  const state = { calls: 0, requestDestroyed: 0, responseDestroyed: 0 }
  function request(url, options, callback) {
    state.calls += 1; state.url = url; state.options = options
    const req = state.req = new EventEmitter()
    req.destroy = () => { state.requestDestroyed += 1 }
    req.end = body => {
      state.body = body
      queueMicrotask(() => {
        const res = state.res = new EventEmitter()
        res.destroy = () => { state.responseDestroyed += 1 }
        res.statusCode = status
        res.headers = headers || { 'content-type': 'application/json' }
        callback(res)
        if (deliver) return deliver(req, res)
        for (const chunk of chunks || [Buffer.from(text)]) res.emit('data', chunk)
        res.emit('end')
      })
    }
    return req
  }
  return { state, request }
}

async function main() {
  await check('legacy query bytes exports and request body remain exact', async () => {
    assert.equal(graphql.QUERY, EXPECTED_QUERY)
    assert.deepEqual(Object.keys(graphql).sort(), ['QUERY', 'MAX_BYTES', 'TIMEOUT_MS', 'resolveAddresses', 'publicLookup', 'readObservation'].sort())
    for (const key of ['MAX_BYTES', 'TIMEOUT_MS', 'resolveAddresses', 'publicLookup']) {
      assert.equal(graphql[key], transport[key])
    }
    assert.equal(transport.MAX_BYTES, 128 * 1024)
    assert.equal(transport.TIMEOUT_MS, 12000)
    const stub = wire()
    assert.deepEqual(await graphql.readObservation(OFFICIAL_TESTNET, { ...expectedInput, query: 'ignored' }, stub), { value: 'ok' })
    assert.equal(stub.state.body, JSON.stringify({ query: EXPECTED_QUERY,
      variables: { package: expectedInput.package_id, digest: expectedInput.publication_digest } }))
  })
  await check('invalid endpoint precedes options and input access synchronously', () => {
    let accesses = 0
    const input = { get package_id() { accesses++; throw new Error('input accessed') } }
    const options = { get request() { accesses++; throw new Error('options accessed') } }
    assert.throws(() => graphql.readObservation('http://private.invalid/', input, options), code('INVALID_ENDPOINT'))
    assert.equal(accesses, 0)
  })
  await check('timeout guard precedes payload and network without converting throws to promises', () => {
    for (const timeoutMs of [0, -1, 12001, 1.5, '1', NaN]) {
      let payloadCalls = 0; let networkCalls = 0
      assert.throws(() => transport.readGraphql(OFFICIAL_TESTNET, () => { payloadCalls++; return payload() },
        { timeoutMs, request() { networkCalls++ } }), code('INVALID_INPUT'))
      assert.equal(payloadCalls, 0); assert.equal(networkCalls, 0)
    }
  })
  await check('legacy option getters precede lazy variables and stringify failures stay synchronous', () => {
    const order = []
    const sentinel = new Error('synthetic payload failure')
    const input = { get package_id() { order.push('input'); throw sentinel } }
    const options = {
      get request() { order.push('request'); return () => { throw new Error('must not send') } },
      get timeoutMs() { order.push('timeout'); return 1 },
    }
    assert.throws(() => graphql.readObservation(OFFICIAL_TESTNET, input, options), error => error === sentinel)
    assert.deepEqual(order, ['request', 'timeout', 'input'])
    const circular = {}; circular.self = circular
    assert.throws(() => transport.readGraphql(OFFICIAL_TESTNET, () => circular), TypeError)
    assert.throws(() => graphql.readObservation(OFFICIAL_TESTNET, null), TypeError)
  })
  await check('fixed TLS and headers cannot be overridden and byte length is UTF8', async () => {
    const stub = wire()
    const input = { query: 'query Fixture($x: String!) { value(x: $x) }', variables: { x: '量化' } }
    const url = 'https://reviewed-provider.org:443/graphql'
    await transport.readGraphql(url, () => input, { ...stub, method: 'PUT', headers: { authorization: 'synthetic-forbidden' },
      agent: true, lookup: () => {}, rejectUnauthorized: false, checkServerIdentity: () => {} })
    assert.equal(stub.state.url, url)
    assert.deepEqual(stub.state.options, {
      method: 'POST', agent: false, lookup: transport.publicLookup, autoSelectFamily: false, rejectUnauthorized: true,
      headers: { 'content-type': 'application/json', accept: 'application/json', 'accept-encoding': 'identity',
        'content-length': Buffer.byteLength(stub.state.body) },
    })
    assert.equal(stub.state.body, JSON.stringify(input))
    assert.ok(Buffer.byteLength(stub.state.body) > stub.state.body.length)
  })
  await check('exact maximum split body accepted and one extra byte rejected', async () => {
    const overhead = Buffer.byteLength(JSON.stringify({ data: { padding: '' } }))
    const bytes = Buffer.from(JSON.stringify({ data: { padding: 'x'.repeat(transport.MAX_BYTES - overhead) } }))
    assert.equal(bytes.length, transport.MAX_BYTES)
    const exact = wire({ chunks: [bytes.subarray(0, 7), bytes.subarray(7)],
      headers: { 'content-type': 'application/graphql-response+json; charset=utf-8', 'content-length': String(bytes.length) } })
    const result = await transport.readGraphql(OFFICIAL_TESTNET, payload, exact)
    assert.equal(result.padding.length, transport.MAX_BYTES - overhead)
    const over = wire({ chunks: [bytes, Buffer.from(' ')], headers: { 'content-type': 'application/json', 'content-length': '1' } })
    await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, over), code('RESPONSE_TOO_LARGE'))
    assert.equal(over.state.requestDestroyed, 1); assert.equal(over.state.responseDestroyed, 1)
  })
  await check('multibyte UTF8 split across chunks decodes once after full collection', async () => {
    const bytes = Buffer.from('{"data":"币"}')
    const stub = wire({ chunks: [bytes.subarray(0, 10), bytes.subarray(10, 11), bytes.subarray(11)] })
    assert.equal(await transport.readGraphql(OFFICIAL_TESTNET, payload, stub), '币')
  })
  await check('response-start deadline cleans existing streams and late events cannot settle again', async () => {
    const stub = wire({ deliver: (_req, res) => { res.emit('data', Buffer.from('{"data":')) } })
    await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, { ...stub, timeoutMs: 10 }), code('TIMEOUT'))
    assert.equal(stub.state.requestDestroyed, 1); assert.equal(stub.state.responseDestroyed, 1)
    stub.state.res.emit('data', Buffer.from('true}')); stub.state.res.emit('end')
    stub.state.res.emit('error', new Error('synthetic late response'))
    stub.state.req.emit('error', new Error('synthetic late request'))
    assert.equal(stub.state.requestDestroyed, 1); assert.equal(stub.state.responseDestroyed, 1)
  })
  await check('success ignores later events and its timer cannot destroy the successful streams', async () => {
    const stub = wire()
    await transport.readGraphql(OFFICIAL_TESTNET, payload, { ...stub, timeoutMs: 10 })
    stub.state.res.emit('aborted'); stub.state.req.emit('error', new Error('synthetic late error'))
    await new Promise(resolve => setTimeout(resolve, 20))
    assert.equal(stub.state.requestDestroyed, 0); assert.equal(stub.state.responseDestroyed, 0)
  })
  await check('transport preserves raw data and empty GraphQL errors semantics', async () => {
    for (const [text, expected] of [['{"data":null}', null], ['{}', undefined], ['{"data":false,"errors":[]}', false]]) {
      assert.equal(await transport.readGraphql(OFFICIAL_TESTNET, payload, wire({ text })), expected)
    }
    for (const text of ['[]', 'null', 'false']) {
      await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, wire({ text })), code('INVALID_RESPONSE'))
    }
    for (const errors of [null, {}, 'bad', [{ message: 'synthetic server message' }]]) {
      await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, wire({ text: JSON.stringify({ data: {}, errors }) })), code('GRAPHQL_ERROR'))
    }
  })
  await check('HTTP status and response errors destroy both streams once', async () => {
    for (const candidate of [
      { status: 302, expected: 'HTTP_ERROR' },
      { deliver: (_req, res) => res.emit('error', new Error('synthetic body error')), expected: 'NETWORK_ERROR' },
      { deliver: (req) => req.emit('error', new ObservationError('PRIVATE_ADDRESS')), expected: 'PRIVATE_ADDRESS' },
      { headers: { 'content-type': 'application/json', 'content-encoding': 'gzip' }, expected: 'INVALID_RESPONSE' },
    ]) {
      const stub = wire(candidate)
      await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, stub), code(candidate.expected))
      assert.equal(stub.state.requestDestroyed, 1); assert.equal(stub.state.responseDestroyed, 1)
    }
    await assert.rejects(transport.readGraphql(OFFICIAL_TESTNET, payload, { request() { throw new Error('synthetic request factory failure') } }), code('NETWORK_ERROR'))
  })
  await check('DNS all mode returns exactly one validated pinned IPv6 address', async () => {
    const result = await new Promise((resolve, reject) => transport.publicLookup('public.org', { all: true },
      (error, records) => error ? reject(error) : resolve(records),
      (_host, options, callback) => { assert.deepEqual(options, { all: true }); callback(null,
        [{ address: '2606:4700:4700::1111', family: 6 }, { address: '8.8.8.8', family: 4 }]) }))
    assert.deepEqual(result, [{ address: '2606:4700:4700::1111', family: 6 }])
  })
  await check('DNS empty malformed mixed private and resolver failures remain bounded', async () => {
    for (const records of [[], null, [{ address: '8.8.8.8', family: 4 }, { address: '::1', family: 6 }]]) {
      const error = await new Promise(resolve => transport.publicLookup('public.org', {}, resolve,
        (_host, _options, callback) => callback(null, records)))
      assert.equal(error.code, 'PRIVATE_ADDRESS')
    }
    for (const error of [new Error('synthetic DNS detail'), new ObservationError('TIMEOUT')]) {
      const actual = await new Promise(resolve => transport.publicLookup('public.org', {}, resolve,
        (_host, _options, callback) => callback(error)))
      assert.equal(actual.code, error instanceof ObservationError ? 'TIMEOUT' : 'NETWORK_ERROR')
    }
  })
  await check('DNS single-family success cancels once and ignores callbacks after completion', async () => {
    const callbacks = {}; let canceled = 0; let completions = 0
    const resolver = { resolve4(_host, cb) { callbacks.v4 = cb }, resolve6(_host, cb) { callbacks.v6 = cb }, cancel() { canceled++ } }
    const result = new Promise(resolve => transport.resolveAddresses('public.org', {}, (error, records) => {
      completions++; resolve({ error, records })
    }, resolver, 30))
    callbacks.v6(null, ['2606:4700:4700::1111']); callbacks.v4(new Error('synthetic no A record'))
    assert.deepEqual(await result, { error: null, records: [{ address: '2606:4700:4700::1111', family: 6 }] })
    callbacks.v4(null, ['127.0.0.1']); callbacks.v6(new Error('synthetic late failure'))
    assert.equal(canceled, 1); assert.equal(completions, 1)
  })
  await check('DNS deadline and synchronous resolver failure cancel without raw errors', async () => {
    for (const throwing of [false, true]) {
      let canceled = 0
      const resolver = { resolve4() { if (throwing) throw new Error('synthetic DNS failure') }, resolve6() {}, cancel() { canceled++ } }
      const result = await new Promise(resolve => transport.resolveAddresses('public.org', {}, resolve, resolver, 10))
      assert.equal(result.code, throwing ? 'NETWORK_ERROR' : 'TIMEOUT')
      assert.equal(canceled, 1)
    }
  })
  console.log(`ESK_SUI_SHARED_TRANSPORT_TESTS=${cases}_passed`)
  console.log('ESK_SUI_NETWORK_REQUESTS=stubbed_only')
}

main().catch(error => { console.error(error); process.exitCode = 1 })
