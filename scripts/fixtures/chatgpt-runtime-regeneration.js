'use strict';

const contract = require('../../android/app/src/main/assets/chatgpt_web_private_regenerate_contract.js');
const models = require('../../android/app/src/main/assets/chatgpt_web_private_model_contract.js');
const runtime = require('../../android/app/src/main/assets/chatgpt_web_private_regenerate_runtime.js');
const id = n => '00000000-0000-4000-8000-' + String(n).padStart(12, '0');
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };

function fixture(options = {}) {
  const calls = [], timers = new Map(), listeners = new Set();
  const identity = { userId: 'synthetic-user', accountId: id(7), loggedIn: true };
  let sequence = 0, prepared = 0, activeStream = null;
  const cid = id(1), conversation = { id: 'client_synthetic', serverId$: () => cid };
  const message = { id: id(3), author: { role: 'assistant' },
    content: { content_type: 'text', parts: ['synthetic original'] }, status: 'finished_successfully' };
  const parent = { id: id(2), message: { author: { role: 'user' } } };
  const tree = { leaf: message.id, variants: [message.id, id(4)],
    nodes: new Map([[message.id, { message, parent }]]) };
  const modelMenu = { conversation, composerIntelligencePickerState: {},
    modelsData: { models: new Map() }, modelSwitcherDenialsBySlug: {} };
  const callback = value => { calls.push(value); options.invoke?.(value); };
  const owner = { conversation, lastMessage: message, onModelSelect: callback };
  const menu = { ...owner, canRegenerateResponse: true, hasImageGenMessage: false,
    retryOption: { value: 'synthetic-model', shouldShowUpsell: false } };
  const element = props => ({ $$typeof: Symbol.for('react.transitional.element'), props });
  const root = { memoizedProps: {}, return: null, stateNode: {} }; root.stateNode.current = root;
  const ownerFiber = { memoizedProps: owner, return: root };
  const menuRoot = { memoizedProps: { children: [
    element({ children: 'synthetic model label' }),
    element({ children: element({ children: [element(menu)] }) })
  ] }, return: ownerFiber };
  const button = { __reactFiber$test: { return: menuRoot }, isConnected: true };
  const turn = { isConnected: true, querySelectorAll: () => [button] };
  const modelFiber = { memoizedProps: { dropdownContent: element(modelMenu), ariaDisabled: false, dropdownOpen: false }, return: root };
  const modelButton = { isConnected: true, __reactFiber$test: { return: modelFiber } };
  root.child = ownerFiber; ownerFiber.sibling = modelFiber;
  ownerFiber.child = menuRoot; menuRoot.child = button.__reactFiber$test;
  modelFiber.child = modelButton.__reactFiber$test;
  const page = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + cid, pathname: '/c/' + cid },
    __elonChatGptDocumentToken: 'doc_regenerate_test', __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: 'Bearer synthetic-only-identity' }) },
    __elonChatGptPrivateModelContract: models,
    document: { querySelector: () => null }, performance: { getEntriesByName: () => [{}] },
    Event: class { constructor(type) { this.type = type; } },
    setTimeout(fn, ms) { const token = ++sequence; timers.set(token, { fn, ms }); return token; },
    clearTimeout(token) { timers.delete(token); }
  };
  const shared = {
    H3: () => identity.loggedIn,
    F5: () => identity.loggedIn ? { user: { id: identity.userId }, account: { id: identity.accountId } } : null,
    mq: () => ({ id: identity.accountId, authUserId: identity.userId }),
    XM: () => tree, HM: {
      getCurrentLeafId: value => value.leaf, getVariantIds: value => value.variants,
      getParentPromptNode: (value, key) => value.nodes.get(key)?.parent,
      getNode: (value, key) => value.nodes.get(key)
    }
  };
  const conversationModule = { f8t: ({ modelSlug, modelSwitcherDenialsBySlug }) =>
    !modelSwitcherDenialsBySlug[modelSlug] };
  const modules = { shared, conversation: conversationModule };
  const stream = {
    subscribe(fn) { listeners.add(fn); return () => listeners.delete(fn); },
    current: () => activeStream,
    prepareSend() { prepared++; activeStream = null; }
  };
  page.__elonChatGptPrivateStreamTransport = stream;
  const api = runtime.create(page, { contract,
    loadRuntime: options.loadRuntime || (async url => url.includes('conversation-small') ? conversationModule : shared) });
  page.__elonChatGptPrivateRegenerateRuntime = api;
  const command = { requestId: 'mcp_retry', turn, getModelTrigger: () => modelButton };

  function publish(value = {}, commit = true) {
    const key = value.id || id(5);
    if (commit) {
      tree.leaf = key;
      tree.nodes.set(key, { message: { id: key, author: { role: 'assistant' } }, parent });
    }
    activeStream = { id: key, conversationId: cid, text: 'synthetic variant', state: 'streaming', ...value };
    for (const fn of listeners) fn();
  }
  function runTimer(ms) {
    const entry = [...timers.entries()].find(([, value]) => value.ms === ms);
    if (entry) { timers.delete(entry[0]); entry[1].fn(); }
    return !!entry;
  }
  return { api, page, command, modelMenu, modelButton, menu, turn, root, button, menuRoot,
    tree, modules, identity, message, parent, calls, timers, listeners, publish, runTimer, prepared: () => prepared };
}

module.exports = { fixture, id, flush };
