'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const fs = require('node:fs');
const vm = require('node:vm');
const catalog = require('../android/app/src/main/assets/chatgpt_web_private_library_catalog.js');
const mutations = require('../android/app/src/main/assets/chatgpt_web_private_library_mutations.js');
const json = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');

async function fixture(overrides = {}) {
  let auth = 'Bearer synthetic-library-mutation', sequence = 0;
  let reply = { status: 200, ok: true, text: '{"event":"file.deletion.completed"}\n' };
  let listReply = { items: [{ kind: 'file', id: 'libfile_synthetic', file_id: 'file-synthetic', name: 'fixture.txt',
    parent_directory_id: 'directory-synthetic', mime_type: 'text/plain', file_size_bytes: 7, ...overrides }], cursor: null };
  const calls = [], events = [], receipts = [];
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/unchanged' },
    __elonChatGptDocumentToken: 'doc_synthetic_library', crypto: webcrypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: auth }) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init, limits) {
      calls.push({ url: new URL(url), init, limits });
      if (init.method === 'GET') {
        if (listReply instanceof Error) throw listReply;
        return { payload: typeof listReply === 'function' ? await listReply(init) : listReply };
      }
      if (reply instanceof Error) throw reply;
      return typeof reply === 'function' ? reply(init) : reply;
    } },
  };
  const service = root.__elonChatGptPrivateLibraryCatalog = catalog.create(root);
  const writer = root.__elonChatGptPrivateLibraryMutations = mutations.create(root);
  const list = (value = {}) => service.list({ requestId: 'mcp_list' + ++sequence, value: JSON.stringify(value) },
    (_, value) => events.push(value), (...value) => receipts.push(value));
  await list();
  const command = (operation = 'rename', options = {}) => ({ requestId: 'mcp_write' + ++sequence, selected: true,
    value: JSON.stringify({ operation, fileHandle: events[0].items[0].handle, name: 'renamed.txt' }), ...options });
  return { root, service, writer, list, command, calls, events, receipts,
    reply: value => { reply = value; }, listReply: value => { listReply = value; }, auth: value => { auth = value; } };
}

test('rename uses one official PATCH and updates retained cache only after acknowledgement', async () => {
  const f = await fixture();
  assert.equal(f.events[0].items[0].canRename, true);
  assert.equal(f.events[0].items[0].canTrash, true);
  const result = await f.writer.start(f.command());
  assert.equal(result.ok, true);
  const call = f.calls[1];
  assert.equal(call.url.pathname, '/backend-api/files/library/files/libfile_synthetic');
  assert.equal(call.init.method, 'PATCH');
  assert.deepEqual(JSON.parse(call.init.body), { file_name: 'renamed.txt' });
  assert.equal(call.init.redirect, 'error');
  assert.equal(call.limits.timeoutMs, 20000);
  f.listReply(new Error('offline'));
  await f.list();
  assert.equal(f.events.at(-1).items[0].name, 'renamed.txt');
  assert.equal(f.events.at(-1).stale, true);
  assert.equal(f.root.location.href, 'https://chatgpt.com/c/unchanged');
});

test('trash uses the observed soft-delete stream with the exact library and backing-file identities', async () => {
  const f = await fixture();
  assert.equal((await f.writer.start(f.command('trash'))).ok, true);
  const call = f.calls[1];
  assert.equal(call.init.method, 'POST');
  assert.equal(call.url.pathname, '/backend-api/files/library/files/libfile_synthetic/delete_stream');
  assert.deepEqual(Object.fromEntries(call.url.searchParams), { file_id: 'file-synthetic',
    parent_directory_id: 'directory-synthetic', file_name: 'fixture.txt', soft_delete: 'true' });
  assert.equal(call.init.body, undefined);
  assert.equal(call.limits.mode, 'text');
  f.listReply(new Error('offline'));
  await f.list();
  assert.equal(f.events.at(-1).items.length, 0);
});

for (const [name, text, ok] of [
  ['empty HTTP success', '', false],
  ['progress alone', '{"event":"file.deletion.progress"}\n', false],
  ['truncated completion', '{"event":"file.deletion.completed"', false],
  ['error after completion', '{"event":"file.deletion.completed"}\n{"event":"file.deletion.error"}', false],
  ['SSE is not the deletion protocol', 'data: {"event":"file.deletion.completed"}\n\n', false],
  ['multiline JSON completion', '{\n"event":"file.deletion.completed"\n}\n', true],
  ['CRLF and progress', '{"event":"file.deletion.progress"}\r\n{"event":"file.deletion.completed"}\r\n', true],
]) test('delete acknowledgement: ' + name, async () => {
  const f = await fixture();
  f.reply({ text });
  assert.equal((await f.writer.start(f.command('trash'))).ok, ok);
  f.listReply(new Error('offline'));
  await f.list();
  assert.equal(f.events.at(-1).items.length, ok ? 0 : 1);
});

test('duplicate request shares one in-flight write; changed parameters and concurrent writes are rejected', async () => {
  const f = await fixture();
  let finish;
  f.reply(() => new Promise(resolve => { finish = resolve; }));
  const command = f.command(), a = f.writer.start(command), b = f.writer.start(command);
  assert.equal(a, b);
  await new Promise(setImmediate);
  assert.equal((await f.writer.start(f.command())).code, 'library_mutation_busy');
  assert.equal((await f.writer.start({ ...command, value: JSON.stringify({ ...JSON.parse(command.value), name: 'different.txt' }) })).code,
    'library_request_conflict');
  finish({});
  await a;
  assert.equal((await f.writer.start(command)).ok, true);
  assert.equal(f.calls.filter(x => x.init.method === 'PATCH').length, 1);
});

test('timeout preserves rows and imposes cooldown without retrying the write or switching endpoints', async () => {
  const f = await fixture();
  f.reply(new Error('timeout'));
  assert.equal((await f.writer.start(f.command('trash'))).code, 'library_result_unconfirmed');
  assert.equal((await f.writer.start(f.command('trash'))).code, 'library_mutation_cooldown');
  f.listReply(new Error('offline'));
  await f.list();
  assert.equal(f.events.at(-1).items.length, 1);
  assert.equal(f.calls.filter(x => x.init.method !== 'GET').length, 1);
});

test('confirmation, name, and opaque selection validation reject before any HTTP write', async () => {
  const f = await fixture();
  assert.equal((await f.writer.start(f.command('rename', { selected: false }))).code, 'user_confirmation_required');
  for (const name of ['', '..', 'a/b', 'a\\b', 'a\nb', 'a'.repeat(181)]) {
    const command = f.command(); command.value = JSON.stringify({ ...JSON.parse(command.value), name });
    assert.equal((await f.writer.start(command)).code, 'invalid_library_mutation');
  }
  const command = f.command(); command.value = JSON.stringify({ ...JSON.parse(command.value), fileHandle: 'library_' + 'f'.repeat(32) });
  assert.equal((await f.writer.start(command)).code, 'library_selection_expired');
  assert.equal(f.calls.length, 1);
});

test('external, project, saved-entity and folder rows never advertise unsupported writes', async () => {
  for (const extra of [{ external_account: {} }, { cloud_doc_url: 'https://example.invalid/file' },
    { saved_entity: {} }, { library_artifact_type: 'writing_block' }, { is_project: true }, { kind: 'directory' }]) {
    const f = await fixture(extra);
    assert.equal(f.events[0].items[0].canRename, false);
    assert.equal((await f.writer.start(f.command())).ok, false);
    assert.equal(f.calls.length, 1);
  }
  const f = await fixture({ file_id: null });
  assert.equal(f.events[0].items[0].canRename, true);
  assert.equal(f.events[0].items[0].canTrash, false);
});

test('identity or navigation changes reject late success and stale request receipts', async () => {
  for (const change of [f => f.auth('Bearer different-synthetic-account'),
    f => { f.root.location.href = 'https://chatgpt.com/c/other'; },
    f => { f.root.__elonChatGptDocumentToken = 'doc_different_library'; }]) {
    const f = await fixture();
    let finish;
    f.reply(() => new Promise(resolve => { finish = resolve; }));
    const command = f.command('trash'), pending = f.writer.start(command);
    await new Promise(setImmediate);
    change(f);
    finish({ text: '{"event":"file.deletion.completed"}\n' });
    assert.equal((await pending).code, 'library_result_unconfirmed');
    assert.equal((await f.writer.start(command)).code, 'library_request_context_changed');
    assert.equal(f.calls.filter(x => x.init.method === 'POST').length, 1);
  }
});

test('starting a mutation cancels an in-flight stale read; new reads wait until terminal', async () => {
  const f = await fixture();
  let readFinish, writeFinish;
  f.listReply(() => new Promise(resolve => { readFinish = resolve; }));
  const read = f.list({ operation: 'refresh' });
  await new Promise(setImmediate);
  f.reply(() => new Promise(resolve => { writeFinish = resolve; }));
  const write = f.writer.start(f.command());
  await new Promise(setImmediate);
  await f.list();
  assert.equal(f.receipts.at(-1)[2], 'library_mutation_busy');
  readFinish({ items: [], cursor: null });
  await read;
  assert.equal(f.events.at(-1).items.length, 1);
  writeFinish({});
  await write;
});

test('a failed old-account write does not place the newly signed-in account in cooldown', async () => {
  const f = await fixture();
  f.reply(new Error('timeout'));
  await f.writer.start(f.command());
  f.auth('Bearer new-synthetic-account');
  await f.list();
  f.reply({});
  const command = f.command();
  command.value = JSON.stringify({ ...JSON.parse(command.value), fileHandle: f.events.at(-1).items[0].handle });
  assert.equal((await f.writer.start(command)).ok, true);
});

test('real bounded response reader handles chunked NDJSON and rejects HTTP authorization errors', async () => {
  const f = await fixture();
  f.root.__elonChatGptPrivateJsonRequest = json;
  f.root.fetch = async () => new Response(new ReadableStream({ start(controller) {
    for (const text of ['{"event":"file.deletion.', 'completed"}\n']) controller.enqueue(new TextEncoder().encode(text));
    controller.close();
  } }), { status: 200 });
  assert.equal((await f.writer.start(f.command('trash'))).ok, true);
  const rejected = await fixture();
  rejected.root.__elonChatGptPrivateJsonRequest = json;
  rejected.root.fetch = async () => new Response('', { status: 403 });
  assert.equal((await rejected.writer.start(rejected.command())).code, 'library_mutation_http_403');
});

test('directory dispatcher upgrades an older registered version and preserves same-version instances', () => {
  const source = fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8');
  const window = { __elonChatGptConversationDirectoryRequests: { version: 6 } };
  vm.runInNewContext(source, { window });
  assert.equal(window.__elonChatGptConversationDirectoryRequests.version, 7);
  const instance = window.__elonChatGptConversationDirectoryRequests;
  vm.runInNewContext(source, { window });
  assert.equal(window.__elonChatGptConversationDirectoryRequests, instance);
});
