'use strict';
const assert = require('node:assert/strict');
const { fixture: contextFixture, CID, PID } = require('./chatgpt-fresh-text-context');
const attachmentFixture = require('./chatgpt-attachment-composer');
const attachments = require('../../android/app/src/main/assets/chatgpt_web_fresh_text_attachments');
const pointer = id => (id.startsWith('file_') ? 'sediment://' : 'file-service://') + id;

// Synthetic provider output; exact production serializer contracts are checked
// against pinned public source, never by executing a downloaded bundle here.
function serialize(files, prompt) {
  const metadata = files.map(file => {
    const s = file.fileSpec;
    return { id: s.id, size: s.size, name: s.name, mime_type: s.mimeType, source: file.source,
      file_token_size: s.fileTokenSize, library_file_id: file.libraryFileId,
      library_persistence_result: s.libraryPersistenceResult, is_big_paste: s.isBigPaste ?? false };
  });
  const images = files.filter(file => 'width' in file.fileSpec).map(file => ({ content_type: 'image_asset_pointer',
    asset_pointer: pointer(file.fileId), size_bytes: file.fileSpec.size,
    width: file.fileSpec.width, height: file.fileSpec.height }));
  return { content: images.length ? { content_type: 'multimodal_text', parts: [...images, prompt] } : prompt,
    attachments: metadata };
}

async function fixture(types = ['text/plain', 'application/pdf', 'image/png']) {
  const f = contextFixture(), a = attachmentFixture();
  a.root.location = f.page.location;
  a.root.__elonChatGptDocumentToken = f.page.__elonChatGptDocumentToken;
  f.page.document.querySelector = a.root.document.querySelector;
  a.root.document = f.page.document;
  a.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: CID, ordinary: true });
  a.setModel(f.conversation.Nrn(f.selected).id);
  f.files = f.binding.files = a.store;
  f.page.__elonChatGptPrivateAttachmentSend = { prepareSubmit: store => a.composer.prepareSubmit(store) };
  f.page.__elonChatGptFreshTextAttachments = attachments;
  const binding = a.composer.capture();
  assert.equal(await a.composer.prepare(binding, new AbortController().signal), true);
  const completed = types.map((type, index) => {
    const file = new File(['synthetic bytes'], 'synthetic-' + index + '.dat', { type });
    const result = { ...a.result(binding), fileId: index === 2 ? 'file_synthetic_2' : 'file-synthetic-' + index,
      fileName: file.name, fileSize: file.size, mimeType: type,
      ...(type.startsWith('image/') ? { imageDimensions: { width: 64, height: 48 } } : {}) };
    return { file, result, leaseId: '00000000-0000-4000-8000-' + String(index).padStart(12, '0') };
  });
  a.composer.associateMany(binding, completed);
  f.conversation.textSerializeAttachments = serialize;
  return Object.assign(f, { attachmentFixture: a, completed, serialize, selectedFiles: [...a.store.files$()],
    capture: () => f.api.capture(f.node, null, { allowAttachments: true }) });
}
module.exports = { fixture, serialize, pointer, CID, PID };
