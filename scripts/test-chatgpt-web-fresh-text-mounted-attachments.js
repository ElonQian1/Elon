'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { fixture: baseFixture, appendMounted, pointer, CID, PID } = require('./fixtures/chatgpt-fresh-text-attachments');
const fixture = types => baseFixture(types, { library: true });
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request');
const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();

function consume(f, binding) {
  f.page.crypto = crypto;
  return requests.create(f.page).create(binding, { requestId: 'mcp_mountsend', prompt: 'Synthetic file question' })
    .consume({ conduit_token: 'synthetic' }, { chatReq: { token: 'synthetic' } },
      () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' }), binding.current).body;
}

test('actual mounted selection and private lease enter a fresh request without reupload or rematerialization', async () => {
  for (const [id, provider] of [['external-gdrive:file:synthetic123', 'google_drive'],
    ['external-box:file:1234567', 'box'], ['external-dropbox:file:id:synthetic123', 'dropbox'],
    ['external-sharepoint:file:v1:item:' + btoa('drive-id').replace(/=+$/, '') + ':' +
      btoa('item-id').replace(/=+$/, ''), 'sharepoint']]) {
    const f = await fixture(['text/plain']), mounted = await appendMounted(f, { id });
    const binding = await f.capture(), body = consume(f, binding), item = body.messages[0].metadata.attachments[1];
    assert.equal(item.id, mounted.result.file_id);
    assert.equal(item.mounted_library_file_id, id);
    assert.equal(item.mounted_library_mime_type, 'text/plain');
    assert.equal(item.library_provider, provider);
    assert.equal(item.library_entrypoint, 'composer_library_picker');
    assert.equal(item.source, 'library'); assert.equal('library_file_id' in item, false);
    assert.equal(mounted.calls.length, 1);
    assert.equal(JSON.parse(mounted.calls[0].init.body).index_for_retrieval, true);
    assert.equal(f.files.files$().length, 2);
    assert.equal(body.conversation_id, CID); assert.equal(body.parent_message_id, PID);
  }
});

test('materialized Drive export keeps its original MIME and unknown size instead of treating it as a local empty upload', async () => {
  const f = await fixture(['text/plain']);
  const m = await appendMounted(f, { mime_type: 'application/vnd.google-apps.document', file_size_bytes: null },
    { file_name: 'mounted.docx', mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
  const binding = await f.capture(), body = consume(f, binding), item = body.messages[0].metadata.attachments[1];
  assert.equal(item.size, 0); assert.equal(item.mime_type, m.result.mime_type);
  assert.equal(item.mounted_library_mime_type, m.source.mime_type);
  assert.equal(m.file.file.size, 0);
  assert.equal(m.calls.length, 1);
  const local = await fixture(['text/plain']); local.selectedFiles[0].fileSpec.size = 0;
  await assert.rejects(local.capture(), /attachments_active/);
});

test('prepared mounted image retains backing pointer and preview without discarding provenance', async () => {
  const f = await fixture(['text/plain']);
  const m = await appendMounted(f, { name: 'mounted.png', mime_type: 'image/png' },
    { preview_file: { file_id: 'file-preview', mime_type: 'image/png' } });
  const binding = await f.capture(), body = consume(f, binding), message = body.messages[0];
  assert.deepEqual(message.metadata.attachments[1].preview_file, m.result.preview_file);
  assert.equal(message.content.parts[0].asset_pointer, pointer(m.result.file_id));
  assert.equal(message.content.parts[0].width, m.file.fileSpec.width);
  f.conversation.textSerializeAttachments = (files, prompt) => ({ ...f.serialize(files, prompt), attachments: [] });
  await assert.rejects(f.capture(), /attachments_active/);
});

test('mounted metadata cannot be substituted or lost by the provider formatter', async () => {
  for (const field of ['mounted_library_file_id', 'mounted_library_mime_type', 'library_provider',
    'library_entrypoint', 'preview_file']) {
    const f = await fixture(['text/plain']);
    await appendMounted(f, {}, { preview_file: { file_id: 'file-preview' } });
    f.conversation.textSerializeAttachments = (files, prompt) => {
      const result = f.serialize(files, prompt); delete result.attachments[1][field]; return result;
    };
    await assert.rejects(f.capture(), /attachments_active/);
    assert.equal(f.files.files$().length, 2);
  }
});

test('unmaterialized, wrong-provider and unowned selections remain outside the fresh path', async () => {
  for (const change of [m => { m.file.fileId = m.file.fileSpec.id = m.source.id; },
    m => { m.file.libraryProvider = 'wrong'; }, m => { m.file.source = 'local'; },
    m => { m.file.libraryFileId = 'libfile-other'; }, m => { m.file.mountedLibraryMimeType = ''; },
    m => { m.file.requiresMountedLibraryMaterialization = true; }]) {
    const f = await fixture(['text/plain']), m = await appendMounted(f); change(m);
    await assert.rejects(f.capture(), /attachments_active/);
    assert.equal(m.calls.length, 1);
  }
  const f = await fixture(['text/plain']); await appendMounted(f);
  f.page.__elonChatGptPrivateAttachmentSend = null;
  await assert.rejects(f.capture(), /attachments_active/);
});

test('prepared ownership invalidates on mounted identity changes and later attachments survive cleanup', async () => {
  const f = await fixture(['text/plain']), m = await appendMounted(f), binding = await f.capture();
  m.file.mountedLibraryFileId = 'external-gdrive:file:other123';
  assert.throws(() => consume(f, binding));
  assert.equal(binding.attachments.consumeAccepted(), false);
  m.file.mountedLibraryFileId = m.source.id;
  const later = { status: 'uploading', tempId: 'later' }; f.files.files$.set([...f.files.files$(), later]);
  assert.equal(binding.attachments.consumeAccepted(), true);
  assert.deepEqual(f.files.files$(), [later]);
  assert.equal(binding.attachments.consumeAccepted(), true); assert.equal(m.calls.length, 1);
});

test('history needs both materialized backing and mounted provenance, not an unrelated copy or image pointer', async () => {
  const f = await fixture(['text/plain']); await appendMounted(f, { name: 'mounted.png', mime_type: 'image/png' });
  const binding = await f.capture(), body = consume(f, binding), user = body.messages[0], aid = crypto.randomUUID();
  const payload = { conversation_id: CID, current_node: aid, mapping: {
    [user.id]: { id: user.id, parent: PID, message: user },
    [aid]: { id: aid, parent: user.id, message: { id: aid, author: { role: 'assistant' },
      status: 'finished_successfully', end_turn: true } }
  } };
  assert.equal(history.ownsResponse(payload, binding, user.id), true);
  for (const field of ['id', 'mounted_library_file_id', 'mounted_library_mime_type']) {
    const wrong = structuredClone(payload); delete wrong.mapping[user.id].message.metadata.attachments[1][field];
    assert.equal(history.ownsResponse(wrong, binding, user.id), false);
  }
  const onlyImage = structuredClone(payload); onlyImage.mapping[user.id].message.metadata.attachments.pop();
  assert.equal(history.ownsResponse(onlyImage, binding, user.id), false);
});
