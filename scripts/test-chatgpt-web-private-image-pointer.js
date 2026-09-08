'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const parser = require('../android/app/src/main/assets/chatgpt_web_private_image_pointer.js');

test('shared pointer parser preserves metadata ID and immutable normalized query', () => {
  const input = 'file-service://file-one#part?variant=first&label=A+B&variant=last&unicode=%E4%B8%AD';
  const parsed = parser.parse(input);
  assert.equal(parsed.id, input.slice('file-service://'.length));
  assert.equal(parsed.downloadFileId, 'file-one*part');
  assert.deepEqual(parsed.downloadQuery, [['variant', 'last'], ['label', 'A B'], ['unicode', '\u4e2d']]);
  assert.throws(() => parsed.downloadQuery[0][1] = 'changed', TypeError);
  assert.throws(() => parsed.downloadQuery.push(['extra', 'value']), TypeError);
  assert.equal(parser.parse('sediment://file-one').downloadFileId, 'file-one');
});

test('parser rejects scope and credential overrides, paths and malformed encodings', () => {
  for (const key of ['gizmo_id', 'project_id', 'conversation_id', 'post_id',
    'check_context_scopes_for_conversation_id', 'context_scopes', 'download_intent', 'inline',
    'authorization', 'cookie', 'access_token']) {
    assert.equal(parser.parse('file-service://file-one?' + key.toUpperCase() + '=other'), null);
  }
  for (const pointer of [null, {}, '', 'https://example.com/file-one', 'file-service://a/b',
    'sediment://file-one?label=%', 'sediment://file-one?label=%C0%AF',
    'sediment://file-one?label=%00', 'sediment://file-one?1invalid=x',
    'sediment://file-one?label=' + 'x'.repeat(1025), 'sediment://' + 'x'.repeat(161),
    'sediment://file-one?label=' + 'x'.repeat(4096)]) assert.equal(parser.parse(pointer), null);
  assert.equal(parser.parse('sediment://file-one?' + Array(33).fill('a=b').join('&')), null);
  assert.ok(parser.parse('sediment://file-one?' + Array(32).fill('a=b').join('&')));
});

test('pure browser registration has no identity, network or timer side effects', () => {
  const source = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/chatgpt_web_private_image_pointer.js'), 'utf8');
  const root = { location: { origin: 'https://chatgpt.com' } };
  vm.runInNewContext(source, { window: root, URLSearchParams });
  assert.equal(root.__elonChatGptPrivateImagePointer.version, 1);
  assert.equal(root.__elonChatGptPrivateImagePointer.parse('sediment://file-one').downloadFileId, 'file-one');
  const other = { location: { origin: 'https://example.com' } };
  vm.runInNewContext(source, { window: other });
  assert.equal(other.__elonChatGptPrivateImagePointer, undefined);
});

test('production loads one shared parser before its consumers', () => {
  const assets = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const name = 'chatgpt_web_private_image_pointer.js';
  assert.equal(assets.split(name).length - 1, 1);
  for (const consumer of ['chatgpt_web_private_file_download.js', 'chatgpt_web_private_image_gallery.js']) {
    assert.ok(assets.indexOf(name) < assets.indexOf(consumer));
  }
});
