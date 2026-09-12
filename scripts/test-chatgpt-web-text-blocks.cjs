'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const blocks = require('../android/app/src/main/assets/chatgpt_web_text_blocks.js');
const stream = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy.js');
const history = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js').create({ streamPolicy: stream });
const message = (text, metadata = {}, status = 'finished_successfully') => ({ id: 'answer-1', author: { role: 'assistant' },
  content: { content_type: 'text', parts: [text] }, metadata, status });

test('real writing delimiters and saved metadata are resolved together without modifying source', () => {
  const input = message('Before\n:::writing{id="block-1" title="Draft" variant="email"}\nOld\n:::\nAfter',
    { writing_blocks: { 'block-1': { content: '  Saved\n\n\ntext  ', title: 'Edited title' } } });
  const before = JSON.stringify(input);
  const result = blocks.project(input);
  assert.equal(result.text, 'Before\n  Saved\n\n\ntext  \nAfter');
  assert.equal(result.parts[0].textBlock.content, '  Saved\n\n\ntext  ');
  assert.equal(result.parts[0].textBlock.title, 'Edited title');
  assert.equal(result.parts[0].textBlock.id, 'block-1');
  assert.equal(JSON.stringify(input), before);
});

test('fenced source is byte-preserving including CRLF, indentation and empty lines', () => {
  const source = '  print("hello")\r\n\r\n\r\n\tprint("next")  \r\n';
  const result = blocks.project(message('```python\r\n' + source + '```'));
  assert.equal(result.parts[0].textBlock.content, source);
  assert.equal(result.parts[0].language, 'python');
  assert.equal(result.parts[0].textBlock.complete, true);
});

test('writing syntax inside source code is not a writing block', () => {
  const result = blocks.project(message('````md\n:::writing{id="x"}\n```\n:::\n````'));
  assert.deepEqual(result.parts.map(p => p.type), ['code']);
  assert.match(result.parts[0].textBlock.content, /:::writing/);
});

test('code fences shield closing directives within a writing block', () => {
  const result = blocks.project(message(':::writing{id="x"}\n~~~md\n:::\n~~~\nEnd\n:::'));
  assert.equal(result.parts.length, 1);
  assert.equal(result.parts[0].textBlock.content, '~~~md\n:::\n~~~\nEnd\n');
  assert.equal(result.parts[0].textBlock.complete, true);
});

test('known nested context lists close at the outer boundary; unknown nested types stay read-only', () => {
  const raw = ':::writing{id="x"}\n:::contextList{}\nNested\n:::\nRemaining\n:::';
  const block = blocks.project(message(raw)).parts[0].textBlock;
  assert.equal(block.content, ':::contextList{}\nNested\n:::\nRemaining\n');
  assert.equal(block.complete, true);
  assert.equal(blocks.project(message(raw.replace('contextList', 'unknown'))).parts[0].textBlock.complete, false);
});

test('two blocks keep distinct bodies; metadata for another id is never used', () => {
  const result = blocks.project(message(':::writing{id="x"}\nOne\n:::\n:::writing{id="y"}\nTwo\n:::',
    { writing_blocks: { z: { content: 'Unrelated' } } }));
  assert.deepEqual(result.parts.map(p => p.textBlock.content), ['One\n', 'Two\n']);
  assert.deepEqual(result.parts.map(p => p.textBlock.id), ['x', 'y']);
});

test('an empty saved block is authoritative, not a signal to restore old text', () => {
  assert.equal(blocks.project(message(':::writing{id="x"}\nOld\n:::',
    { writing_blocks: { x: { content: '' } } })).parts[0].textBlock.content, '');
});

test('partial writing cannot become editable even after stream completion', () => {
  for (const state of ['in_progress', 'finished_successfully']) {
    const result = blocks.project(message(':::writing{id="x"}\nPartial', {}, state));
    assert.equal(result.parts[0].textBlock.complete, false);
    assert.match(result.text, /:::writing/);
  }
});

test('unclosed CommonMark code is final only once the message finishes', () => {
  assert.equal(blocks.project(message('```js\nlet x =', {}, 'in_progress')).parts[0].textBlock.complete, false);
  assert.equal(blocks.project(message('```js\nlet x = 1')).parts[0].textBlock.complete, true);
});

test('bounds refuse an editable truncated document', () => {
  const raw = '```\n' + 'x'.repeat(blocks.MAX_CONTENT + 1) + '\n```';
  assert.equal(blocks.project(message(raw)).parts.length, 0);
  assert.equal(blocks.project(message(raw)).text, raw);
  assert.equal(blocks.domCode('x'.repeat(blocks.MAX_CONTENT + 1), '', 0), null);
});

test('malformed attributes, prototype keys, and indented literals stay inert', () => {
  assert.equal(blocks.project(message(':::writing{id="x" id="y"}\nHello\n:::')).parts.length, 0);
  assert.equal(blocks.project(message('    :::writing{id="x"}\n    Literal\n    :::')).parts.length, 0);
  const inherited = Object.create({ x: { content: 'Other' } });
  assert.equal(blocks.project(message(':::writing{id="x"}\nActual\n:::', { writing_blocks: inherited }))
    .parts[0].textBlock.content, 'Actual\n');
});

test('history projection and SSE projection agree on the original structured body', () => {
  const input = message(':::writing{id="x"}\nFirst\n:::', { writing_blocks: { x: { content: 'Saved\n\n\nbody' } } });
  const row = history.project({ messages: [input] })[0];
  const frame = stream.assistantFrame({ message: input });
  assert.deepEqual(row.content.find(p => p.type === 'writing_block'), frame.blockParts[0]);
  assert.doesNotMatch(row.content[0].text, /First|:::writing/);
  assert.equal(row.content[0].text, frame.text);
});

test('new streaming message and subsequent snapshots both preserve block operations without duplication', () => {
  const input = message('```js\n  return 1;\n```');
  const frame = stream.assistantFrame({ message: input });
  const first = stream.mergeMessages([], frame);
  assert.equal(first[0].content.filter(p => p.type === 'code').length, 1);
  const next = stream.mergeMessages(first, frame);
  assert.equal(next[0].content.filter(p => p.type === 'code').length, 1);
  assert.equal(next[0].content.find(p => p.type === 'code').textBlock.content, '  return 1;\n');
});

test('edited shorter writing text is not overwritten by the old longest DOM text', () => {
  const frame = stream.assistantFrame({ message: message(':::writing{id="x"}\nShort\n:::') });
  const before = [{ id: 'answer-1', role: 'assistant', content: [{ type: 'markdown', text: 'Much longer stale content' }] }];
  assert.equal(stream.mergeMessages(before, frame)[0].content[0].text, frame.text);
});

test('ordinary Markdown remains ordinary, and mixed multimodal text is not guessed', () => {
  assert.deepEqual(blocks.project(message('## Heading\nText')).parts, []);
  const input = message('text'); input.content.parts.push({ text: 'not a code block' });
  assert.equal(blocks.project(input), null);
});

test('typed writing widgets work without a textual wrapper and unrelated widgets are not promoted', () => {
  const reference = { type: 'client_defined_widget', category: 'writing_block', data: { id: 'x', content: 'Original', title: 'Note' } };
  const input = message('', { content_references: [reference, { ...reference, category: 'other' }] });
  const frame = stream.assistantFrame({ message: input });
  assert.equal(frame.blockParts.length, 1);
  assert.equal(stream.mergeMessages([], frame)[0].content.find(p => p.type === 'writing_block').textBlock.content, 'Original');
  assert.equal(history.project({ messages: [input] })[0].content[0].type, 'writing_block');
});

test('DOM snapshots reuse current structured writing state without imports, requests or composer access', () => {
  const id = '11111111-1111-4111-8111-111111111111';
  const input = message(':::writing{id="x" variant="standard"}\nOld\n:::\n```js\nlet x;\n```',
    { writing_blocks: { x: { content: 'Current\n', variant: 'standard' } } });
  const owner = { id: 'client', serverId$: () => id };
  const page = { location: { href: 'https://chatgpt.com/c/' + id }, __elonChatGptDocumentToken: 'doc_owner',
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => 'account' }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }),
      peek(role) { assert.equal(role, 'shared'); return { canvasConversations: () => [owner],
        XM(key) { assert.equal(key, 'client'); return {}; }, HM: { getNodeIfExists: (_, key) => ({ message: key === input.id ? input : null }) } }; },
      load() { assert.fail('render cannot import'); } },
    fetch() { assert.fail('render cannot fetch'); } };
  const before = JSON.stringify(input);
  const value = blocks.runtimeProjection(page, input.id);
  assert.deepEqual(value.parts.map(part => part.type), ['writing_block', 'code']);
  assert.equal(value.parts[0].textBlock.content, 'Current\n');
  assert.equal(JSON.stringify(input), before);
  assert.equal(blocks.runtimeProjection(page, 'other'), null);
  input.status = 'in_progress'; assert.equal(blocks.runtimeProjection(page, input.id), null);
  input.status = 'finished_successfully';
  page.location.href += '?temporary-chat=true'; assert.equal(blocks.runtimeProjection(page, input.id), null);
  page.location.href = 'https://chatgpt.com/c/' + id;
  page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: 'unknown' });
  assert.equal(blocks.runtimeProjection(page, input.id), null);
});
