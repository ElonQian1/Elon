'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-rspack.cjs');
const messagesApi = require('../android/app/src/main/assets/chatgpt_web_rspack_messages.js');
const projectionApi = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy.js');
const ID = '22222222-2222-4222-8222-222222222222';

function message(id, role, text, status = 'finished_successfully') {
  return { id, author: { role }, content: { content_type: 'text', parts: [text] }, status };
}
async function setup() {
  const f = fixture();
  f.page.location = new URL('https://chatgpt.com/c/' + ID);
  f.values.set(f.conversation.i, ID);
  f.conversation.G = { scope: f.cache.c3.exports.a };
  f.mapping = { root: { id: 'root', parent: null, message: null },
    user: { id: 'user', parent: 'root', message: message('user', 'user', 'fixture prompt') },
    assistant: { id: 'assistant', parent: 'user', message: message('assistant', 'assistant', '**fixture answer**') } };
  f.values.set(f.conversation.G, f.mapping);
  f.values.set(f.conversation.z, 'assistant');
  f.page.__elonChatGptPrivateHistoryProjection = projectionApi;
  f.page.__elonChatGptPrivateStreamPolicy = policy;
  f.reader = messagesApi.create(f.page);
  await f.page.__elonChatGptRspackRuntime.load();
  return f;
}

test('reads the current official branch without a DOM turn, fetch or write', async () => {
  const f = await setup();
  const result = f.reader.read(f.editor);
  assert.deepEqual(result.messages.map(m => [m.id, m.role, m.content[0].text]), [
    ['user', 'user', 'fixture prompt'], ['assistant', 'assistant', '**fixture answer**']]);
  assert.equal(result.observedCount, 2);
  assert.equal(result.streaming, false);
  assert.deepEqual(f.calls, []);
  assert.ok(!JSON.stringify(result).includes('not-a-credential'));
});

test('temporary local owner keeps its answer after receiving a server conversation ID', async () => {
  const f = await setup();
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true');
  assert.equal(f.reader.read(f.editor).messages[1].content[0].text, '**fixture answer**');
  assert.equal(f.values.get(f.conversation.i), ID);
});

test('temporary root can read the exact committed server owner without allowing another fresh send', async () => {
  const f = await setup();
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true');
  f.parent.memoizedProps.conversationId = ID;
  assert.equal(f.reader.read(f.editor).messages[1].content[0].text, '**fixture answer**');
  assert.equal(f.context.capture(f.editor), null);
  assert.equal(f.context.state().code, 'conversation_mismatch');
  f.parent.memoizedProps.conversationId = '33333333-3333-4333-8333-333333333333';
  assert.equal(f.reader.read(f.editor), null);
});

test('group read diagnostics contain only changed fixed codes after acceptance', async () => {
  const f = await setup(), events = [];
  f.page.__elonChatGptGroupReadDiagnosticsEnabled = true;
  f.page.__elonChatGptRspackSubmit = { state: () => ({ code: 'accepted' }) };
  f.page.elonChatGptNative = { postMessage: raw => events.push(JSON.parse(raw)) };
  f.values.set(f.conversation.z, 'missing');
  f.reader.read(f.editor); f.reader.read(f.editor);
  assert.deepEqual(events, [{ type: 'browser_diagnostic', kind: 'rspack_read_branch_missing', detail: 'rspack_read_branch_missing' }]);
  f.values.set(f.conversation.z, 'assistant');
  f.reader.read(f.editor);
  assert.equal(events.at(-1).kind, 'rspack_read_ready');
  assert.ok(!JSON.stringify(events).includes('fixture prompt'));
  f.page.__elonChatGptGroupReadDiagnosticsEnabled = false;
  f.values.set(f.conversation.z, 'missing'); f.reader.read(f.editor);
  assert.equal(events.length, 2);
});

test('streaming reads are allowed without relaxing the send guard', async () => {
  const f = await setup();
  f.values.set(f.conversation.T, 'streaming');
  f.values.set(f.composer.h, true);
  f.mapping.assistant.message.status = 'in_progress';
  assert.equal(f.context.capture(f.editor), null);
  assert.equal(f.context.state().code, 'busy');
  const result = f.reader.read(f.editor);
  assert.equal(result.streaming, true);
  assert.equal(result.messages[1].state, 'streaming');
  f.mapping.assistant.message.content.parts[0] += ' continued';
  assert.equal(f.reader.read(f.editor).messages[1].content[0].text, '**fixture answer** continued');
});

test('read does not wait for a draft, attachment or tool to clear', async () => {
  const f = await setup();
  f.values.set(f.composer.r, 'unsent draft');
  f.values.set(f.composer.f, [{ id: 'fixture-upload' }]);
  f.values.set(f.composer.u, ['fixture-tool']);
  assert.equal(f.context.capture(f.editor), null);
  assert.equal(f.reader.read(f.editor).messages.length, 2);
  assert.equal(f.values.get(f.composer.r), 'unsent draft');
});

test('does not expose alternate branches, hidden messages or analysis', async () => {
  const f = await setup();
  f.mapping.alt = { id: 'alt', parent: 'user', message: message('alt', 'assistant', 'not selected') };
  f.mapping.analysis = { id: 'analysis', parent: 'user', message: {
    ...message('analysis', 'assistant', 'internal thought'), channel: 'analysis' } };
  f.mapping.assistant.parent = 'analysis';
  assert.deepEqual(f.reader.read(f.editor).messages.map(m => m.id), ['user', 'assistant']);
});

for (const [name, mutate] of [
  ['identity switch', f => f.switchAccount()],
  ['account mismatch', f => f.changeAccount()],
  ['wrong conversation', f => f.values.set(f.conversation.i, 'other')],
  ['unsupported project route', f => { f.page.location = new URL('https://chatgpt.com/g/g-p-fixture/c/' + ID); }],
  ['detached composer', f => { f.editor.isConnected = false; }],
  ['unreviewed mapping atom', f => { f.conversation.G.scope = {}; }],
  ['unknown streaming state', f => f.values.set(f.conversation.T, 'unknown')],
  ['missing branch', f => f.values.set(f.conversation.z, 'absent')],
  ['cycle in branch', f => { f.mapping.user.parent = 'assistant'; }],
  ['incomplete branch', f => { f.mapping.user.parent = 'missing'; }],
]) test('returns no authoritative snapshot for ' + name, async () => {
  const f = await setup(); mutate(f);
  assert.equal(f.reader.read(f.editor), null);
  assert.equal(f.calls.length, 0);
});

test('identity recheck rejects a switch during projection', async () => {
  const f = await setup();
  f.page.__elonChatGptPrivateHistoryProjection = { create: () => ({
    sourceMessages: () => [{}], project: () => { f.changeAccount(); return [{}]; } }) };
  f.reader = messagesApi.create(f.page);
  assert.equal(f.reader.read(f.editor), null);
});

test('cold runtime warms once and requests a snapshot without a timer loop', async () => {
  const f = fixture();
  f.reader = messagesApi.create(f.page);
  let notified = 0;
  for (let i = 0; i < 4; i++) assert.equal(f.reader.read(f.editor, () => notified++), null);
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(notified, 1);
  assert.equal(f.imports.length, 1);
  assert.equal(f.calls.length, 0);
});
