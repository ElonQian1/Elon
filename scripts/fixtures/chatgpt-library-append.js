'use strict';
const fixture = require('./chatgpt-attachment-composer');
const assets = '../../android/app/src/main/assets/';

module.exports = function setup() {
  const f = fixture(), sources = new Map();
  Object.assign(f.fiber.memoizedProps, { conversation: {}, onCreateNewCompletion() {},
    currentModelId: 'synthetic-model', entrySurface: 'chat_composer', isLibraryEnabled: true });
  const props = { conversation: f.fiber.memoizedProps.conversation, composerDisabled: false,
    isTemporaryChat: false, isProjectThread: false, currentModelId: 'synthetic-model',
    maxLibraryAttachmentCount: 9, maxTotalLibraryAttachmentCount: 9 };
  f.fiber.type = { name: 'Evn' };
  const plus = { isConnected: true, __reactFiber$test: f.fiber };
  const querySelector = f.root.document.querySelector;
  f.root.document.querySelector = selector => selector === '#composer-plus-btn' ? plus : querySelector(selector);
  // One committed owner supplies the observed upload store and current menu props.
  Object.assign(f.fiber.memoizedProps, props);
  let validation = true, loads = 0, cached = null, fresh = true, requests = 0;
  const validations = [], responses = [], changes = [];
  const namespace = { fh: { validateChatAttachment(...args) {
    validations.push(args);
    return validation && args[5].length < args[4];
  } } };
  Object.assign(f.root, { File, crypto: require('node:crypto').webcrypto,
    __elonChatGptPrivateLibraryAttachment: require(assets + 'chatgpt_web_private_library_attachment'),
    __elonChatGptPrivateLibraryAttachmentPolicy: require(assets + 'chatgpt_web_private_library_attachment_policy'),
    __elonChatGptPrivateMountedLibraryAttachment: require(assets + 'chatgpt_web_private_mounted_library_attachment'),
    __elonChatGptPrivateLibraryDownload: require(assets + 'chatgpt_web_private_library_download'),
    __elonChatGptPrivateJsonRequest: require(assets + 'chatgpt_web_private_json_request'),
    __elonChatGptPrivateRuntimeBindings: { tools: () => ({ owner: 'Evn' }), peek: () => cached,
      load: async () => { loads++; cached = namespace; return cached; } },
    __elonChatGptPrivateLibraryCatalog: { selectAttachment: handle => sources.has(handle)
      ? { source: sources.get(handle), current: () => fresh } : null },
    fetch: async () => { requests++; return Response.json({ file_id: 'file-materialized', file_name: 'cloud.txt',
      mime_type: 'text/plain', file_size_bytes: 60 }); },
  });
  const sender = require(assets + 'chatgpt_web_private_attachment_send').create(f.root, { composer: f.composer });
  function source(index, patch = {}) {
    const handle = 'library_' + index.toString(16).padStart(32, '0');
    sources.set(handle, { kind: 'file', id: 'libfile_' + index, file_id: 'file-' + index,
      name: 'fixture-' + index + '.txt', mime_type: 'text/plain', file_size_bytes: 45, ...patch });
    return handle;
  }
  async function attach(handle, id) {
    await sender.attachLibrary({ selected: true, requestId: id || 'mcp_append' + responses.length,
      value: JSON.stringify({ fileHandle: handle }) }, (...row) => responses.push(row), row => changes.push(row));
    return responses.at(-1);
  }
  return { ...f, sender, source, attach, sources, responses, changes, validations, plus, namespace,
    props: f.fiber.memoizedProps, loads: () => loads, requests: () => requests,
    expire: () => { fresh = false; }, reject: () => { validation = false; } };
};
