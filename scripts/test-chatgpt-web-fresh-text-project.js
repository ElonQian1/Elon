'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request').create({ crypto });
const CID = '11111111-1111-4111-8111-111111111111';
const PID = '22222222-2222-4222-8222-222222222222';
const PROJECT = 'g-p-' + 'a'.repeat(32);
const base = { conversationId: CID, parentId: PID, model: 'fixture-model',
  historyDisabled: false, doNotRemember: false, projectId: PROJECT };
const command = { requestId: 'mcp_project1', prompt: 'Synthetic project prompt' };
const preparation = { conduit_token: 'fixture-conduit' };
const security = { chatReq: { token: 'fixture-fresh' } };
const securityHeaders = value => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': value.token });

for (const tool of [null, 'search', 'picture_v2']) {
  test('project mode survives prepare/security/dispatch with tool ' + tool, () => {
    const request = requests.create({ ...base, tool }, command);
    const mode = { kind: 'gizmo_interaction', gizmo_id: PROJECT };
    assert.deepEqual(request.preparationBody().conversation_mode, mode);
    assert.deepEqual(request.securityMetadata().conversationMode, mode);
    const prepared = request.preparationBody(); prepared.conversation_mode.gizmo_id = 'changed';
    request.securityMetadata().conversationMode.kind = 'changed';
    const sent = request.consume(preparation, security, securityHeaders, () => true);
    assert.deepEqual(sent.body.conversation_mode, mode);
    assert.deepEqual(sent.body.system_hints, tool && tool !== 'search' ? [tool] : []);
    assert.equal(sent.body.force_use_search, tool === 'search' ? true : undefined);
    assert.deepEqual(sent.body.messages[0].content.parts, [command.prompt]);
    assert.equal('gizmo' in sent.body.conversation_mode, false);
    assert.equal('project_instructions' in sent.body, false);
    sent.body.conversation_mode.gizmo_id = 'changed';
    assert.deepEqual(request.preparationBody().conversation_mode, mode);
    assert.throws(() => request.consume(preparation, security, securityHeaders, () => true), /preparation_consumed/);
  });
}

test('authorized project headers are isolated copies for prepare and stream, never added to stop', () => {
  const supplied = { 'x-openai-locked-chats-pin': 'fixture-pin' };
  const request = requests.create({ ...base, projectHeaders: supplied }, command);
  supplied['x-openai-locked-chats-pin'] = 'changed';
  const headers = request.preparationHeaders(); headers['x-openai-locked-chats-pin'] = 'changed-again';
  assert.equal(request.preparationHeaders()['x-openai-locked-chats-pin'], 'fixture-pin');
  const sent = request.consume(preparation, security, securityHeaders, () => true);
  assert.equal(sent.headers['x-openai-locked-chats-pin'], 'fixture-pin');
  assert.equal(sent.headers['x-conduit-token'], preparation.conduit_token);
  const stopped = request.consumeStop([], () => true);
  assert.deepEqual(Object.keys(stopped.additionalHeaders).sort(), ['x-conduit-token', 'x-oai-turn-trace-id']);
  assert.equal(stopped.requestBody.conversation_id, CID);
});

test('invalid project modes and unreviewed headers fail before fresh preparation', () => {
  for (const patch of [
    { projectId: 'g-custom' }, { projectId: '' }, { projectId: PROJECT + '/c/another' },
    { doNotRemember: true }, { projectHeaders: { authorization: 'fixture' } },
    { projectHeaders: { 'x-openai-locked-chats-pin': '' } },
    { projectHeaders: { 'x-openai-locked-chats-pin': 'fixture\r\nunsafe' } },
    { projectHeaders: { 'x-openai-locked-chats-pin': 'x'.repeat(65537) } },
    { projectHeaders: [] }, { projectHeaders: 'not-headers' },
    { projectId: null, projectHeaders: { 'x-openai-locked-chats-pin': 'fixture' } }
  ]) assert.throws(() => requests.create({ ...base, ...patch }, command), /context_invalid/);
});

test('ordinary fresh text keeps its accepted mode and security metadata', () => {
  const request = requests.create({ ...base, projectId: null }, command);
  assert.deepEqual(request.preparationBody().conversation_mode, { kind: 'primary_assistant' });
  assert.deepEqual(request.preparationHeaders(), {});
  assert.deepEqual(request.securityMetadata(), { systemHints: [] });
});
