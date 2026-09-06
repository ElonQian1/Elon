'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const assets = '../android/app/src/main/assets/';
const protocol = require(assets + 'chatgpt_web_private_attachment_protocol.js');
const transportModule = require(assets + 'chatgpt_web_private_attachment_transport.js');
const jsonRequest = require(assets + 'chatgpt_web_private_json_request.js');
const PROJECT = 'g-p-0123456789abcdef0123456789abcdef';
const OTHER = 'g-p-fedcba9876543210fedcba9876543210';
const CONVERSATION = '01234567-89ab-4cde-8123-456789abcdef';
const LEAF = '11111111-2222-4333-8444-555555555555';
const ORIGIN = { origination_thread_id: CONVERSATION, origination_message_id: LEAF };

function context(existing = false) {
  return { useCase: 'ace_upload', projectScopeId: PROJECT, isProjectThread: true,
    isTemporaryChat: false, storeInLibrary: false, libraryPersistenceMode: 'required',
    indexForRetrieval: false, ...(existing ? { libraryFileInfo: { ...ORIGIN } } : {}) };
}
const textFile = () => new File(['synthetic read-only project attachment'], 'fixture.txt', { type: 'text/plain' });

test('chat-only project request separates local scope from server project-write metadata', () => {
  const file = textFile();
  for (const existing of [false, true]) {
    const selected = context(existing), create = protocol.prepare(file, selected);
    const process = protocol.processBody('file-synthetic', file, selected);
    assert.equal(create.use_case, 'ace_upload');
    assert.equal(create.gizmo_id, undefined);
    assert.equal(process.gizmo_id, undefined);
    assert.equal(process.metadata.is_project_thread, true);
    assert.equal(process.metadata.store_in_library, false);
    assert.deepEqual(process.metadata.library_file_info, existing ? ORIGIN : undefined);
    assert.equal(JSON.stringify([create, process]).includes(PROJECT), false);
    assert.equal(JSON.stringify([create, process]).includes('projectScopeId'), false);
  }
});

test('chat-only images and PDFs keep their existing model/dimension protocol', () => {
  for (const existing of [false, true]) {
    const image = new File(['synthetic pixels'], 'fixture.png', { type: 'image/png' });
    const selected = { ...context(existing), useCase: 'multimodal', imageDimensions: { width: 12, height: 8 },
      indexForRetrieval: true };
    assert.equal(protocol.prepare(image, selected).use_case, 'multimodal');
    const process = protocol.processBody('file-synthetic', image, selected);
    assert.equal(process.index_for_retrieval, true);
    assert.deepEqual(process.metadata.library_file_info, existing ? ORIGIN : undefined);
    const pdf = new File(['%PDF-1.7'], 'fixture.pdf', { type: 'application/pdf' });
    assert.deepEqual(protocol.creationHeaders(pdf, { ...context(existing), modelSlug: 'synthetic-model' }),
      { 'x-oai-model-slug': 'synthetic-model' });
  }
});

test('unknown project binding, partial origins or mixed project-write metadata fail before upload', () => {
  for (const override of [{ projectScopeId: undefined }, { projectScopeId: 'unconfirmed' },
    { isProjectThread: false }, { isTemporaryChat: true }, { gizmoId: PROJECT }, { useCase: 'gizmo' },
    { libraryFileInfo: { ...ORIGIN, should_upload_to_project: true } },
    { libraryFileInfo: { ...ORIGIN, is_project: false } }, { libraryFileInfo: { origination_thread_id: CONVERSATION } },
    { libraryFileInfo: { ...ORIGIN, origination_message_id: 'unconfirmed' } },
    { libraryFileInfo: { ...ORIGIN, gizmo_id: OTHER, is_project: true, should_upload_to_project: true } }]) {
    assert.throws(() => protocol.prepare(textFile(), { ...context(true), ...override }), /unsupported_upload_context/);
  }
});

function transaction(options = {}) {
  const calls = [];
  const binding = {};
  const headers = { authorization: 'Bearer synthetic-page-token' };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + CONVERSATION },
    __elonChatGptPrivateAttachmentProtocol: protocol, __elonChatGptPrivateJsonRequest: jsonRequest,
    AbortController, setTimeout, clearTimeout, fetch: async (url, init) => {
      calls.push({ url, init });
      if (url === '/backend-api/files') return Response.json({ status: 'success', file_id: 'file-synthetic',
        upload_url: 'https://uploads.oaiusercontent.com/synthetic?sig=fixture' });
      if (init.method === 'PUT') return new Response('');
      if (url.endsWith('/process_upload_stream')) {
        if (options.failProcessing) throw new Error('synthetic process connection loss');
        return new Response(JSON.stringify({ event: 'file.processing.completed', file_id: 'file-synthetic', progress: 100 }));
      }
      throw new Error('unexpected_request');
    } };
  const transport = transportModule.create(root, { isCurrent: value => value === binding,
    acquireHeaders: async () => { options.beforeAuth?.(); return headers; } });
  return { calls, binding, transport };
}

test('private transaction retains local project ownership without requesting any project file write', async () => {
  const selected = context(true);
  const f = transaction({ beforeAuth: () => {
    selected.projectScopeId = OTHER;
    selected.libraryFileInfo.origination_message_id = CONVERSATION;
  } });
  const result = await f.transport.upload(textFile(), selected, f.binding);
  assert.equal(result.ok, true);
  assert.equal(result.associated, false);
  assert.equal(result.projectId, PROJECT);
  assert.equal(result.projectWriteRequested, false);
  assert.equal(f.calls.length, 3);
  assert.deepEqual(JSON.parse(f.calls[2].init.body).metadata.library_file_info, ORIGIN);
  assert.equal(f.calls[1].init.credentials, 'omit');
  assert.equal(f.calls[1].init.headers.authorization, undefined);
});

test('ambiguous processing never repeats the upload or converts a chat-only request to project scope', async () => {
  const f = transaction({ failProcessing: true });
  const result = await f.transport.upload(textFile(), context(true), f.binding);
  assert.equal(result.ok, false);
  assert.equal(result.mayHaveSideEffects, true);
  assert.equal(f.calls.length, 3);
  assert.equal(f.calls.filter(call => call.url === '/backend-api/files').length, 1);
  assert.equal(f.calls.some(call => call.init.method === 'DELETE'), false);
  assert.equal(JSON.stringify(f.calls.filter(call => call.init.body && typeof call.init.body === 'string')
    .map(call => JSON.parse(call.init.body))).includes('should_upload_to_project'), false);
});
