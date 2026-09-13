'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const base = '../android/app/src/main/assets/';
const blocks = require(base + 'chatgpt_web_text_blocks.js');
const stream = require(base + 'chatgpt_web_private_stream_policy.js');
const history = require(base + 'chatgpt_web_private_history_projection.js').create({ streamPolicy: stream });

function message(status = 'finished_successfully') {
  return { id: 'answer-1', author: { role: 'assistant' }, status,
    content: { content_type: 'text', parts: [':::writing{id="block-1" variant="standard"}\nBody\n:::'] },
    metadata: { writing_blocks: { 'block-1': { id: 'block-1', index: '0', variant: 'standard', content: 'Saved\n' } } } };
}

test('closed blocks are not complete documents until the provider finishes the message', () => {
  for (const status of ['in_progress', 'incomplete', 'cancelled', 'failed', 'unknown', '']) {
    const input = message(status);
    input.content.parts[0] += '\n```python\n  return 1\n```';
    for (const projected of [blocks.project(input).parts, stream.assistantFrame({ message: input }).blockParts,
      history.project({ messages: [input] })[0].content.filter(part => part.textBlock)]) {
      assert.equal(projected.length, 2);
      assert.ok(projected.every(part => part.textBlock.complete === false), status);
      assert.ok(projected.every(part => part.textBlock.sourceMessageId === undefined), status);
    }
  }
});

test('all reviewed completion states retain normal editing and export admission', () => {
  for (const status of ['finished_successfully', 'completed', 'finished']) {
    const input = message(status);
    input.content.parts[0] += '\n```python\n  return 1\n```';
    assert.ok(blocks.project(input).parts.every(part => part.textBlock.complete));
    assert.equal(blocks.project(input, true).writeSources.length, 1);
  }
});

test('text and typed representations agree on malformed or mismatched saved ownership', () => {
  for (const change of [
    saved => { saved.id = 'another-block'; },
    saved => { saved.index = '1'; },
    saved => { saved.index = {}; },
    saved => { saved.index = [0]; },
    saved => { saved.variant = 'unreviewed'; },
    saved => { saved.library_file_id = ''; },
    saved => { saved.metadata = []; },
    saved => { saved.metadata = 'invalid'; }
  ]) {
    const input = message(); change(input.metadata.writing_blocks['block-1']);
    const widget = structuredClone(input);
    widget.content.parts = [''];
    widget.metadata.content_references = [{ type: 'client_defined_widget', category: 'writing_block',
      data: { id: 'block-1', variant: 'standard', content: 'Body\n' } }];
    for (const row of [input, widget]) {
      const before = JSON.stringify(row), projected = blocks.project(row, true);
      assert.equal(projected.parts[0].textBlock.content, 'Saved\n');
      assert.equal(projected.parts[0].textBlock.complete, true);
      assert.equal(projected.parts[0].textBlock.sourceMessageId, undefined);
      assert.deepEqual(projected.writeSources, []);
      assert.equal(JSON.stringify(row), before);
    }
  }
});

test('malformed stored values cannot be laundered into an absent save record', () => {
  for (const saved of [false, 0, '', []]) {
    const input = message(); input.metadata.writing_blocks['block-1'] = saved;
    const widget = structuredClone(input);
    widget.content.parts = [''];
    widget.metadata.content_references = [{ type: 'client_defined_widget', category: 'writing_block',
      data: { id: 'block-1', variant: 'standard', content: 'Body\n' } }];
    for (const row of [input, widget]) {
      const projected = blocks.project(row, true);
      assert.equal(projected.parts[0].textBlock.content, 'Body\n');
      assert.equal(projected.parts[0].textBlock.complete, true);
      assert.equal(projected.parts[0].textBlock.sourceMessageId, undefined);
      assert.deepEqual(projected.writeSources, []);
    }
  }
});

test('unsupported original variants and library-linked wrappers remain local-only', () => {
  for (const attributes of ['variant="unreviewed"', 'variant="standard" library_file_id="libfile_sample"']) {
    const input = message(); input.metadata = {};
    input.content.parts = [':::writing{id="block-1" ' + attributes + '}\nBody\n:::'];
    const projected = blocks.project(input, true);
    assert.equal(projected.parts[0].textBlock.content, 'Body\n');
    assert.equal(projected.parts[0].textBlock.complete, true);
    assert.equal(projected.parts[0].textBlock.sourceMessageId, undefined);
    assert.deepEqual(projected.writeSources, []);
  }
});

test('missing optional saved identity and exact current identity remain writable', () => {
  for (const saved of [{ content: '' }, { id: 'block-1', index: 0, content: '' },
    { id: 'block-1', index: '0', content: '', metadata: { tag: 'retained' } }]) {
    const input = message(); input.metadata.writing_blocks['block-1'] = saved;
    const projected = blocks.project(input, true);
    assert.equal(projected.writeSources.length, 1);
    assert.equal(projected.writeSources[0].content, '');
    assert.deepEqual(projected.writeSources[0].metadata, saved.metadata ?? {});
  }
});
