'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fixture = require('./fixtures/chatgpt-attachment-composer');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_attachment');
const HANDLE = 'library_' + 'a'.repeat(32);

function setup(remote = true) {
  const f = fixture(), tasks = new Map(), responses = [], changed = [];
  let now = 0, sequence = 0, scopeReady, materialized, writes = 0;
  Object.assign(f.fiber.memoizedProps, { conversation: {}, onCreateNewCompletion() {},
    currentModelId: 'synthetic-model', entrySurface: 'chat_composer', isLibraryEnabled: true });
  Object.assign(f.root, { File, performance: { now: () => now },
    setTimeout: (fn, delay) => { const id = ++sequence; tasks.set(id, { fn, at: now + delay }); return id; },
    clearTimeout: id => tasks.delete(id) });
  const source = { kind: 'file', id: 'libfile_fixture', file_id: 'file-fixture',
    name: 'fixture.txt', mime_type: 'text/plain', file_size_bytes: 45 };
  const cid = '00000000-0000-4000-8000-000000000001';
  f.root.location.href += 'c/' + cid;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = () => new Promise(resolve => { scopeReady = resolve; });
  f.root.__elonChatGptPrivateLibraryCatalog = { selectAttachment: () => ({ source, current: () => true }) };
  f.root.__elonChatGptPrivateMountedLibraryAttachment = { create: () => ({
    descriptor: () => remote ? { name: source.name, type: 'text/plain', size: 45 } : null,
    prepare: () => { writes++; return new Promise(resolve => { materialized = resolve; }); },
  }) };
  const owner = library.create(f.root, { composer: f.composer });
  const attach = (requestId = 'mcp_deadline') => owner.attach({ requestId, selected: true,
    value: JSON.stringify({ fileHandle: HANDLE }) }, (...v) => responses.push(v), v => changed.push(v));
  const flush = () => new Promise(resolve => setImmediate(resolve));
  async function advance(ms, fire = true) {
    now += ms;
    if (fire) for (const [id, task] of [...tasks]) if (task.at <= now) { tasks.delete(id); task.fn(); }
    await flush();
  }
  return { ...f, owner, attach, tasks, responses, changed, advance, flush,
    writes: () => writes,
    scope: () => scopeReady({ conversationId: cid, ordinary: true }),
    materialize: () => materialized({ descriptor: { name: source.name, type: 'text/plain', size: 45 },
      item: owner.ready(source, 'mcp_deadline') }),
  };
}

test('slow scope plus mounted preparation remains pending past the old UI and command limits', async () => {
  const f = setup(), pending = f.attach();
  await f.advance(9000); f.scope(); await f.flush();
  await f.advance(11500);
  assert.equal(f.owner.busy(), true);
  assert.equal(f.responses.length, 0);
  f.materialize(); await pending;
  assert.deepEqual(f.responses[0], ['attach_library_file', true, 'library_attachment_associated']);
  assert.equal(f.changed.length, 1); assert.equal(f.store.files$().length, 1);
  assert.equal(f.tasks.size, 0);
});

test('whole-operation deadline releases the owner even if a provider ignores cancellation', async () => {
  const f = setup(), pending = f.attach(), duplicate = f.attach();
  f.scope(); await f.flush();
  await f.advance(24000);
  assert.equal(f.owner.busy(), false);
  await Promise.all([pending, duplicate]);
  assert.equal(f.responses.length, 2);
  assert.ok(f.responses.every(row => row[1] === false));
  assert.equal(f.tasks.size, 0);
  f.materialize(); await f.flush();
  assert.equal(f.store.files$().length, 0); assert.equal(f.changed.length, 0);
  await f.attach('mcp_next');
  assert.equal(f.writes(), 1, 'unknown remote write must not be repeated');
});

test('explicit cancellation resolves the receipt without waiting for an uncooperative provider', async () => {
  const f = setup(), pending = f.attach(); f.scope(); await f.flush();
  f.owner.cancel(); await f.flush();
  assert.equal(f.owner.busy(), false);
  await pending;
  f.materialize(); await f.flush();
  assert.equal(f.responses[0][1], false);
  assert.equal(f.changed.length, 0); assert.equal(f.tasks.size, 0);
});

test('a delayed JS timer cannot publish a mounted result after the monotonic deadline', async () => {
  const f = setup(), pending = f.attach(); f.scope(); await f.flush();
  await f.advance(24001, false); f.materialize(); await pending;
  assert.equal(f.responses[0][1], false);
  assert.equal(f.changed.length, 0); assert.equal(f.tasks.size, 0);
});

test('a delayed timer cannot stage an ordinary reference after its scope read exceeds the deadline', async () => {
  const f = setup(false), pending = f.attach();
  await f.advance(24001, false); f.scope(); await pending;
  assert.equal(f.responses[0][1], false);
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.writes(), 0); assert.equal(f.tasks.size, 0);
});

test('native observation and command budgets outlive the page operation without changing other command limits', () => {
  const fs = require('node:fs'), path = require('node:path');
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  const policy = read('WebChatLibraryAttachmentReceiptPolicy.kt');
  assert.equal(library.operationTimeoutMs, 24000);
  assert.match(policy, /OPERATION_TIMEOUT_MS = 24_000L/);
  assert.match(policy, /COMMAND_TIMEOUT_MS = 30_000L/);
  assert.match(policy, /OBSERVATION_TIMEOUT_MS = 31_000L/);
  assert.match(read('chatgptweb/ChatGptWebObservedState.kt'), /action == "attach_library_file"[\s\S]*?WebChatLibraryAttachmentReceiptPolicy\.COMMAND_TIMEOUT_MS/);
  assert.match(read('chatgptweb/ChatGptWebObservedState.kt'), /const val COMMAND_TIMEOUT_MS = 20_000L/);
});
