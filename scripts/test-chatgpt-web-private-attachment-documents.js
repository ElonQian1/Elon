'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');
const assets = '../android/app/src/main/assets/';
const protocol = require(assets + 'chatgpt_web_private_attachment_protocol.js');
const sourceModule = require(assets + 'chatgpt_web_native_attachment_source.js');
const composerModule = require(assets + 'chatgpt_web_private_attachment_composer.js');
const transportModule = require(assets + 'chatgpt_web_private_attachment_transport.js');
const sendModule = require(assets + 'chatgpt_web_private_attachment_send.js');
const jsonRequest = require(assets + 'chatgpt_web_private_json_request.js');
const cases = require('../android/app/src/test/resources/chatgpt_private_attachment_documents.json');
const CONVERSATION = '00000000-0000-4000-8000-000000000001';

function fixture(spec, { temporary = false, existing = false, incomplete = false, strategy = false } = {}) {
  const bytes = Buffer.alloc(140000, 47), chunks = [], requests = [], receipts = [];
  bytes.set([0, 255, 128, 13, 10], 65534);
  let files = [], fallbacks = 0;
  const files$ = () => files;
  files$.set = value => { files = value; };
  const store = { files$, readyFiles$: () => files.filter(file => file.status === 'ready'),
    hasUploadInProgress$: () => files.some(file => file.status === 'uploading') };
  const top = { stateNode: {} };
  top.stateNode.current = top;
  const input = { isConnected: true, __reactFiber$synthetic: { memoizedProps: { value: store },
    return: { memoizedProps: { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'synthetic-model' }, return: top } } };
  const headers = () => ({ authorization: 'Bearer synthetic-page-token', 'chatgpt-account-id': 'synthetic-account' });
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' +
      (existing ? 'c/' + CONVERSATION : '') + (temporary ? '?temporary-chat=true' : '') },
    __elonChatGptDocumentToken: 'doc_synthetic_documents',
    __elonChatGptComposer: { currentModel: () => 'synthetic-model' },
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptPrivateAttachmentTransport: transportModule,
    __elonChatGptPrivateJsonRequest: jsonRequest,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: headers, acquireSameOriginRequestHeaders: async () => headers(),
      readAttachmentContext: async () => ({ conversationId: CONVERSATION, ordinary: !temporary, temporary }) },
    document: { querySelector: selector => selector === '#upload-files' ? input : {} },
    AbortController, File, atob, setTimeout, clearTimeout, setInterval, clearInterval,
    fetch: async (url, init) => {
      requests.push({ url, init });
      if (url === '/backend-api/files') return Response.json({ status: 'success', file_id: 'file-synthetic',
        upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic',
        ...(strategy ? { direct_library_upload_strategy: { unknown: true } } : {}) });
      if (init.method === 'PUT') return new Response('');
      if (url === '/backend-api/files/process_upload_stream') return new Response(JSON.stringify({
        file_id: 'file-synthetic', event: incomplete ? 'file.processing.file_ready' : 'file.processing.completed',
        progress: 100, extra: { total_tokens: 8, mime_type: spec.type },
      }));
      assert.fail('unexpected private request: ' + url);
    } };
  const bridge = { onmessage: null, postMessage: raw => {
    const request = JSON.parse(raw);
    chunks.push(request);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ requestId: request.requestId,
      offset: request.offset, data: bytes.subarray(request.offset, request.offset + 65536).toString('base64') }) }));
  } };
  root.elonChatGptAttachmentSource = bridge;
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href, size: bytes.length, ...spec };
  const composer = composerModule.create(root);
  const send = sendModule.create(root, { composer, source: sourceModule.create(root) });
  return { root, bytes, chunks, requests, receipts, bridge, store, composer,
    start: () => send.start(JSON.stringify(descriptor), (...args) => receipts.push(args), () => {}, () => { fallbacks++; }),
    fallbacks: () => fallbacks };
}

test('native policy and page byte-source contract admit the same explicit document MIME types', () => {
  const native = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebNativeAttachmentPolicy.kt'), 'utf8');
  const declaration = /private val documentTypes = setOf\(([\s\S]*?)\n    \)/.exec(native);
  assert.ok(declaration, 'native production MIME policy must enable the shared document contract');
  const types = [...declaration[1].matchAll(/"([^"\n]+)"/g)].map(match => match[1]);
  assert.deepEqual(types.sort(), [...protocol.documentMimeTypes].sort());
  assert.deepEqual(types.sort(), cases.documents.map(file => file.type).sort());
  assert.match(native, /if \(mimeType in documentTypes\) return true/);
});

test('new/existing ordinary/temporary documents use real byte-source, upload and ready-store modules', async () => {
  for (const spec of cases.documents) {
    for (const temporary of [false, true]) {
      for (const existing of [false, true]) {
        const f = fixture(spec, { temporary, existing });
        await f.start();
        assert.deepEqual(f.receipts, [['request_attachment_upload', true, 'private_attachment_associated']], spec.type);
        assert.equal(f.fallbacks(), 0);
        assert.deepEqual(f.chunks.map(item => item.offset), [0, 65536, 131072]);
        assert.deepEqual(f.requests.map(item => item.init.method), ['POST', 'PUT', 'POST']);
        const [create, put, process] = f.requests, body = JSON.parse(process.init.body);
        assert.equal(JSON.parse(create.init.body).mime_type, spec.type);
        assert.equal(JSON.parse(create.init.body).use_case, 'ace_upload');
        assert.equal(create.init.headers['x-oai-model-slug'], spec.type === 'application/pdf' ? 'synthetic-model' : undefined);
        assert.equal(put.init.credentials, 'omit');
        assert.equal(put.init.headers['Content-Type'], spec.type);
        assert.equal(put.init.headers.authorization, undefined);
        assert.equal(put.init.headers['x-oai-model-slug'], undefined);
        assert.deepEqual(Buffer.from(await put.init.body.arrayBuffer()), f.bytes);
        assert.equal(body.index_for_retrieval, false);
        assert.equal(body.metadata.is_temporary_chat, temporary);
        assert.equal(body.metadata.is_project_thread, false);
        assert.equal(body.metadata.store_in_library, false);
        assert.equal(body.library_persistence_mode, temporary ? undefined : 'required');
        assert.equal(f.store.readyFiles$()[0].fileSpec.mimeType, spec.type);
        assert.equal(f.store.readyFiles$()[0].fileSpec.fileTokenSize, 8);
        assert.equal(f.composer.remove(f.composer.merge([])[0].id), true);
        assert.equal(f.store.readyFiles$().length, 0);
        assert.equal(f.requests.length, 3, 'local removal neither sends text nor deletes a remote file');
        assert.equal(f.bridge.onmessage, null);
      }
    }
  }
});

test('partial processing and unknown upload strategy never claim ready or replay a document write', async () => {
  for (const options of [{ incomplete: true }, { strategy: true }]) {
    const f = fixture(cases.documents.find(file => file.name.endsWith('.docx')), options);
    await f.start();
    assert.equal(f.receipts[0][1], false);
    assert.equal(f.fallbacks(), 0);
    assert.equal(f.store.readyFiles$().length, 0);
    assert.equal(f.requests.length, options.strategy ? 1 : 3);
    assert.equal(f.bridge.onmessage, null);
  }
});
