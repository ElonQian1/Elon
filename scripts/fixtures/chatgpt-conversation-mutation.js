'use strict';
const mutationModule = require('../../android/app/src/main/assets/chatgpt_web_private_conversation_mutation.js');
const jsonRequest = require('../../android/app/src/main/assets/chatgpt_web_private_json_request.js');

function response(status, payload) {
  return { status, ok: status >= 200 && status < 300, text: async () => JSON.stringify(payload) };
}

function deferred() {
  let resolve;
  const promise = new Promise(accept => { resolve = accept; });
  return { promise, resolve };
}

function fixture(fetchImpl, enabled = true, rootOverrides = {}, directoryOverrides = {}) {
  const calls = [], accepted = [];
  const root = Object.assign({
    __elonChatGptPrivateJsonRequest: jsonRequest,
    __elonChatGptDocumentToken: 'doc_mutation_fixture',
    location: { origin: 'https://chatgpt.com' },
    AbortController, setTimeout, clearTimeout,
    fetch: async (url, init) => { calls.push({ url, init }); return fetchImpl(url, init); },
  }, rootOverrides);
  const headers = {
    Authorization: 'Bearer page-local-secret',
    Cookie: 'must-not-be-forwarded',
    'User-Agent': 'must-not-be-forwarded',
    'oai-device-id': 'page-local-device',
  };
  const privateTransport = {
    copySameOriginRequestHeaders: () => ({ ...headers }),
    acquireSameOriginRequestHeaders: async () => ({ ...headers }),
  };
  const directory = Object.assign({
    acceptPinnedState: (id, pinned) => { accepted.push({ id, pinned }); return true; },
    acceptTitleState: (id, title) => { accepted.push({ id, title }); return true; },
    acceptArchivedState: (id, archived) => { accepted.push({ id, archived }); return true; },
    acceptConversationMembership: (id, title, projectId) => { accepted.push({ id, title, projectId }); return true; },
    refreshProject: async () => false,
    snapshot: () => ({ conversations: [] }),
  }, directoryOverrides);
  return { calls, accepted, root, headers, privateTransport, directory,
    transport: mutationModule.create(root, { enabled, privateTransport, directory }) };
}

module.exports = { fixture, response, deferred };
