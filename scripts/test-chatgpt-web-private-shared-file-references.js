'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const projectionModule = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const projection = projectionModule.create({});
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');

function reference(id = 'libfile_synthetic') {
  return { library_file_id: id, name: 'shared.txt', mime_type: 'text/plain',
    size_bytes: 42, display_path: '/Shared/shared.txt', entrypoint: 'library' };
}
function message(id = 'question', refs = [reference()]) {
  return { id, author: { role: 'user' }, content: { parts: [] },
    metadata: { shared_library_file_references: refs } };
}

test('metadata-only shared files appear in native history and the selected conversation file index', () => {
  const payload = { messages: [message()] };
  assert.deepEqual(projection.project(payload)[0]?.content,
    [{ type: 'file', kind: 'file', text: 'shared.txt', mediaType: 'text/plain' }]);
  const index = projection.files(payload);
  assert.deepEqual(index, { files: [{ id: 'question:0', messageId: 'question', role: 'user',
    name: 'shared.txt', kind: 'file', mediaType: 'text/plain' }], truncated: false });
  assert.deepEqual(projection.fileSource(payload, index.files[0].id), {
    sharedLibraryReference: reference(), name: 'shared.txt', projectId: '',
  });
  assert.doesNotMatch(JSON.stringify({ index, messages: projection.project(payload) }),
    /libfile|display_path|entrypoint|Shared\//);
});

test('ordinary attachment source takes precedence and shared references do not duplicate visible files', () => {
  const row = message('question', [reference(), reference('libfile_next'), reference('libfile_next')]);
  row.metadata.attachments = [{ id: 'file-copy', library_file_id: 'libfile_synthetic', name: 'copy.txt' }];
  const payload = { messages: [row] }, index = projection.files(payload);
  assert.deepEqual(index.files.map(file => file.name), ['copy.txt', 'shared.txt']);
  assert.equal(projection.fileSource(payload, 'question:0').attachment.id, 'file-copy');
  assert.equal(projection.fileSource(payload, 'question:1').sharedLibraryReference.library_file_id, 'libfile_next');
});

test('mixed images, ordinary files and shared metadata retain the same source-to-row positions', () => {
  const row = message();
  row.content.parts.push({ content_type: 'image_asset_pointer', asset_pointer: 'file-service://file-image' });
  row.metadata.attachments = [{ id: 'file-ordinary', name: 'ordinary.txt' }];
  const payload = { messages: [row] };
  assert.deepEqual(projection.files(payload).files.map(file => file.id), ['question:0', 'question:1', 'question:2']);
  assert.ok(projection.fileSource(payload, 'question:0').image);
  assert.ok(projection.fileSource(payload, 'question:1').attachment);
  assert.ok(projection.fileSource(payload, 'question:2').sharedLibraryReference);
});

test('shared files preserve selected-branch, hidden-message and historical-window boundaries', () => {
  const payload = { current_node: 'selected', mapping: {
    root: { message: message('root'), parent: null },
    selected: { message: message('selected', [reference('libfile_selected')]), parent: 'root' },
    alternate: { message: message('alternate', [reference('libfile_alternate')]), parent: 'root' },
  } };
  payload.mapping.root.message.metadata.is_visually_hidden_from_conversation = true;
  assert.deepEqual(projection.files(payload).files.map(row => row.messageId), ['selected']);
  assert.equal(projection.fileSource(payload, 'alternate:0'), null);
  const older = { messages: [message('old'), ...Array.from({ length: 85 }, (_, i) => ({
    id: 'later-' + i, author: { role: 'assistant' }, content: { parts: ['later'] },
  }))] };
  assert.equal(projection.project(older).some(row => row.id === 'old'), false);
  assert.equal(projection.files(older).files[0].messageId, 'old');
});

test('shared-reference arrays and output rows remain bounded without claiming a complete truncated index', () => {
  const many = Array.from({ length: 21 }, (_, i) => reference('libfile_' + i));
  const payload = { messages: [message('many', many)] };
  assert.equal(projection.files(payload).files.length, 20);
  assert.equal(projection.files(payload).truncated, true);
  assert.equal(projection.project(payload)[0].content.length, 19);
  assert.equal(projection.fileSource(payload, 'many:20'), null);
  const full = { messages: Array.from({ length: 6 }, (_, i) => message('row-' + i, many)) };
  assert.equal(projection.files(full).files.length, 100);
  assert.equal(projection.files(full).truncated, true);
});

test('malformed shared metadata is safe to project and never gains an ordinary file fallback', () => {
  for (const refs of [null, {}, 'invalid', [null, {}, { name: 17 }]]) {
    assert.deepEqual(projection.files({ messages: [message('bad', refs)] }), { files: [], truncated: false });
  }
  for (const extra of [{ library_file_id: '../escape' }, { id: 'file-other' },
    { mounted_library_file_id: 'mounted' }, { source_url: 'https://external.test' },
    { context_scopes: ['other'] }, { preview_file: {} }, { mime_type: {} },
    { size_bytes: -1 }, { name: 'bad\nname.txt' }]) {
    assert.equal(library.sharedReference({ ...reference(), ...extra }), null);
  }
  assert.deepEqual(library.sharedReference(reference()), { sharedLibraryFileId: 'libfile_synthetic' });
  assert.equal(library.target(reference()), null, 'metadata and ordinary attachments remain distinct descriptors');
});

test('module reinjection updates file projection without replacing the proven identity transport', () => {
  let retired = 0;
  const transport = { identity: 'synthetic' };
  const window = { location: { origin: 'https://chatgpt.com' },
    __elonChatGptPrivateTransport: transport,
    __elonChatGptPrivateFileDownload: { version: download.version - 1, dispose() { retired++; } } };
  const context = vm.createContext({ window, Map, Set, URL, URLSearchParams });
  for (let i = 0; i < 2; i++) {
    for (const name of ['history_projection', 'library_download', 'file_download']) {
      vm.runInContext(fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_private_' + name + '.js'), 'utf8'), context);
    }
  }
  assert.equal(retired, 1);
  assert.equal(window.__elonChatGptPrivateTransport, transport);
  assert.equal(window.__elonChatGptPrivateHistoryProjection.create({}).files({ messages: [message()] }).files.length, 1);
  assert.equal(window.__elonChatGptPrivateLibraryDownload.version, library.version);
  assert.equal(window.__elonChatGptPrivateFileDownload.version, download.version);
});
