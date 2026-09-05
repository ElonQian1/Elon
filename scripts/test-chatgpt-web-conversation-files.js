'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.join(__dirname, '..');
const assets = path.join(root, 'android/app/src/main/assets');
const projection = require(path.join(assets, 'chatgpt_web_private_history_projection.js')).create({});
const fixture = JSON.parse(fs.readFileSync(path.join(root,
  'android/app/src/test/resources/webchat/private-conversation-files-contract.json'), 'utf8'));
const clone = (value) => JSON.parse(JSON.stringify(value));
const fileMessage = (id) => ({ id, author: { role: 'user' },
  content: { parts: ['Synthetic message'] }, metadata: { attachments: [{ name: id + '.txt' }] } });
let cases = 0;
async function test(name, run) { await run(); cases++; console.log('PASS ' + name); }

function runtime(fetchImpl, ready = true) {
  const timers = new Set();
  let now = Date.now();
  class Clock extends Date { static now() { return now; } }
  const context = { URL, AbortController, Date: Clock, Map, Set, Promise, console };
  const window = {
    __elonChatGptPrivateConversationPrefetchEnabled: true,
    __elonChatGptPrivateResearchEnabled: false,
    __elonChatGptPrivateAuthContext: {
      canAcquire: () => ready,
      state: () => ({ ready, lastOutcome: 'session_ready', lastSuccessAt: now, lastLatencyMs: 100 }),
      copyRequestHeaders: () => ready ? { Authorization: 'synthetic-fixture' } : null,
      acquireRequestHeaders: async () => ({ Authorization: 'synthetic-fixture' }),
      subscribe: () => () => {}, invalidate: () => {},
    },
    fetch: fetchImpl,
    setTimeout: (fn, ms) => { const id = setTimeout(fn, ms); timers.add(id); return id; },
    clearTimeout: (id) => { clearTimeout(id); timers.delete(id); },
    location: { origin: 'https://chatgpt.com', pathname: '/c/current', href: 'https://chatgpt.com/c/current' },
  };
  Object.assign(context, { window, location: window.location });
  for (const name of ['chatgpt_web_private_transport_policy.js', 'chatgpt_web_private_history_projection.js',
    'chatgpt_web_private_transport.js', 'chatgpt_web_adapter_conversation_directory_requests.js']) {
    vm.runInNewContext(fs.readFileSync(path.join(assets, name), 'utf8'), context, { filename: name });
  }
  return { window, transport: window.__elonChatGptPrivateTransport, timers, advance: ms => { now += ms; } };
}
const response = (payload) => ({ ok: true, status: 200, json: async () => payload });
const plain = (value) => JSON.parse(JSON.stringify(value));

(async () => {
  await test('shared Android fixture contains descriptors, never credentials or file handles', () => {
    assert.deepEqual(projection.files(fixture.input), { files: fixture.event.files, truncated: false });
    assert.doesNotMatch(JSON.stringify(projection.files(fixture.input)), /secret|download_url|asset_pointer/);
  });
  await test('empty recognized history is distinct from unknown or broken history', () => {
    for (const value of [{ mapping: {} }, { messages: [] }, { data: { items: [] } }]) {
      assert.deepEqual(projection.files(value), { files: [], truncated: false });
    }
    for (const value of [{}, { messages: [null] }, { error: 'not_ready' }, { mapping: { a: { parent: 'missing' } } },
      { current_node: 'a', mapping: { a: { parent: 'a' } } }]) assert.equal(projection.files(value), null);
    assert.deepEqual(projection.files({ messages: true, items: [] }), { files: [], truncated: false });
  });
  await test('only selected branch attachments are indexed', () => {
    const value = clone(fixture.input);
    value.mapping.alternative = { parent: 'question', message: fileMessage('unselected') };
    assert.deepEqual(projection.files(value).files, fixture.event.files);
    delete value.current_node;
    assert.equal(projection.files(value), null);
  });
  await test('directory reaches attachments older than the native 80-message window', () => {
    const messages = Array.from({ length: 90 }, (_, i) => ({ id: 'm-' + i, role: 'user', content: 'text' }));
    messages[0] = fileMessage('old-file');
    assert.equal(projection.project({ messages }).length, 80);
    assert.equal(projection.files({ messages }).files[0].name, 'old-file.txt');
  });
  await test('hidden and internal assistant records cannot leak into the directory', () => {
    const hidden = fileMessage('hidden'); hidden.metadata.is_visually_hidden_from_conversation = true;
    const analysis = fileMessage('analysis'); analysis.author.role = 'assistant'; analysis.channel = 'analysis';
    const tool = fileMessage('tool'); tool.author.role = 'assistant'; tool.recipient = 'python';
    assert.deepEqual(projection.files({ messages: [hidden, analysis, tool] }).files, []);
  });
  await test('mixed image/file entries are not limited by message display parts', () => {
    const message = fileMessage('mixed');
    message.content.parts = Array.from({ length: 15 }, () => ({ content_type: 'image_asset_pointer' }));
    message.metadata.attachments = Array.from({ length: 15 }, (_, i) => ({ name: 'f-' + i }));
    const value = projection.files({ messages: [message] });
    assert.equal(value.files.length, 30);
    assert.equal(value.truncated, false);
  });
  await test('row and traversal limits are visible as partial, not complete empty lists', () => {
    const value = projection.files({ messages: Array.from({ length: 101 }, (_, i) => fileMessage('f-' + i)) });
    assert.equal(value.files.length, 100); assert.equal(value.truncated, true);
    const large = Array.from({ length: 4097 }, (_, i) => ({ id: 'm-' + i, role: 'user', content: '' }));
    assert.equal(projection.files({ messages: large }).truncated, true);
    const message = fileMessage('many');
    message.metadata.attachments = Array.from({ length: 21 }, () => ({ name: 'test.txt' }));
    assert.equal(projection.files({ messages: [message] }).truncated, true);
  });
  await test('private file read is GET-only and cannot change the current conversation or composer', async () => {
    const requests = []; const events = []; const receipts = [];
    const { window, transport, timers } = runtime(async (url, options) => {
      requests.push({ url, options }); return response(fixture.input);
    });
    await transport.listConversationFiles('/c/fixture', 'mcp_f1', e => events.push(plain(e)),
      (...args) => receipts.push(args));
    assert.deepEqual(events, [fixture.event]);
    assert.deepEqual(receipts, [['list_conversation_files', true, 'private_files_ready']]);
    assert.equal(requests.length, 1); assert.equal(requests[0].options.method, 'GET');
    assert.equal(requests[0].url, '/backend-api/conversations/fixture');
    assert.equal(window.location.pathname, '/c/current'); assert.equal(timers.size, 0);
  });
  await test('concurrent history prefetch and directory share one HTTP request', async () => {
    let resolve; let count = 0; const events = [];
    const pending = new Promise(r => { resolve = r; });
    const { transport } = runtime(async () => { count++; return pending; });
    assert.equal(transport.prefetchConversation('/c/fixture', e => events.push(e), () => {}), true);
    const first = transport.listConversationFiles('/c/fixture', 'mcp_a', e => events.push(e), () => {});
    const second = transport.listConversationFiles('/g/g-p-demo/c/fixture', 'mcp_b', e => events.push(e), () => {});
    resolve(response(fixture.input));
    await Promise.all([first, second]);
    assert.equal(count, 1);
    assert.equal(events.filter(e => e.type === 'conversation_files_snapshot').length, 2);
    assert.equal(events.filter(e => e.type === 'message_snapshot').length, 1);
  });
  await test('missing identity and invalid requests do not navigate or fetch', async () => {
    const { transport } = runtime(() => assert.fail('must not fetch'), false);
    const receipts = []; const emit = () => assert.fail('must not publish');
    await transport.listConversationFiles('/c/test', 'mcp_x', emit, (...args) => receipts.push(args));
    await transport.listConversationFiles('https://example.com/c/test', 'mcp_x', emit, (...args) => receipts.push(args));
    await transport.listConversationFiles('/c/test', 'bad-request', emit, (...args) => receipts.push(args));
    assert.deepEqual(receipts.map(r => r[2]), ['files_not_ready', 'invalid_file_request', 'invalid_file_request']);
  });
  await test('failed or unknown response does not publish an empty success', async () => {
    for (const payload of [null, { error: 'unknown_shape' }]) {
      const { transport, timers } = runtime(async () => response(payload)); const receipts = [];
      await transport.listConversationFiles('/c/test', 'mcp_x', () => assert.fail('no empty success'),
        (...args) => receipts.push(args));
      assert.deepEqual(receipts, [['list_conversation_files', false, 'files_read_failed']]);
      assert.equal(timers.size, 0);
    }
  });
  await test('a rejected read releases its single-flight slot for an explicit retry', async () => {
    let count = 0; const receipts = []; const events = [];
    const { transport, advance } = runtime(async () => {
      if (++count === 1) throw new Error('network');
      return response(fixture.input);
    });
    const read = () => transport.listConversationFiles('/c/test', 'mcp_x', e => events.push(e), (...a) => receipts.push(a));
    await read(); advance(10_001); await read();
    assert.equal(count, 2); assert.equal(events.length, 1);
    assert.deepEqual(receipts.map(r => r[1]), [false, true]);
  });
  await test('domain dispatcher reuses file transport and preserves cancel/probe behavior', () => {
    const { window } = runtime(() => assert.fail('no fetch')); const calls = []; const receipts = [];
    const directory = window.__elonChatGptConversationDirectoryRequests.create({
      conversationAdapter: { cancelDirectoryWork: () => calls.push('cancel') },
      privateTransport: { listConversationFiles: (...args) => calls.push(args.slice(0, 2)) },
      emitEvent: () => {}, optional: (fallback, run) => run(),
    });
    const reply = (...args) => receipts.push(args);
    assert.equal(directory.handleCommand({ action: 'list_conversation_files', value: '/c/test', requestId: 'mcp_x' }, reply), true);
    assert.equal(directory.handleCommand({ action: 'cancel_conversation_directory' }, reply), true);
    assert.equal(directory.handleCommand({ action: 'probe_conversation_project' }, reply), true);
    assert.equal(directory.handleCommand({ action: 'send_prompt' }, reply), false);
    assert.deepEqual(calls, [['/c/test', 'mcp_x'], 'cancel']);
    assert.equal(receipts[1][2], 'membership_probe_unavailable');
  });
  await test('directory module upgrades an already-injected older factory', () => {
    const window = { __elonChatGptConversationDirectoryRequests: { create: () => ({}) } };
    vm.runInNewContext(fs.readFileSync(path.join(assets,
      'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'), { window });
    assert.equal(window.__elonChatGptConversationDirectoryRequests.version, 2);
    assert.equal(typeof window.__elonChatGptConversationDirectoryRequests.create({}).handleCommand, 'function');
  });
  console.log('PASS conversation files: ' + cases + ' cases');
})().catch(error => { console.error(error); process.exitCode = 1; });
