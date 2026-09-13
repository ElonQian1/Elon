'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const api = require('../android/app/src/main/assets/chatgpt_web_history_parent_diagnostic.js');
const ID = '123e4567-e89b-12d3-a456-426614174000';
const USER = '223e4567-e89b-12d3-a456-426614174000';
const SYSTEM = '323e4567-e89b-12d3-a456-426614174000';
const ANSWER = '423e4567-e89b-12d3-a456-426614174000';
function tree() {
  return { conversation_id: ID, current_node: ANSWER, mapping: {
    '': { id: '', parent: null, message: null, children: [SYSTEM] },
    [SYSTEM]: { id: SYSTEM, parent: '', children: [USER], message: { id: SYSTEM,
      author: { role: 'system' }, metadata: { is_visually_hidden_from_conversation: true },
      content: { content_type: 'text', parts: ['not-for-diagnostics'] } } },
    [USER]: { id: USER, parent: SYSTEM, children: [ANSWER], message: { id: USER,
      author: { role: 'user' }, content: { content_type: 'text', parts: ['not-for-diagnostics'] } } },
    [ANSWER]: { id: ANSWER, parent: USER, children: [], message: { id: ANSWER, author: { role: 'assistant' } } }
  } };
}
function host() {
  let payload = tree(), account = 'fixture-account', reads = 0, loads = 0, hook;
  const page = { location: { href: 'https://chatgpt.com/c/' + ID }, document: {},
    __elonChatGptDocumentToken: 'doc_fixture', AbortController, setTimeout, clearTimeout };
  page.__elonChatGptPrivateModelContract = { create: () => ({ withRuntimeIdentity: () => ({ account }) }) };
  page.__elonChatGptPrivateRuntimeBindings = {
    state: () => ({ profile_id: 'web_20260912' }),
    async load(name) {
      loads++;
      if (name === 'shared') return {};
      return { async textHydrateHistory(id, options) {
        reads++;
        assert.equal(id, ID);
        assert.equal(options.forceNetworkFetch, true);
        assert.equal(options.shouldApplyResponse(), false);
        if (hook) await hook(options);
        options.onConversationLoadedFromNetwork(payload);
        assert.equal(options.shouldApplyResponse(), false);
      } };
    }
  };
  return { page, reads: () => reads, loads: () => loads, payload: value => { payload = value; },
    account: value => { account = value; }, hook: value => { hook = value; } };
}
test('describes ancestry without IDs or content and does not change the tree', () => {
  const payload = tree(), before = JSON.stringify(payload), result = api.describe(payload);
  assert.equal(result.code, 'observed');
  assert.deepEqual(result.nodes.map(node => node.role), ['user', 'system', 'none']);
  assert.deepEqual(result.nodes.map(node => node.parent_kind), ['uuid', 'empty_root', 'null']);
  assert.equal(result.nodes[1].hidden, true);
  assert.equal(result.nodes[0].reciprocal, true);
  assert.equal(result.terminal, 'null_parent');
  assert.equal(JSON.stringify(payload), before);
  assert.doesNotMatch(JSON.stringify(result), /not-for-diagnostics|123e4567|223e4567|323e4567|423e4567/);
});
test('reports broken child links, inconsistent IDs, and unknown roles without admitting them', () => {
  const payload = tree();
  payload.mapping[SYSTEM].children = [];
  payload.mapping[SYSTEM].id = 'wrong';
  payload.mapping[SYSTEM].message.author.role = 'untrusted-role';
  const result = api.describe(payload);
  assert.equal(result.nodes[0].reciprocal, false);
  assert.equal(result.nodes[1].node_id_matches, false);
  assert.equal(result.nodes[1].role, 'other');
});
test('missing, cyclic and oversized ancestor chains are bounded', () => {
  const payload = tree();
  payload.mapping[SYSTEM].parent = 'missing';
  assert.equal(api.describe(payload).terminal, 'missing_parent');
  payload.mapping[SYSTEM].parent = USER;
  assert.equal(api.describe(payload).terminal, 'cycle');
  for (let i = 0; i < 20; i++) payload.mapping['root' + i] = { id: 'root' + i, parent: 'root' + (i + 1) };
  payload.mapping[SYSTEM].parent = 'root0';
  assert.equal(api.describe(payload).nodes.length, 16);
  assert.equal(api.describe(payload).terminal, 'limit');
});
test('missing current branch cannot select an unrelated user', () => {
  const payload = tree();
  payload.current_node = 'missing';
  assert.equal(api.describe(payload).code, 'user_missing');
  payload.current_node = ANSWER;
  payload.mapping[ANSWER].parent = ANSWER;
  assert.equal(api.describe(payload).code, 'user_missing');
  assert.equal(api.describe({ messages: [] }).code, 'mapping_missing');
});
test('current conversation read is coalesced and never applies history', async () => {
  const h = host(), a = api.inspect(h.page), b = api.inspect(h.page);
  assert.equal(a, b);
  assert.equal((await a).code, 'observed');
  assert.equal(h.reads(), 1);
  assert.equal((await api.inspect(h.page)).code, 'observed');
  assert.equal(h.reads(), 2);
});
for (const [name, change] of [
  ['navigation', h => { h.page.location.href += '?changed=true'; }],
  ['document', h => { h.page.document = {}; }],
  ['document token', h => { h.page.__elonChatGptDocumentToken = 'doc_changed'; }],
  ['account', h => h.account('changed')]
]) test('discards history after ' + name + ' change', async () => {
  const h = host();
  h.hook(() => change(h));
  assert.equal((await api.inspect(h.page)).code, 'owner_changed');
});
test('requires exact conversation identity and bounded safe errors', async () => {
  const h = host();
  h.payload({ ...tree(), conversation_id: USER });
  assert.equal((await api.inspect(h.page)).code, 'conversation_mismatch');
  h.hook(() => { throw Error('secret-response-body'); });
  const value = await api.inspect(h.page);
  assert.equal(value.code, 'read_failed');
  assert.doesNotMatch(JSON.stringify(value), /secret/);
});
test('timeout aborts the read and late callbacks cannot hydrate or change results', async () => {
  const h = host();
  h.page.setTimeout = callback => setTimeout(callback, 5);
  let options, release;
  h.hook(value => { options = value; return new Promise(resolve => { release = resolve; }); });
  const result = await api.inspect(h.page);
  assert.equal(result.code, 'timeout');
  assert.equal(options.signal.aborted, true);
  assert.equal(options.shouldApplyResponse(), false);
  release();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(result.code, 'timeout');
});
for (const route of ['https://chatgpt.com/', 'https://chatgpt.com/c/' + ID + '?x=1',
  'https://chatgpt.com/c/' + ID + '#x', 'https://example.com/c/' + ID,
  'https://chatgpt.com/g/g-p-example/c/' + ID, 'https://user@chatgpt.com/c/' + ID]) {
  test('does not read an unsupported route: ' + route, async () => {
    const h = host(); h.page.location.href = route;
    assert.equal((await api.inspect(h.page)).code, 'route_unsupported');
    assert.equal(h.loads(), 0); assert.equal(h.reads(), 0);
  });
}
test('requires document, reviewed runtime and identity', async () => {
  const h = host();
  h.page.__elonChatGptDocumentToken = '';
  assert.equal((await api.inspect(h.page)).code, 'document_unavailable');
  h.page.__elonChatGptDocumentToken = 'doc_fixture'; h.account(null);
  assert.equal((await api.inspect(h.page)).code, 'identity_unavailable');
  h.page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: 'unknown' });
  assert.equal((await api.inspect(h.page)).code, 'runtime_unavailable');
  assert.equal(h.reads(), 0);
});
test('MCP handles only the explicit diagnostic mode', async () => {
  const h = host(), receipts = [];
  assert.equal(api.handle(h.page, 'send_prompt', { value: 'history_parent' }, () => {}), false);
  const result = await new Promise(resolve => {
    assert.equal(api.handle(h.page, 'private_protocol_probe', { value: 'history_parent' },
      (...args) => { receipts.push(args); resolve(args); }), true);
  });
  assert.equal(result[1], true); assert.equal(receipts.length, 1);
  assert.equal(JSON.parse(result[2]).code, 'observed');
});
test('existing probe upgrades only its command surface and retains observers', () => {
  let dispatched = 0;
  const source = fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_private_research_probe.js'), 'utf8');
  const oldHandle = () => 'old', window = { __elonChatGptPrivateResearchProbe: { version: 25, handle: oldHandle,
    observer: 'preserved' }, __elonChatGptHistoryParentDiagnostic: { handle: () => { dispatched++; return true; } } };
  vm.runInNewContext(source, { window, location: { origin: 'https://chatgpt.com' } });
  assert.equal(window.__elonChatGptPrivateResearchProbe.version, 26);
  assert.equal(window.__elonChatGptPrivateResearchProbe.observer, 'preserved');
  window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'history_parent' }, () => {});
  assert.equal(dispatched, 1);
});
