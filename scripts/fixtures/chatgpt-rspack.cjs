'use strict';
const path = require('node:path');
const assets = path.resolve(__dirname, '../../android/app/src/main/assets');
const runtimeApi = require(path.join(assets, 'chatgpt_web_rspack_runtime.js'));
const contextApi = require(path.join(assets, 'chatgpt_web_rspack_context.js'));
const submitApi = require(path.join(assets, 'chatgpt_web_rspack_submit.js'));
const CDN = 'https://chatgpt.com/cdn/assets/';
const files = ['633146.03cad12214.js', '908190.44b0fc59dd.js', '238022.3eafa0ab02.js',
  '586656.107574cbd4.js', '421899.a0eae5f5f4.js', '498514.f27755c4fc.js', '934244.56bcd8ce43.js'];
const newFiles = ['633146.6ed5d111e4.js', '908190.d446cd6dfd.js', '238022.59b5f57fb1.js',
  '376616.59ccddcc11.js', '36750.72bb8d082e.js', '498514.937d7074b7.js', '934244.c57697fda7.js'];
const currentFiles = ['633146.e8647fddbe.js', '908190.80f53e7a66.js', '238022.33322e145f.js',
  '376616.5a8098a4e7.js', '184143.0e420b28d4.js', '109686.cd293bd41c.js', '934244.b068731984.js'];
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
  const scope = { node, scope: scopeToken, chain, get: key => values.get(key),
    set(key, id, update) { values.set(key, typeof update === 'function' ? update(values.get(key)) : update); }, watch() {}, when() {} };
  const officialSubmit = { a: async (...args) => { calls.push(args); return true; } };
  const cache = Object.fromEntries(Object.entries({ OS: auth, c3: { a: scopeToken }, sAW: identityAtoms,
    LGwv: conversation, vG: composer, CUv: officialSubmit }).map(([id, exports]) => [id, { exports }]));
  if (options.latest) {
    cache.sA = cache.sAW; delete cache.sAW;
    cache[options.latest === 'current' ? 'JqV' : 'wg'] = cache.LGwv; delete cache.LGwv;
    cache.q4q = { exports: { m: entries => entries.filter(e => e.status === 'ready').map(({ uploadId, status, ...spec }) => spec) } };
  }
  const loader = () => { throw Error('Must not execute require'); };
  loader.c = cache; loader.m = {};
  const page = { location: new URL('https://chatgpt.com/'), __elonChatGptDocumentToken: 'doc_rspack_fixture',
    __elonChatGptPrivateTextTransactionsEnabled: true,
    performance: { getEntriesByType: () => (options.latest === 'current' ? currentFiles : options.latest ? newFiles : files)
      .map(file => ({ name: CDN + file })) },
    setTimeout, clearTimeout, setInterval, clearInterval, AbortController, document: { body: {}, documentElement: {} } };
  const id = 'local-chatgpt:11111111-1111-4111-8111-111111111111';
  const root = { memoizedProps: {}, stateNode: {}, return: null };
  root.stateNode.current = root;
  const parent = { return: root, memoizedProps: { conversationId: id }, memoizedState: { memoizedState: { current: scope } } };
  const fiber = { return: parent, memoizedProps: {} };
  root.child = parent; parent.child = fiber;
  const editor = { isConnected: true, '__reactFiber$fixture': fiber };
  page.document.querySelector = selector => selector === '#prompt-textarea' ? editor : null;
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
module.exports = { fixture, files, newFiles, CDN };
