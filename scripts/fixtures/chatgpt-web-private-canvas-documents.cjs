'use strict';
const { webcrypto } = require('node:crypto');
const policy = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js');
const context = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_edit_context.js');
const observer = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_edit_observer.js');
const { observerFixture } = require('./chatgpt-canvas-edit-observer.cjs');
const documents = require('../../android/app/src/main/assets/chatgpt_web_private_canvas_documents.js');
const contract = require('../../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract.js');
const CID = '11111111-1111-4111-8111-111111111111', ID = 'synthetic_original_canvas';
const PATH = '/c/' + CID, LIST = '/backend-api/conversation/' + CID + '/textdocs', SAVE = '/backend-api/textdoc/' + ID;
const doc = (patch = {}) => ({ id: ID, title: 'Synthetic original', content: 'Draft content',
  version: 4, textdoc_type: 'document', comments: [], ...patch });

function fixture() {
  let now = 1000, hook = async () => {}, imports = 0;
  const requests = [], invalidations = [], rejectedAuth = [], rows = [doc()];
  const edits = { userEdits: {}, timestamps: {} }, headers = { Authorization: 'Bearer synthetic-identity' };
  const observation = observerFixture(edits);
  const page = { location: new URL('https://chatgpt.com' + PATH), document: observation.document, crypto: webcrypto,
    __elonChatGptDocumentToken: 'doc_synthetic', setTimeout, clearTimeout,
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateConversationShareContract: contract,
    __elonChatGptPrivateCanvasEditObserver: observer,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ ...headers }) },
    __elonChatGptPrivateAuthContext: { invalidate: code => rejectedAuth.push(code) },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, init, limits) => {
      requests.push({ url, init, limits });
      const override = await hook({ url, init, limits });
      if (override) return override;
      if (init.method === 'GET') {
        if (!/^\/backend-api\/conversation\/[a-f0-9-]+\/textdocs$/.test(url)) throw Error('unknown_request');
        return { payload: structuredClone(rows) };
      }
      if (url === SAVE + '/rename' && init.method === 'POST') {
        rows[0].title = JSON.parse(init.body).title;
        return { status: 204, ok: true };
      }
      if (url !== SAVE || init.method !== 'POST') throw Error('unknown_request');
      const body = JSON.parse(init.body);
      if (rows[0].version !== body.version) throw Error('http_409');
      Object.assign(rows[0], body, { version: body.version + 1 });
      return { payload: { version: rows[0].version } };
    } },
    __elonChatGptPrivateRuntimeBindings: { observed: () => true, state: () => ({ profile_id: 'web_20260912' }), load: async role => {
      imports += 1;
      if (role === 'conversation') return observation.conversation;
      if (role === 'react') return observation.runtime;
      return { canvasQueryClient: () => ({ ...observation.client,
        invalidateQueries: async value => { invalidations.push(value); } }) };
    } }
  };
  const snapshot = { url: page.location.href, streaming: false, composerReady: false };
  const api = documents.create(page, { policy, context, now: () => now });
  const run = (input, confirmed = false) => api.run({ path: page.location.pathname, ...input }, confirmed, () => snapshot);
  return { page, snapshot, api, run, requests, rows, edits, headers, invalidations, rejectedAuth,
    list: force => run({ operation: 'list', force }),
    save: (ticket, patch = {}, confirmed = true) => run({ operation: 'save', ticket, id: ID, content: 'Edited content', comments: [], ...patch }, confirmed),
    advance: value => { now += value; }, setHook: value => { hook = value; }, imports: () => imports,
    navigate: path => { page.location = new URL('https://chatgpt.com' + path); snapshot.url = page.location.href; } };
}
module.exports = { fixture, CID, ID, PATH, LIST, SAVE, doc };
