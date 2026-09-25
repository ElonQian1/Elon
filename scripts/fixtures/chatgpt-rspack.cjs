'use strict';
const path = require('node:path');
const assets = path.resolve(__dirname, '../../android/app/src/main/assets');
const runtimeApi = require(path.join(assets, 'chatgpt_web_rspack_runtime.js'));
const contextApi = require(path.join(assets, 'chatgpt_web_rspack_context.js'));
const submitApi = require(path.join(assets, 'chatgpt_web_rspack_submit.js'));
const CDN = 'https://chatgpt.com/cdn/assets/';
const files = ['633146.03cad12214.js', '908190.44b0fc59dd.js', '238022.3eafa0ab02.js',
  '586656.107574cbd4.js', '421899.a0eae5f5f4.js', '498514.f27755c4fc.js', '934244.56bcd8ce43.js'];
function fixture(options = {}) {
  const values = new Map(), calls = [], imports = [];
  const scopeToken = { id: Symbol(), __scopeBrand: 'AppScope' };
  const atom = value => { const ref = { scope: scopeToken }; values.set(ref, value); return ref; };
  let generation = 0, switching = false, identity = { accountId: 'fixture-account', userId: 'fixture-user', accessToken: 'not-a-credential' };
  const auth = { getBrowserChatGptAuthSnapshot: () => identity, getBrowserChatGptAuthGeneration: () => generation,
    isBrowserWorkspaceSwitchPending: () => switching, isBrowserAccountSwitchLoading: () => false };
  const identityAtoms = { d: atom(identity.accountId), i: atom(identity.userId), j: atom({ status: 'allowed' }) };
  const conversation = { i: atom(null), z: atom(null), w: atom(null), K: atom(null), T: atom('idle') };
  const composer = { r: atom(''), s: atom({ slug: 'auto', thinkingEffort: null }), u: atom([]), f: atom([]), h: atom(false) };
  const node = { token: scopeToken }, chain = new Map([[scopeToken.id, node]]);
  const scope = { node, scope: scopeToken, chain, get: key => values.get(key), set() {}, watch() {}, when() {} };
  const officialSubmit = { a: async (...args) => { calls.push(args); return true; } };
  const cache = Object.fromEntries(Object.entries({ OS: auth, c3: { a: scopeToken }, sAW: identityAtoms,
    LGwv: conversation, vG: composer, CUv: officialSubmit }).map(([id, exports]) => [id, { exports }]));
  const loader = () => { throw Error('Must not execute require'); };
  loader.c = cache; loader.m = {};
  const page = { location: new URL('https://chatgpt.com/'), __elonChatGptDocumentToken: 'doc_rspack_fixture',
    __elonChatGptPrivateTextTransactionsEnabled: true,
    performance: { getEntriesByType: () => files.map(file => ({ name: CDN + file })) },
    setTimeout, clearTimeout, AbortController, document: { body: {}, documentElement: {} } };
  const id = 'local-chatgpt:11111111-1111-4111-8111-111111111111';
  const root = { memoizedProps: {}, stateNode: {}, return: null };
  root.stateNode.current = root;
  const parent = { return: root, memoizedProps: { conversationId: id }, memoizedState: { memoizedState: { current: scope } } };
  const fiber = { return: parent, memoizedProps: {} };
  root.child = parent; parent.child = fiber;
  const editor = { isConnected: true, '__reactFiber$fixture': fiber };
  page.__elonChatGptRspackRuntime = runtimeApi.create(page, { importModule: async url => {
    imports.push(url); return options.importModule ? options.importModule() : { __webpack_require__: loader };
  } });
  page.__elonChatGptRspackContext = contextApi;
  const context = contextApi.create(page);
  const submit = submitApi.create(page, { timeoutMs: options.timeoutMs });
  const command = { prompt: 'fixture message', expectedDraft: '', requestId: 'mcp_test1', composer: editor,
    readDraft: () => values.get(composer.r), clearDraft: () => values.set(composer.r, '') };
  return { page, context, submit, command, values, scope, node, parent, root, editor, cache, loader, imports, calls,
    auth, identityAtoms, composer, conversation, officialSubmit,
    changeAccount: () => { identity = { ...identity, accountId: 'changed' }; generation++; },
    switchAccount: () => { switching = true; } };
}
module.exports = { fixture, files, CDN };
