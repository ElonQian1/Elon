const assert = require('node:assert/strict')
const { test } = require('node:test')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')

const cache = new Map()
function load(name) {
  const filename = path.join(__dirname, `${name}.ts`)
  if (cache.has(filename)) return cache.get(filename)
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = module.paths
  compiled.require = (id) => id.startsWith('./') ? load(id.slice(2)) : require(id)
  compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS }, fileName: filename,
  }).outputText, filename)
  cache.set(filename, compiled.exports)
  return compiled.exports
}
const { parseResearchResult, parseResearchAction, createResearchEpoch, researchErrorMessage } = load('browserResearchModel')
const { createResearchExecutor } = load('browserResearchExecutor')
const { nativeResearchErrorCode, receiptErrorCode, RESEARCH_FAILURE_CODES } = load('browserResearchErrors')
const schema = 'yilong.browser-research.result.v1'
const result = { schema, kind: 'sites', items: [], total: 0, offset: 0, next_offset: null }
const action = (id = 'research_1') => ({ action_id: id, project_key: 'a'.repeat(64), command: { kind: 'sites' }, requested_at_ms: 100, expires_at_ms: 1000, status: 'queued' })
const resource = { id: 'resource_1', url: 'https://example.org/app.js', resource_type: 'Script', mime: 'text/javascript', size_bytes: 11, sha256: 'b'.repeat(64), generation: 2, truncated: false, redacted: true }

test('version, command kind, item ID, and page movement are validated before display', () => {
  assert.equal(parseResearchResult(result, { kind: 'sites' }).total, 0)
  assert.throws(() => parseResearchResult({ ...result, schema: 'future' }, { kind: 'sites' }))
  assert.throws(() => parseResearchResult(result, { kind: 'sessions' }))
  assert.throws(() => parseResearchResult({ ...result, next_offset: 0 }, { kind: 'sites' }))
  const read = { schema, kind: 'read_resource', item: resource, content: 'const x = 1', offset: 0, next_offset: null, complete: true }
  assert.equal(parseResearchResult(read, { kind: 'read_resource', resource_id: resource.id }).content, 'const x = 1')
  assert.throws(() => parseResearchResult(read, { kind: 'read_resource', resource_id: 'another_resource' }))
})

test('oversized UTF-8 result and unsafe numeric metadata fail closed', () => {
  assert.throws(() => parseResearchResult({ ...result, extra: '中'.repeat(23000) }, { kind: 'sites' }))
  assert.throws(() => parseResearchResult({ ...result, total: Number.MAX_SAFE_INTEGER + 1 }, { kind: 'sites' }))
})

test('search partial coverage remains visible and must be boolean', () => {
  const search = { ...result, kind: 'search', partial: true, items: [{ resource_id: resource.id, url: resource.url, offset: 2, excerpt: 'code candidate' }], total: 1 }
  assert.equal(parseResearchResult(search, { kind: 'search' }).partial, true)
  assert.throws(() => parseResearchResult({ ...search, partial: 'yes' }, { kind: 'search' }))
})

test('register_site consumes the actual native list response', () => {
  const site = { schema: 'yilong.browser-research.site.v1', id: 'fixture', name: '合成研究站点', entry_url: 'https://example.org', navigation_origins: ['https://example.org'], resource_origins: [], api_origins: [], identity_origins: [] }
  const registered = { ...result, kind: 'register_site', items: [site], total: 1 }
  assert.equal(parseResearchResult(registered, { kind: 'register_site', manifest: site }).items[0].id, 'fixture')
  assert.throws(() => parseResearchResult({ schema, kind: 'register_site', site }, { kind: 'register_site' }))
})

test('a session cannot be shown for another selected session or with trading enabled', () => {
  const session = { id: 'session_1', site_id: 'example', active: false, generation: 1, expires_at_ms: 500, resource_count: 0, request_count: 0, phase: 'paused', gaps: [], trading_enabled: false }
  assert.throws(() => parseResearchResult({ schema, kind: 'status', session }, { kind: 'status', session_id: 'session_2' }))
  assert.throws(() => parseResearchResult({ schema, kind: 'status', session: { ...session, trading_enabled: true } }, { kind: 'status', session_id: 'session_1' }))
})

test('queue commands reject injected owner, path, and arbitrary execution fields', () => {
  assert.equal(parseResearchAction(action()).command.kind, 'sites')
  for (const extra of [{ ownerKey: 'other' }, { project_root: 'elsewhere' }, { script: 'trade()' }]) {
    assert.throws(() => parseResearchAction({ ...action(), command: { kind: 'sites', ...extra } }))
  }
})

test('invalidating a view discards prior and late results', () => {
  const epoch = createResearchEpoch()
  const first = epoch.next()
  assert.equal(epoch.current(first), true)
  const second = epoch.next()
  assert.equal(epoch.current(first), false)
  assert.equal(epoch.current(second), true)
  epoch.next()
  assert.equal(epoch.current(second), false)
})

function harness(overrides = {}) {
  const calls = { invoked: [], receipts: [], claimed: [] }
  const deps = {
    pending: async () => [action()],
    claim: async (id) => { calls.claimed.push(id); return { action: { ...action(id), status: 'executing' }, claim_token: 'claim_1' } },
    receipt: async (id, value) => { calls.receipts.push({ id, value }) },
    invoke: async (...args) => { calls.invoked.push(args); return result },
    owner: () => 'native_stable_owner', now: () => 200, ...overrides,
  }
  return { ...calls, executor: createResearchExecutor(deps), deps }
}

test('native execution uses successful claim payload exactly once', async () => {
  const h = harness({ claim: async () => ({ action: { ...action(), project_key: 'c'.repeat(64), command: { kind: 'sites' } }, claim_token: 'claim_2' }) })
  await h.executor.poll(); await h.executor.poll()
  assert.equal(h.invoked.length, 1)
  assert.equal(h.invoked[0][0], 'c'.repeat(64))
  assert.equal(h.receipts[0].value.claim_token, 'claim_2')
  assert.equal(h.receipts[0].value.status, 'succeeded')
})

test('claim conflict, absent owner, and expired action never invoke host', async () => {
  for (const overrides of [{ claim: async () => { throw new Error('conflict') } }, { owner: () => '' }, { now: () => 1001 }]) {
    const h = harness(overrides); await h.executor.poll(); assert.equal(h.invoked.length, 0)
  }
})

test('overlapping polls are coalesced until native completion', async () => {
  let release
  const h = harness({ invoke: () => new Promise((resolve) => { release = resolve }) })
  const first = h.executor.poll()
  await new Promise((resolve) => setImmediate(resolve))
  await h.executor.poll()
  assert.equal(h.claimed.length, 1)
  release(result); await first
})

test('receipt transport retries receipt without replaying native command', async () => {
  let attempts = 0
  const h = harness({ receipt: async () => { if (++attempts === 1) throw new Error('offline') } })
  await h.executor.poll(); await h.executor.poll()
  assert.equal(attempts, 2)
  assert.equal(h.invoked.length, 1)
})

test('owner switch or logout permanently drops an undelivered private receipt', async () => {
  let owner = 'first_owner'
  const attempts = []
  const h = harness({
    owner: () => owner,
    receipt: async (_id, value) => {
      attempts.push(JSON.parse(JSON.stringify(value)))
      if (attempts.length <= 2) throw new Error('transport offline')
    },
  })
  await h.executor.poll()
  assert.equal(attempts[0].status, 'succeeded')
  owner = ''
  await h.executor.poll()
  assert.deepEqual(attempts[1], { claim_token: 'claim_1', status: 'host_unavailable', error_code: 'host_unavailable' })
  owner = 'first_owner'
  await h.executor.poll()
  assert.equal(attempts[2].status, 'host_unavailable', 'logging back in must not resurrect discarded content')
  assert.equal('result' in attempts[2], false)
  assert.equal(h.invoked.length, 1, 'delivery retries must never rerun the native command')
})

test('stable native failures remain diagnosable through receipts without raw messages', async () => {
  for (const [native, expected] of [
    ['browser_research_host_dispatch_failed', 'host_unavailable'],
    ['research_host_unavailable', 'host_unavailable'],
    ['session_scope_mismatch', 'invalid_scope'],
    ['site_scope_changed', 'invalid_scope'],
    ['research_session_expired', 'session_expired'],
    ['invalid_content_offset', 'invalid_command'],
  ]) {
    const h = harness({ invoke: async () => { throw native } })
    await h.executor.poll()
    assert.equal(h.receipts[0].value.error_code, expected)
    assert.equal('result' in h.receipts[0].value, false)
  }
  for (const raw of ['private_request_body', 'site_scope_changed: private_request_body', ' session_scope_mismatch ', 'constructor', { message: 'session_scope_mismatch' }]) {
    assert.equal(nativeResearchErrorCode(raw), 'operation_failed')
  }
  assert.equal(nativeResearchErrorCode(new Error('research_session_expired')), 'session_expired')
  assert.equal(receiptErrorCode('session_scope_mismatch'), 'operation_failed', 'receipt codes are stricter than native aliases')
})

test('every public frontend failure code is accepted by the node receipt contract', () => {
  const contract = fs.readFileSync(path.resolve(__dirname, '../../../../server/src/node_agent_browser_research_contract.rs'), 'utf8')
  const whitelist = contract.slice(contract.indexOf('pub(crate) fn valid_error'))
  for (const code of RESEARCH_FAILURE_CODES) assert.ok(whitelist.includes(`"${code}"`), `node contract missing ${code}`)
})

test('expired receipt cannot block new research actions forever', async () => {
  let now = 200
  let pending = action()
  const h = harness({ now: () => now, pending: async () => [pending], receipt: async () => { throw new Error('offline') } })
  await h.executor.poll()
  now = 1100; pending = { ...action('research_2'), expires_at_ms: 2000 }
  h.deps.claim = async (id) => ({ action: pending, claim_token: id })
  await h.executor.poll()
  assert.equal(h.invoked.length, 2)
})

test('owner changes discard private result and exceptions never expose raw errors', async () => {
  let owner = 'first_owner'
  const h = harness({ owner: () => owner, invoke: async () => { owner = 'second_owner'; return result } })
  await h.executor.poll()
  assert.deepEqual(h.receipts[0].value, { claim_token: 'claim_1', status: 'host_unavailable', error_code: 'host_unavailable' })
  const failed = harness({ invoke: async () => { throw new Error('private_request_body') } })
  await failed.executor.poll()
  assert.equal(JSON.stringify(failed.receipts).includes('private_request_body'), false)
  assert.equal(researchErrorMessage(new Error('private_request_body')).includes('private_request_body'), false)
})

test('malformed native result is a failed receipt, never succeeded', async () => {
  const h = harness({ invoke: async () => ({ schema, kind: 'sites', items: 'wrong' }) })
  await h.executor.poll()
  assert.equal(h.receipts[0].value.status, 'failed')
  assert.equal('result' in h.receipts[0].value, false)
})
