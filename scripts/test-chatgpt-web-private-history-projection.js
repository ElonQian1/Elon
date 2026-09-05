'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.join(__dirname, '..');
const assets = path.join(root, 'android/app/src/main/assets');
const moduleApi = require(path.join(assets, 'chatgpt_web_private_history_projection.js'));
const streamPolicy = require(path.join(assets, 'chatgpt_web_private_stream_policy.js'));
const projection = moduleApi.create({ streamPolicy });
const fixture = JSON.parse(fs.readFileSync(path.join(root,
  'android/app/src/test/resources/webchat/private-history-contract.json'), 'utf8'));
const clone = (value) => JSON.parse(JSON.stringify(value));
const message = (id, role, text) => ({ id, author: { role }, content: { parts: [text] } });
let cases = 0;
function test(name, run) { run(); cases++; console.log('PASS ' + name); }

test('same wire fixture consumed by Android retains text, files and citations', () => {
  assert.deepEqual(projection.project(fixture.input), fixture.event.messages);
  assert.doesNotMatch(JSON.stringify(projection.project(fixture.input)), /session=synthetic/);
});
test('wrapped linear history uses the same projection', () => {
  assert.deepEqual(projection.project({ data: { conversation: {
    messages: Object.values(fixture.input.mapping).map((node) => node.message)
  } } }), fixture.event.messages);
});
test('only the selected regeneration branch is displayed', () => {
  const input = clone(fixture.input);
  input.mapping.alternative = { parent: 'question', message: message('other', 'assistant', 'Do not show') };
  assert.deepEqual(projection.project(input), fixture.event.messages);
  delete input.current_node;
  assert.deepEqual(projection.project(input), [], 'ambiguous branch must defer to the official page');
});
test('unique leaf is usable without an explicit current node', () => {
  const input = clone(fixture.input);
  delete input.current_node;
  assert.deepEqual(projection.project(input), fixture.event.messages);
});
test('broken and cyclic ancestry cannot mix unrelated messages', () => {
  const input = clone(fixture.input);
  input.mapping.answer.parent = 'missing';
  assert.deepEqual(projection.project(input), []);
  input.mapping.answer.parent = 'answer';
  assert.deepEqual(projection.project(input), []);
});
test('hidden, analysis and tool-directed messages are not user-facing history', () => {
  const hidden = Object.assign(message('hidden', 'assistant', 'internal'), {
    metadata: { is_visually_hidden_from_conversation: true }
  });
  const analysis = Object.assign(message('analysis', 'assistant', 'internal'), { channel: 'analysis' });
  const tool = Object.assign(message('tool', 'assistant', 'internal'), { recipient: 'python' });
  const visible = message('visible', 'assistant', 'Visible result');
  assert.deepEqual(projection.project({ messages: [hidden, analysis, tool, visible] }).map((x) => x.id), ['visible']);
});
test('image-only messages survive without leaking file handles or signed URLs', () => {
  const image = message('image', 'user', '');
  image.content.parts = [{ content_type: 'image_asset_pointer',
    asset_pointer: 'file-service://private-id', width: 512, height: 512 }];
  const result = projection.project({ messages: [image] });
  assert.equal(result.length, 1);
  assert.equal(result[0].content[0].type, 'image');
  assert.equal(result[0].content[0].imageWidth, 512);
  assert.doesNotMatch(JSON.stringify(result), /file-service|private-id/);
});
test('file-only messages survive and metadata stays bounded', () => {
  const file = message('file', 'user', '');
  file.metadata = { attachments: [{ name: 'a'.repeat(300), mime_type: 'invalid type',
    id: 'secret', download_url: 'https://example.com/?token=private' }] };
  const result = projection.project({ messages: [file] });
  assert.equal(result[0].content[0].text.length, 180);
  assert.equal(result[0].content[0].mediaType, undefined);
  assert.doesNotMatch(JSON.stringify(result), /secret|token=private/);
});
test('history size, text size and streaming state remain bounded', () => {
  const messages = Array.from({ length: 100 }, (_, i) => message('message-' + i, 'user', 'a'.repeat(25000)));
  messages[99].status = 'in_progress';
  const result = projection.project({ messages });
  assert.equal(result.length, 80);
  assert.equal(result[0].id, 'message-20');
  assert.equal(result[0].content[0].text.length, 20000);
  assert.equal(result.at(-1).state, 'streaming');
});
test('existing finance and chart parsers are reused, not reimplemented', () => {
  const calls = [];
  const custom = moduleApi.create({ streamPolicy: {
    assistantFrame: streamPolicy.assistantFrame,
    financePartsFromMetadata: (metadata) => { calls.push(metadata); return [{ type: 'rich_card', text: 'Finance' }]; },
    clientChartPartFromMetadata: () => ({ type: 'rich_card', text: 'Chart' })
  } });
  const value = message('visible', 'assistant', 'Result');
  value.metadata = { sample: true };
  assert.deepEqual(custom.project({ messages: [value] })[0].content.map((x) => x.type),
    ['markdown', 'rich_card', 'rich_card']);
  assert.deepEqual(calls, [value.metadata]);
});
test('asset registration precedes private history requests', () => {
  const adapter = fs.readFileSync(path.join(root,
    'android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  assert.ok(adapter.indexOf('chatgpt_web_private_history_projection.js') < adapter.indexOf('chatgpt_web_private_transport.js'));
  const desktop = fs.readFileSync(path.join(root,
    'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs'), 'utf8');
  assert.ok(desktop.indexOf('chatgpt_web_private_history_projection.js') < desktop.indexOf('chatgpt_web_private_transport.js'));
});
test('raw widget control tokens never replace visible text', () => {
  const value = message('widget', 'assistant', '\ue200genui\ue202internal-widget-key\ue201');
  assert.deepEqual(projection.project({ messages: [value] }), []);
});
console.log('PRIVATE_HISTORY_PROJECTION_TESTS=' + cases + '_passed');
