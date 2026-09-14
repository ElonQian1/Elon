'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const blocks = require('../android/app/src/main/assets/chatgpt_web_text_blocks.js');
const stream = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy.js');
const history = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js').create({ streamPolicy: stream });

const message = (header, status = 'finished_successfully') => ({ id: 'answer-1', author: { role: 'assistant' },
  content: { content_type: 'text', parts: [header + '\r\n  First\r\n\r\n\tLast  \r\n:::\r\nAfter'] },
  metadata: {}, status });

test('quoted braces remain part of a complete local writing block, not a header terminator', () => {
  for (const [attributes, title] of [
    ['title="Map {key} to {value}"', 'Map {key} to {value}'],
    ["title='Close } first'", 'Close } first'],
    ['title="Escaped \\"quote\\" and }"', 'Escaped "quote" and }'],
    ['subject="Body with }" variant="email"', 'Body with }']
  ]) {
    const input = message(':::writing{id="writing-1" ' + attributes + '}');
    const before = JSON.stringify(input);
    const result = blocks.project(input, true);
    assert.equal(result.parts.length, 1, attributes);
    const block = result.parts[0].textBlock;
    assert.equal(block.kind, 'writing');
    assert.equal(block.title, title);
    assert.equal(block.content, '  First\r\n\r\n\tLast  \r\n');
    assert.equal(block.complete, true);
    assert.equal(block.sourceMessageId, undefined, 'local compatibility is not cloud write authority');
    assert.deepEqual(result.writeSources, []);
    assert.equal(result.text, block.content + 'After');
    assert.equal(JSON.stringify(input), before);
  }
});

test('saved content, historical reads and streaming projection keep one identical local block', () => {
  const input = message(':::writing{id="writing-1" title="{name}" variant="standard"}');
  input.metadata.writing_blocks = { 'writing-1': { content: '', title: 'Saved } title' } };
  const projected = blocks.project(input);
  const frame = stream.assistantFrame({ message: input });
  const row = history.project({ messages: [input] })[0];
  assert.equal(projected.parts.length, 1);
  assert.equal(projected.parts[0].textBlock.content, '');
  assert.equal(projected.parts[0].textBlock.title, 'Saved } title');
  assert.deepEqual(frame.blockParts, projected.parts);
  assert.deepEqual(row.content.filter(part => part.textBlock), projected.parts);
});

test('expanded local headers do not authorize saves for potentially shifted provider indexes', () => {
  const input = message(':::writing{id="first" title="Part }" variant="standard"}');
  input.content.parts[0] += '\n:::writing{id="second" variant="standard"}\nSecond\n:::';
  const result = blocks.project(input, true);
  assert.deepEqual(result.parts.map(part => part.textBlock.id), ['first', 'second']);
  assert.ok(result.parts.every(part => part.textBlock.complete));
  assert.ok(result.parts.every(part => !part.textBlock.sourceMessageId));
  assert.deepEqual(result.writeSources, []);
});

test('partial and streaming headers stay read-only and malformed attributes stay ordinary text', () => {
  const header = ':::writing{id="x" title="Close }"}';
  assert.equal(blocks.project(message(header, 'in_progress')).parts[0]?.textBlock.complete, false);
  const unfinished = message(header);
  unfinished.content.parts = [header + '\nPartial'];
  assert.equal(blocks.project(unfinished).parts[0]?.textBlock.complete, false);
  for (const invalid of [
    ':::writing{id="x" title="Unclosed }',
    ':::writing{id="x" title=bad}tail}',
    ':::writing{id="x" title="Close }" id="y"}',
    ':::writing{id="x" title="Close }"} trailing',
    ':::writing{id="x" title="Close }"'
  ]) {
    const input = message(invalid);
    const result = blocks.project(input, true);
    assert.deepEqual(result.parts, [], invalid);
    assert.deepEqual(result.writeSources, [], invalid);
    assert.equal(result.text, input.content.parts[0]);
  }
});

test('literal headers inside code remain code and existing supported headers keep their cloud identity', () => {
  const input = message(':::writing{id="x" title="Part }"}');
  const raw = input.content.parts[0];
  input.content.parts = ['````md\n' + raw + '\n````'];
  const result = blocks.project(input, true);
  assert.equal(result.parts.length, 1);
  assert.equal(result.parts[0].textBlock.kind, 'code');
  assert.equal(result.parts[0].textBlock.content, raw + '\n');
  const supported = blocks.project(message(':::writing{id="x" title="Part" variant="standard"}'), true);
  assert.equal(supported.writeSources.length, 1);
  assert.equal(supported.parts[0].textBlock.sourceMessageId, 'answer-1');
});
