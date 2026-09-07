'use strict';
const path = require('node:path');
const assets = path.join(__dirname, '../../android/app/src/main/assets');
const contract = require(path.join(assets, 'chatgpt_web_private_conversation_share_contract.js'));
const share = require(path.join(assets, 'chatgpt_web_private_conversation_share.js'));
const CID = '11111111-1111-4111-8111-111111111111';
const NODE = '22222222-2222-4222-8222-222222222222';
const NEXT = '33333333-3333-4333-8333-333333333333';
const SID = '44444444-4444-4444-8444-444444444444';
const PATH = '/c/' + CID;
const LINK = 'https://chatgpt.com/share/' + SID;
const MODERATION = { has_been_auto_blocked: false, has_been_auto_moderated: false, has_been_blocked: false };

function fixture(variant = 'control') {
  const requests = [];
  const headers = { Authorization: 'Bearer synthetic-test-only', 'chatgpt-account-id': 'synthetic-personal',
    'chatgpt-sentinel-proof-token': 'must-not-replay', cookie: 'must-not-export' };
  const thread = { isLoading: false, is_do_not_remember: false, contextScopes: [], leaf: NODE, node: NODE, gizmo: null };
  const account = { isQuorum: () => false, isWorkspaceAccount: () => false };
  const modules = {
    shared: { H3: () => true, mq: () => account, wV: selector => selector({ personal: true }),
      SV: { isPersonalWorkspace: value => value.personal }, XM: id => id === CID ? thread : null,
      HM: { getGizmoId: t => t.gizmo, getCurrentLeafId: t => t.leaf, hasNode: (t, id) => [NODE, NEXT].includes(id) } },
    conversation: { AGt: t => t.node, J5t: options => {
      require('node:assert/strict').deepEqual(options, { disableExposureLog: true });
      return variant;
    } },
  };
  const snapshot = { url: 'https://chatgpt.com' + PATH, composerReady: true, streaming: false, attachments: [] };
  let loaded = true;
  const page = {
    location: { origin: 'https://chatgpt.com', href: snapshot.url }, __elonChatGptDocumentToken: 'doc_share_test',
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => headers },
    performance: { getEntriesByName: () => loaded ? [{}] : [] }, document: { querySelector: () => null },
    setTimeout, clearTimeout,
    __elonChatGptPrivateJsonRequest: { request: async (_page, url, init, limits) => {
      requests.push({ url, init, limits });
      if (init.method === 'POST') return { payload: { share_id: SID, share_url: LINK, current_node_id: NODE,
        is_visible: true, is_public: url.endsWith('/v2/create'), is_anonymous: true,
        title: 'Synthetic fixture', moderation_state: MODERATION } };
      return { payload: { moderation_state: MODERATION } };
    } },
  };
  const loadRuntime = async url => url.includes('4813494d') ? modules.shared : modules.conversation;
  const api = share.create(page, { contract, loadRuntime });
  return { page, requests, headers, thread, modules, account, snapshot, api,
    start: (confirmed = true) => api.start(PATH, confirmed, () => snapshot),
    setLoaded: value => { loaded = value; }, setVariant: value => { variant = value; }, loadRuntime };
}

module.exports = { fixture, CID, NODE, NEXT, SID, PATH, LINK, MODERATION };
