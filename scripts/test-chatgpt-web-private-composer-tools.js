'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const { createPrivateRuntime } = require(path.join(assets, 'chatgpt_web_adapter_composer_tool_selection.js'));
const RUNTIME = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
const CID = '00000000-0000-0000-0000-000000000001';
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };

function fixture(options = {}) {
  const calls = [], events = [], results = [], timers = new Set();
  let loaded = true, account = 'Bearer fixture-only-not-a-credential', expanded = false;
  const conversation = { serverId$: () => CID }, controller = { conversation };
  const props = { conversation, composerController: controller, composerDisabled: false,
    selectModelId() {}, clearModelSelection() {}, isTemporaryChat: false,
    currentModelId: 'fixture-model', availableSystemHints: [{ systemHint: 'search' }, { systemHint: 'picture_v2' }] };
  const top = { stateNode: {} };
  top.stateNode.current = top;
  const ancestor = { memoizedProps: props, return: top };
  const host = { return: ancestor };
  const node = { isConnected: true, __reactFiber$fixture: host,
    getAttribute: name => name === 'aria-expanded' ? String(expanded) : null };
  let signal = { locked: false, activeSystemHintType: null,
    activeConnectorSystemHintTypes: new Set(), activeCustomAgentSystemHintType: null };
  const namespace = {
    Ng(value) { assert.equal(value, controller); return signal; },
    Bg(value, hint, command) {
      assert.equal(value, controller);
      calls.push({ hint, command });
      if (signal.activeSystemHintType === command.ifPrevSystemHint && !signal.locked) {
        signal = { ...signal, activeSystemHintType: hint };
      }
    }
  };
  const page = { location: { href: 'https://chatgpt.com/c/' + CID },
    __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    performance: { getEntriesByName(url, type) { assert.equal(url, RUNTIME); assert.equal(type, 'resource'); return loaded ? [{}] : []; } },
    document: { querySelector: selector => selector === '#composer-plus-btn' ? node : null },
    setTimeout(fn) { timers.add(fn); return fn; }, clearTimeout(fn) { timers.delete(fn); } };
  let imports = 0, fallbacks = 0, snapshots = 0;
  const runtime = createPrivateRuntime(page, { loadRuntime: async url => {
    assert.equal(url, RUNTIME); imports++;
    return options.loadRuntime ? options.loadRuntime(namespace) : namespace;
  } });
  const result = (...value) => results.push(value);
  const emit = list => events.push(list);
  const list = () => runtime.requestPrivateOptions(emit, result, () => { fallbacks++; });
  const pick = id => runtime.selectPrivate(id, result, () => { snapshots++; });
  const choice = semantic => events.at(-1).find(item => item.semantic === semantic);
  return { page, node, host, top, ancestor, props, controller, conversation, namespace,
    runtime, calls, events, results, timers, result, list, pick, choice,
    get state() { return signal; }, set state(value) { signal = value; },
    set loaded(value) { loaded = value; }, set account(value) { account = value; },
    set expanded(value) { expanded = value; },
    get imports() { return imports; }, get fallbacks() { return fallbacks; }, get snapshots() { return snapshots; } };
}

test('search and image selection use the existing official signal without autofocus or DOM polling', async () => {
  const f = fixture();
  assert.equal(f.list(), true); await flush();
  assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.imports, 1);
  for (const semantic of ['web_search', 'image_generation']) {
    const option = f.choice(semantic);
    assert.equal(f.pick(option.id), true);
    assert.equal(f.results.at(-1)[1], true);
    const hint = semantic === 'web_search' ? 'search' : 'picture_v2';
    // A send-preparation fixture reads the same official signal, not a parallel native label.
    assert.equal(f.namespace.Ng(f.controller).activeSystemHintType, hint);
    assert.equal(f.calls.at(-1).command.skipComposerAutofocus, true);
    assert.equal(f.choice(semantic).selected, true);
    const count = f.calls.length;
    f.pick(option.id);
    assert.equal(f.calls.length, count, 'retry must not toggle the tool off');
    assert.equal(f.results.at(-1)[1], true);
    f.pick(f.choice(semantic).id);
    assert.equal(f.state.activeSystemHintType, null);
  }
  f.list();
  assert.equal(f.imports, 1, 'warm menu must not reload the module');
  assert.equal(f.timers.size, 0, 'no steady-state timer or polling');
  assert.equal(f.fallbacks, 0);
  for (const item of f.events.flat()) {
    assert.deepEqual(Object.keys(item).sort(), ['id', 'kind', 'label', 'opensSubmenu', 'selected', 'semantic']);
    assert.ok(item.id.length < 64);
  }
  assert.doesNotMatch(JSON.stringify([f.events, f.results]), /Bearer|fixture-model|00000000|controller|headers/i);
});

for (const [name, mutate] of Object.entries({
  route: f => { f.page.location.href = 'https://chatgpt.com/'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_other_1'; },
  account: f => { f.account = 'Bearer different-fixture-account'; },
  model: f => { f.props.currentModelId = 'different-model'; },
  controller: f => { f.props.composerController = { conversation: f.conversation }; },
  detached: f => { f.node.isConnected = false; },
  eligibility: f => { f.props.availableSystemHints = [{ systemHint: 'search' }]; },
  locked: f => { f.state = { ...f.state, locked: true }; },
  connector: f => { f.state.activeConnectorSystemHintTypes.add('connector-fixture'); },
  'unrelated tool': f => { f.state.activeSystemHintType = 'research'; },
  'another choice': f => { f.state.activeSystemHintType = 'picture_v2'; }
})) {
  test('stale selection rejects changed ' + name + ' without replay', async () => {
    const f = fixture(); f.list(); await flush();
    const id = f.choice('web_search').id;
    mutate(f); f.pick(id);
    assert.equal(f.results.at(-1)[1], false);
    assert.equal(f.calls.length, 0);
    assert.equal(f.fallbacks, 0);
  });
}

for (const [name, mutate] of Object.entries({
  'absent runtime': f => { f.loaded = false; },
  'missing fiber': f => { delete f.node.__reactFiber$fixture; },
  'uncommitted fiber': f => { f.top.stateNode.current = {}; },
  'disabled composer': f => { f.props.composerDisabled = true; },
  'files only': f => { f.props.composerToolAvailability = 'files_only'; },
  'login gate': f => { f.props.loginModalGate = { shouldGateToLoginModal: true }; },
  'logged-out upsell': f => { f.props.availableSystemHints[0].isLoggedOutUpsell = true; },
  'hidden tool': f => { f.props.availableSystemHints[0].hideFromInitialSelection = true; },
  'duplicate tool': f => { f.props.availableSystemHints.push({ systemHint: 'search' }); },
  'wrong conversation': f => { f.conversation.serverId$ = () => null; },
  'ambiguous controller': f => {
    f.host.return = { memoizedProps: { ...f.props, composerController: { conversation: f.conversation } }, return: f.ancestor };
  },
  'menu already open': f => { f.expanded = true; }
})) {
  test('unknown ' + name + ' leaves the existing path available', async () => {
    const f = fixture(); mutate(f);
    assert.equal(f.list(), false); await flush();
    assert.equal(f.imports, 0);
    assert.equal(f.calls.length, 0);
    assert.equal(f.results.length, 0, 'not an unsupported-capability error');
  });
}

test('committed alternate is accepted instead of obsolete host props', async () => {
  const f = fixture();
  const oldTop = { stateNode: f.top.stateNode };
  f.node.__reactFiber$fixture = { return: { memoizedProps: { ...f.props, composerDisabled: true }, return: oldTop }, alternate: f.host };
  assert.equal(f.list(), true); await flush();
  assert.equal(f.results.at(-1)[1], true);
});

for (const route of ['/', '/?temporary-chat=true', '/g/g-p-' + 'a'.repeat(32) + '-fixture/project']) {
  test('new conversation binding supports ' + route, async () => {
    const f = fixture(); f.page.location.href = 'https://chatgpt.com' + route;
    f.conversation.serverId$ = () => undefined;
    f.props.isTemporaryChat = route.includes('temporary');
    assert.equal(f.list(), true); await flush();
    f.pick(f.choice('image_generation').id);
    assert.equal(f.results.at(-1)[1], true);
  });
}

test('unexpected runtime export falls back before mutation and cools down', async () => {
  const f = fixture({ loadRuntime: () => ({}) });
  assert.equal(f.list(), true); await flush();
  assert.equal(f.fallbacks, 1);
  assert.equal(f.list(), false);
  assert.equal(f.imports, 1);
  assert.equal(f.calls.length, 0);
});

test('runtime timeout is bounded and late import cannot apply a selection', async () => {
  let resolve;
  const f = fixture({ loadRuntime: () => new Promise(r => { resolve = r; }) });
  f.list(); await flush();
  assert.equal(f.timers.size, 1);
  [...f.timers][0](); await flush();
  assert.equal(f.fallbacks, 1);
  assert.equal(f.timers.size, 0);
  resolve(f.namespace); await flush();
  assert.equal(f.events.length, 0);
  assert.equal(f.calls.length, 0);
});

test('overlapping menu requests share one import and only newest may complete', async () => {
  let resolve;
  const f = fixture({ loadRuntime: () => new Promise(r => { resolve = r; }) });
  f.list(); f.list(); await flush();
  assert.equal(f.imports, 1);
  assert.deepEqual(f.results.map(row => row[1]), [false]);
  resolve(f.namespace); await flush();
  assert.deepEqual(f.results.map(row => row[1]), [false, true]);
  assert.equal(f.events.length, 1);
});

test('context change during module load cannot fall back into the next conversation', async () => {
  let resolve;
  const f = fixture({ loadRuntime: () => new Promise(r => { resolve = r; }) });
  f.list(); await flush();
  f.page.location.href = 'https://chatgpt.com/';
  resolve({}); await flush();
  assert.equal(f.results.at(-1)[1], false);
  assert.equal(f.fallbacks, 0);
  assert.equal(f.events.length, 0);
});

for (const mode of ['throw', 'ignore', 'mutate then throw']) {
  test('setter ' + mode + ' cannot report success or trigger DOM replay', async () => {
    const f = fixture(); f.list(); await flush();
    const original = f.namespace.Bg;
    f.namespace.Bg = (...args) => {
      if (mode === 'mutate then throw') original(...args);
      if (mode !== 'ignore') throw new Error('fixture');
    };
    f.pick(f.choice('web_search').id);
    assert.equal(f.results.at(-1)[1], false);
    assert.equal(f.fallbacks, 0);
  });
}

test('dismiss cancels pending request and invalidates handles without webpage interactions', async () => {
  const f = fixture(); f.list();
  assert.equal(f.runtime.dismissPrivateOptions(), true); await flush();
  assert.equal(f.events.length, 0);
  f.list(); await flush();
  const id = f.choice('web_search').id;
  assert.equal(f.runtime.dismissPrivateOptions(), true);
  f.pick(id);
  assert.equal(f.results.at(-1)[1], false);
  assert.equal(f.calls.length, 0);
  assert.equal(f.pick('ordinary_dom_id'), false);
});

test('production composer delegates request, selection and dismiss without a synthetic touch', () => {
  const calls = [], events = [], results = [];
  const sandbox = { document: { querySelector: () => null, querySelectorAll: () => [] },
    location: { origin: 'https://chatgpt.com' }, window: {
      __elonChatGptComposerToolSelection: {
        requestPrivateOptions(emit, result) { calls.push('list'); emit([]); result('list_composer_tools', true, ''); return true; },
        selectPrivate() { calls.push('select'); return true; },
        dismissPrivateOptions() { calls.push('dismiss'); return true; }
      },
      __elonChatGptDictationActions: { create: () => ({}) },
      __elonChatGptComposerDismissPolicy: require(path.join(assets, 'chatgpt_web_adapter_composer_dismiss_policy.js')),
      __elonChatGptComposerSubmenu: { createRecovery: () => ({}) }
    } };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_composer.js'), 'utf8'), sandbox);
  const composer = sandbox.window.__elonChatGptComposer;
  composer.requestOptions('tools', null, event => events.push(event), (...value) => results.push(value));
  composer.selectOption('tools', 'private_tool_fixture', null, event => events.push(event), () => {}, () => {});
  composer.dismissOpenMenu(null, event => events.push(event), (...value) => results.push(value));
  assert.deepEqual(calls, ['list', 'select', 'dismiss']);
  assert.equal(events.some(event => event.type === 'web_touch_request'), false);
  assert.equal(results.at(-1)[0], 'dismiss_composer_menu');
});

test('production composer uses actual private binding, updates selection and dismisses without DOM touch', async () => {
  const f = fixture(), events = [], results = [];
  const page = f.page;
  page.document.querySelectorAll = () => [];
  page.getComputedStyle = () => ({ display: 'block', visibility: 'visible' });
  f.node.getBoundingClientRect = () => ({ width: 40, height: 40, left: 0, top: 0 });
  page.__elonChatGptDictationActions = { create: () => ({}) };
  page.__elonChatGptComposerSubmenu = { createRecovery: () => ({}) };
  page.__elonChatGptComposerDismissPolicy = require(path.join(assets, 'chatgpt_web_adapter_composer_dismiss_policy.js'));
  page.__elonChatGptComposerToolSelection = f.runtime;
  const input = { closest: () => page.document };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_composer.js'), 'utf8'), {
    window: page, document: page.document, location: { origin: 'https://chatgpt.com' }
  });
  const composer = page.__elonChatGptComposer, emit = event => events.push(event), result = (...value) => results.push(value);
  composer.requestOptions('tools', input, emit, result); await flush();
  const option = events.at(-1).options.find(item => item.semantic === 'web_search');
  composer.selectOption('tools', option.id, input, emit, result, () => {});
  assert.equal(f.state.activeSystemHintType, 'search');
  assert.equal(events.at(-1).options.find(item => item.semantic === 'web_search').selected, true);
  assert.equal(results.at(-1)[1], true);
  composer.dismissOpenMenu(input, emit, result);
  assert.equal(events.some(event => event.type === 'web_touch_request'), false);
  assert.equal(results.at(-1)[0], 'dismiss_composer_menu');
  assert.equal(f.timers.size, 0);
});
