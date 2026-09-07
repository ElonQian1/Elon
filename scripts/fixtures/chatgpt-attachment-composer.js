'use strict';
const composerModule = require('../../android/app/src/main/assets/chatgpt_web_private_attachment_composer.js');
const transportModule = require('../../android/app/src/main/assets/chatgpt_web_private_attachment_transport.js');
const protocol = require('../../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');

module.exports = function fixture() {
  let values = [], account = 'Bearer synthetic-page-token', model = 'synthetic-model';
  const files$ = () => values;
  files$.set = next => { values = next; };
  const store = { files$, readyFiles$: () => values.filter(item => item.status === 'ready'),
    hasUploadInProgress$: () => values.some(item => item.status === 'uploading') };
  const fiber = { memoizedProps: { value: store }, dependencies: { firstContext: { memoizedValue: store } } };
  const input = { isConnected: true, __reactFiber$synthetic: fiber };
  const headers = () => ({ Authorization: account, 'chatgpt-account-id': 'synthetic-workspace' });
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    document: { querySelector: name => name === '#upload-files' ? input : {} },
    __elonChatGptDocumentToken: 'doc_synthetic_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: headers, acquireSameOriginRequestHeaders: async () => headers() },
    __elonChatGptPrivateAttachmentTransport: transportModule,
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptComposer: { currentModel: () => model },
    AbortController, setTimeout, clearTimeout, setInterval, clearInterval,
  };
  const composer = composerModule.create(root);
  const file = new File(['synthetic bytes'], 'fixture.txt', { type: 'text/plain' });
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    name: file.name, size: file.size, type: file.type };
  const result = binding => ({ ok: true, stage: 'processed', associated: false, binding,
    fileId: 'file-synthetic', fileName: file.name, fileSize: file.size, mimeType: file.type,
    isTemporaryChat: binding.isTemporaryChat, metadata: { fileTokenSize: 5 } });
  return { root, composer, store, input, fiber, file, descriptor, result,
    setAccount: next => { account = next; }, setModel: next => { model = next; } };
};
