'use strict';
const assert = require('node:assert/strict');
const base = '../../android/app/src/main/assets/';
const parser = require(base + 'chatgpt_web_text_blocks');
const policy = require(base + 'chatgpt_web_writing_block_policy');
const context = require(base + 'chatgpt_web_writing_block_context');
const library = require(base + 'chatgpt_web_writing_library_session');
const transport = require(base + 'chatgpt_web_private_writing_blocks');
const cid = '11111111-1111-4111-8111-111111111111', mid = 'message-a', bid = 'block-a', lid = 'libfile_a', path = '/c/' + cid;
function harness() {
  const message = { id: mid, author: { role: 'assistant' }, status: 'finished_successfully',
    content: { content_type: 'text', parts: [':::writing{id="block-a" variant="document"}\nOriginal\n:::'] },
    metadata: { writing_blocks: { [bid]: { content: 'Original\n', library_file_id: lid, metadata: { tag: 'keep' } },
      unrelated: { content: 'Keep other block' } } } };
  const state = { current: true, message, server: structuredClone(message), posts: [], invalidated: [], retains: 0,
    dispatched: false, effects: true, timerCallbacks: [], holdQueue: false, queued: null, postError: null, updates: 0,
    session: { libraryFileId: lid, draftContent: 'Original\n', lastSavedContent: 'Original\n', baseVersionNumber: 4,
      fileId: 'file_a', draftSource: 'seed', hydratedFromLibrary: true, dirty: false, hasPendingEditorChanges: false,
      latestIssuedSaveSequence: 0, inFlightSaveSequence: null } };
  const change = patch => { state.session = { ...state.session, ...patch }; return state.session; };
  const store = {
    getSessionSnapshot: id => id === lid ? state.session : null,
    retainSession() { state.retains++; return () => state.retains--; },
    updateDraft({ content, source }) { return change({ draftContent: content, draftSource: source,
      dirty: content !== state.session.lastSavedContent, hasPendingEditorChanges: false }); },
    beginSave({ content }) {
      const s = state.session;
      if (!s.dirty || s.hasPendingEditorChanges || s.inFlightSaveSequence !== null || content !== s.draftContent) return null;
      const sequence = s.latestIssuedSaveSequence + 1;
      change({ latestIssuedSaveSequence: sequence, inFlightSaveSequence: sequence });
      return { saveSequence: sequence, expectedCurrentVersion: s.baseVersionNumber };
    },
    completeSaveSuccess({ saveSequence, submittedContent }) {
      if (saveSequence !== state.session.latestIssuedSaveSequence) return;
      change({ lastSavedContent: submittedContent, baseVersionNumber: null, inFlightSaveSequence: null,
        dirty: state.session.hasPendingEditorChanges || state.session.draftContent !== submittedContent });
    },
    completeSaveError({ saveSequence }) {
      if (saveSequence === state.session.latestIssuedSaveSequence)
        change({ inFlightSaveSequence: null, dirty: state.session.hasPendingEditorChanges || state.session.draftContent !== state.session.lastSavedContent });
    },
    acknowledgeLocalSave({ content }) { change({ lastSavedContent: content,
      dirty: state.session.hasPendingEditorChanges || state.session.draftContent !== content, hydratedFromLibrary: true }); },
    enqueueSave({ save }) {
      return state.holdQueue ? new Promise((resolve, reject) => { state.queued = () => Promise.resolve().then(save).then(resolve, reject); })
        : Promise.resolve().then(save);
    }
  };
  const client = { getQueryCache: () => ({ findAll: () => [{ state: { data: 'opaque-preview-key' } }] }),
    invalidateQueries(value) { state.invalidated.push(value); } };
  const selected = { id: 'client-a', serverId$: () => cid };
  const shared = { canvasConversations: () => [selected], canvasQueryClient: () => client, XM: () => ({}), Fl: () => false,
    HM: { getNodeIfExists: () => ({ message: state.message }), getCurrentLeafId: () => mid,
      getRequestId: () => null, getGizmoId: () => null },
    writingUpdateState: (_, callback) => callback({}), writingTreeOwner: { updateTree(_, callback) { callback({
      containsNode: id => id === mid, getMaybeMessage: () => state.message,
      updateNodeMessageMetadata(_, patch) { state.updates++; Object.assign(state.message.metadata, patch); }
    }); } } };
  const conversation = { writingLibrarySessions: () => store };
  const bindings = { state: () => ({ profile_id: 'web_20260912' }), load: async role => role === 'shared' ? shared : conversation,
    peek: role => role === 'shared' ? shared : conversation };
  const page = { crypto: require('node:crypto').webcrypto, document: {}, location: { href: 'https://chatgpt.com' + path },
    __elonChatGptDocumentToken: 'doc_library_test', __elonChatGptPrivateRuntimeBindings: bindings,
    __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy, __elonChatGptWritingBlockContext: context,
    __elonChatGptWritingLibrarySession: library, __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.current ? 'test-account' : null }) },
    setTimeout(fn, delay) { const timer = setTimeout(fn, delay); state.timerCallbacks.push({ timer, fn, delay }); return timer; }, clearTimeout,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      if (init.method === 'POST') {
        assert.equal(url, '/backend-api/conversation/message/writing-blocks');
        assert.notEqual(state.session.inFlightSaveSequence, null);
        const body = JSON.parse(init.body); state.posts.push(body); state.dispatched = true;
        if (state.effects) Object.assign(state.server.metadata.writing_blocks[bid], body.writing_block);
        state.onPost?.();
        if (state.postError) throw Error(state.postError);
        return {};
      }
      return { payload: { conversation_id: cid, is_do_not_remember: false, current_node: mid,
        mapping: { [mid]: { id: mid, parent: null, message: structuredClone(state.server) } } } };
    } } };
  const snapshot = () => ({ url: page.location.href, streaming: false, composerReady: false });
  const core = transport.create(page);
  const prepare = () => core.run({ operation: 'prepare', path, messageId: mid, id: bid, content: state.server.metadata.writing_blocks[bid].content }, false, snapshot);
  const save = (ticket, content = 'Native edit') => core.run({ operation: 'save', path, ticket, content }, true, snapshot);
  const verify = ticket => core.run({ operation: 'verify', path, ticket }, false, snapshot);
  return { state, store, change, core, page, bindings, shared, prepare, save, verify, mid, bid, lid, path };
}
module.exports = { harness, parser, policy };
