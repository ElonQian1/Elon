'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request').create({ crypto });
const context = { conversationId: '11111111-1111-4111-8111-111111111111',
  parentId: '22222222-2222-4222-8222-222222222222', model: 'fixture-model',
  historyDisabled: false, doNotRemember: false };
const command = { requestId: 'mcp_tools', prompt: 'Synthetic test prompt' };
const preparation = { conduit_token: 'fixture-conduit' };
const security = { chatReq: { token: 'fixture-token' } };
const headers = () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'fixture-token' });

for (const tool of [null, 'search', 'picture_v2']) {
  test((tool || 'plain') + ' separates prepare, security and final wire metadata', () => {
    const selected = tool ? [tool] : [], dispatched = tool === 'search' ? [] : selected;
    const request = requests.create({ ...context, tool }, command);
    assert.deepEqual(request.preparationBody().system_hints, selected);
    assert.deepEqual(request.securityMetadata(), { systemHints: dispatched });
    const result = request.consume(preparation, security, headers, () => true);
    assert.deepEqual(result.body.system_hints, dispatched);
    assert.deepEqual(result.body.messages[0].metadata, tool ? { system_hints: selected } : undefined);
    assert.equal(result.body.force_use_search, tool === 'search' ? true : undefined);
    assert.equal(result.body.client_reported_search_source, tool === 'search' ? 'conversation_composer_web_icon' : undefined);
    assert.equal(result.body.enable_message_followups, tool ? true : undefined);
    assert.equal(result.body.model, context.model);
    assert.equal(result.body.conversation_id, context.conversationId);
    assert.equal(result.body.parent_message_id, context.parentId);
    assert.deepEqual(result.body.messages[0].content, { content_type: 'text', parts: [command.prompt] });
    assert.equal(result.body.messages.length, 1);
    assert.throws(() => request.consume(preparation, security, headers, () => true), /preparation_consumed/);
  });
}

test('tool hints are fresh immutable snapshots, not a previous request or mutable provider array', () => {
  const input = { ...context, tool: 'picture_v2' }, request = requests.create(input, command);
  input.tool = 'search';
  request.preparationBody().system_hints.push('unknown');
  request.securityMetadata().systemHints.push('unknown');
  const result = request.consume(preparation, security, headers, () => true);
  assert.deepEqual(result.body.system_hints, ['picture_v2']);
  assert.deepEqual(result.body.messages[0].metadata.system_hints, ['picture_v2']);
  result.body.system_hints.length = 0;
  assert.deepEqual(request.securityMetadata().systemHints, ['picture_v2']);
});

test('unknown, multiple and non-string tools are never silently sent as plain text', () => {
  for (const tool of ['canvas', 'tatertot', 'agent', 'search,picture_v2', ['search'], {}, true, '']) {
    assert.throws(() => requests.create({ ...context, tool }, command), /context_invalid/);
  }
});

test('a changed owner rejects tool dispatch before consuming fresh credentials', () => {
  const request = requests.create({ ...context, tool: 'search' }, command);
  let securityRead = false;
  assert.throws(() => request.consume(preparation, security, () => { securityRead = true; return headers(); }, () => false), /context_changed/);
  assert.equal(securityRead, false);
});
