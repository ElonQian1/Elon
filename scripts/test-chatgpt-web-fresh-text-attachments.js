'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { fixture, pointer, CID, PID } = require('./fixtures/chatgpt-fresh-text-attachments');
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request');
const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();
const command = { requestId: 'mcp_attachment1', prompt: 'Synthetic question' };
function consume(f, binding, prompt = command.prompt) {
  f.page.crypto = crypto;
  const request = requests.create(f.page).create(binding, { ...command, prompt });
  const result = request.consume({ conduit_token: 'synthetic' }, { chatReq: { token: 'synthetic' } },
    () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' }), binding.current);
  return { request, ...result };
}

test('owned TXT/PDF/image selections use provider metadata and multimodal content in a fresh request', async () => {
  const f = await fixture(), binding = await f.capture(), sent = consume(f, binding);
  const prepared = sent.request.preparationBody(), message = sent.body.messages[0];
  assert.deepEqual(prepared.attachment_mime_types, ['text/plain', 'application/pdf', 'image/png']);
  assert.equal('attachment_mime_types' in sent.body, false, 'MIME preflight is not a copied dispatch field');
  assert.equal(sent.body.conversation_id, CID); assert.equal(sent.body.parent_message_id, PID);
  assert.equal(sent.body.enable_message_followups, true);
  assert.equal(message.content.content_type, 'multimodal_text');
  assert.equal(message.content.parts[0].asset_pointer, 'sediment://file_synthetic_2');
  assert.equal(message.content.parts.at(-1), command.prompt);
  assert.deepEqual(message.metadata.attachments.map(item => item.id), f.selectedFiles.map(file => file.fileId));
  assert.equal(message.metadata.attachments[0].file_token_size, 5);
  assert.equal(f.files.files$().length, 3, 'serialization alone cannot consume attachments');
});

test('document-only and file-only sends keep text content and deduplicated MIME preparation', async () => {
  const f = await fixture(['text/plain', 'text/plain']), binding = await f.capture(), sent = consume(f, binding, '');
  assert.deepEqual(sent.request.preparationBody().attachment_mime_types, ['text/plain']);
  assert.deepEqual(sent.body.messages[0].content, { content_type: 'text', parts: [''] });
  assert.equal(sent.body.messages[0].metadata.attachments.length, 2);
});

test('ordinary library identity is preserved, not converted to a new local upload', async () => {
  const f = await fixture(['text/plain']), file = f.selectedFiles[0];
  file.source = 'library'; file.libraryFileId = 'synthetic-library-file';
  file.fileSpec.libraryPersistenceResult = 'stored';
  const binding = await f.capture(), sent = consume(f, binding), metadata = sent.body.messages[0].metadata.attachments[0];
  assert.equal(metadata.id, file.fileId); assert.equal(metadata.source, 'library');
  assert.equal(metadata.library_file_id, file.libraryFileId);
  assert.equal(metadata.library_persistence_result, 'stored');
  assert.equal(f.files.files$().length, 1);
});

test('incorrect image dimensions or substituted document metadata cannot produce a dispatch', async () => {
  for (const corrupt of [value => { value.content.parts[0].width++; },
    value => { value.content.parts[0].asset_pointer = pointer('file-other'); },
    value => { value.attachments[0].id = 'file-other'; },
    value => { value.attachments[0].library_file_id = 'foreign-library'; }]) {
    const f = await fixture();
    f.conversation.textSerializeAttachments = (files, prompt) => {
      const value = f.serialize(files, prompt); corrupt(value); return value;
    };
    await assert.rejects(f.capture(), /attachments_active/);
    assert.equal(f.files.files$().length, 3);
  }
});

test('attachment admission reuses the private owner and fails before dispatch on absent or lossy serialization', async () => {
  for (const change of [f => { f.page.__elonChatGptPrivateAttachmentSend = null; },
    f => { f.conversation.textSerializeAttachments = null; },
    f => { f.conversation.textSerializeAttachments = (_, prompt) => ({ content: prompt, attachments: [] }); },
    f => { f.selectedFiles[0].fileSpec.contextConnectorInfo = {}; },
    f => { f.selectedFiles[0].mountedLibraryFileId = 'unreviewed'; },
    f => { f.selectedFiles[0].sharedLibraryFileReference = {}; },
    f => { f.selectedFiles[0].status = 'uploading'; },
  ]) {
    const f = await fixture(); change(f); await assert.rejects(f.capture(), /attachments_active/);
    assert.equal(f.files.files$().length, 3);
  }
  const f = await fixture(); await assert.rejects(f.api.capture(f.node), /attachments_active/);
});

test('changed metadata, selection, account or model invalidates a prepared attachment owner', async () => {
  for (const change of [f => { f.selectedFiles[0].fileSpec.name = 'changed.txt'; },
    f => { f.files.files$.set(f.files.files$().slice(1)); },
    f => { f.identity('other-account'); }, f => { f.attachmentFixture.setModel('other-model'); },
    f => { f.conversation.textSerializeAttachments = (_, prompt) => ({ content: prompt, attachments: [] }); },
  ]) {
    const f = await fixture(), binding = await f.capture(); change(f);
    assert.throws(() => consume(f, binding));
    assert.ok(f.files.files$().length > 0);
  }
});

test('private lease cleanup preserves later files and is idempotent after acceptance', async () => {
  const f = await fixture(), binding = await f.capture(); consume(f, binding);
  const later = { tempId: 'later', status: 'uploading' };
  f.files.files$.set([...f.files.files$(), later]);
  assert.equal(binding.attachments.consumeAccepted(), true);
  assert.deepEqual(f.files.files$(), [later]);
  assert.equal(binding.attachments.consumeAccepted(), true);
  assert.deepEqual(f.files.files$(), [later]);
});

test('terminal history must contain all submitted file references, not just the same user ID', async () => {
  const f = await fixture(), binding = await f.capture(), sent = consume(f, binding);
  const user = sent.body.messages[0], aid = crypto.randomUUID();
  const payload = { conversation_id: CID, current_node: aid, mapping: {
    [user.id]: { id: user.id, parent: PID, message: user },
    [aid]: { id: aid, parent: user.id, message: { id: aid, author: { role: 'assistant' },
      status: 'finished_successfully', end_turn: true } }
  } };
  assert.equal(history.ownsResponse(payload, binding, user.id), true);
  const missing = structuredClone(payload); missing.mapping[user.id].message.metadata.attachments.shift();
  assert.equal(history.ownsResponse(missing, binding, user.id), false);
  const imageOnly = structuredClone(payload);
  imageOnly.mapping[user.id].message.metadata.attachments.pop();
  assert.equal(history.ownsResponse(imageOnly, binding, user.id), true);
  imageOnly.mapping[user.id].message.content.parts[0].asset_pointer = pointer('file-other');
  assert.equal(history.ownsResponse(imageOnly, binding, user.id), false);
});

test('serializer may represent a model-compatible image without redundant metadata but may not drop a document', async () => {
  const f = await fixture(['image/png']);
  f.conversation.textSerializeAttachments = (files, prompt) => ({ ...f.serialize(files, prompt), attachments: [] });
  const binding = await f.capture(), sent = consume(f, binding);
  assert.equal(sent.body.messages[0].content.parts[0].asset_pointer, 'file-service://file-synthetic-0');
  assert.deepEqual(sent.body.messages[0].metadata, {});
});
