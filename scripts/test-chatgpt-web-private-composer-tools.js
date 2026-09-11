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
const flush = async () => { for (let i = 0; i < 60; i++) await Promise.resolve(); };
const toolContext = require(path.join(assets, 'chatgpt_web_private_composer_tool_context.js'));

function fixture(options = {}) {
  const calls = [], events = [], results = [], timers = new Set();
  let loaded = true, account = 'Bearer fixture-only-not-a-credential', expanded = false;
  const conversation = { serverId$: () => CID }, controller = { conversation };
  const props = { conversation, composerController: controller, composerDisabled: false,
    selectModelId() {}, clearModelSelection() {}, isTemporaryChat: false,
    currentModelId: 'fixture-model', availableSystemHints: [{ systemHint: 'search' }, { systemHint: 'picture_v2' }] };
  const shared = { getSharedProps: () => ({ ...props, isNewThread: true }), subscribeToSharedProps() {} };
  const files = { files$: () => [], readyFiles$: () => [], hasUploadInProgress$: () => false };
  const menu = { conversation, clearModelSelection: props.clearModelSelection,
    availableSystemHints: props.availableSystemHints, resolveSystemHintBehavior: () => undefined,
    isConsumerLockdownModeEnabled: false, isLoading: false };
  const memo = Array(265);
  memo[56] = props.availableSystemHints;
  memo[63] = memo[64] = memo[65] = false;
  memo[199] = { props: { children: [{ props: menu }] } };
  const top = { stateNode: {} };
  top.stateNode.current = top;
  const ancestor = { type: { name: options.current ? 'Kgn' : 'Whn' }, memoizedProps: props,
    updateQueue: { memoCache: { data: [memo] } }, return: top };
  const host = { return: ancestor, dependencies: { firstContext: { memoizedValue: shared,
    next: { memoizedValue: files } } } };
  top.child = ancestor; ancestor.child = host;
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
    performance: { getEntriesByName(url) { return loaded && [RUNTIME,
      'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js'].includes(url) ? [{}] : []; } },
    document: { querySelector: selector => ['#composer-plus-btn', '#prompt-textarea'].includes(selector) ? node : null },
    setTimeout(fn) { timers.add(fn); return fn; }, clearTimeout(fn) { timers.delete(fn); } };
  let imports = 0, fallbacks = 0, snapshots = 0;
  Object.defineProperty(page.location, 'origin', { get: () => new URL(page.location.href).origin });
  const guest = { authStatus: 'logged_out', session: null };
  const sharedRuntime = { R5: () => ({ authStatus: guest.authStatus }), F5: () => guest.session };
  if (options.current) {
    const bridge = require('./fixtures/chatgpt-runtime-bindings').attach(page, { composer: namespace, shared: sharedRuntime });
    page.performance.getEntriesByName = url => loaded && bridge.observed.has(url) ? [{}] : [];
  } else {
    page.__elonChatGptPrivateRuntimeBindings = require(path.join(assets, 'chatgpt_web_private_runtime_bindings')).create(page, {
      loadRuntime: async url => {
        if (url.includes('4813494d-')) return sharedRuntime;
        assert.equal(url, RUNTIME); imports++;
        return options.loadRuntime ? options.loadRuntime(namespace) : namespace;
      }
    });
  }
  page.__elonChatGptPrivateTextRuntimeSubmit = require(path.join(assets, 'chatgpt_web_private_text_runtime_submit')).create(page);
  const runtime = createPrivateRuntime(page);
  const result = (...value) => results.push(value);
  const emit = list => events.push(list);
  const list = () => runtime.requestPrivateOptions(emit, result, () => { fallbacks++; });
  const pick = id => runtime.selectPrivate(id, result, () => { snapshots++; });
  const choice = semantic => events.at(-1).find(item => item.semantic === semantic);
  return { page, node, host, top, ancestor, props, controller, conversation, namespace, menu, memo, guest,
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
  'all logged-out upsell': f => { f.props.availableSystemHints.forEach(h => { h.isLoggedOutUpsell = true; }); },
  'all hidden tools': f => { f.props.availableSystemHints.forEach(h => { h.hideFromInitialSelection = true; }); },
  'duplicate tool': f => { f.props.availableSystemHints.push({ systemHint: 'search' }); },
  'wrong conversation': f => { f.conversation.serverId$ = () => null; },
  'ambiguous controller': f => {
    const duplicate = { ...f.ancestor, child: f.host, return: f.ancestor };
    f.host.return = duplicate; f.ancestor.child = duplicate;
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

test('production assembly loads tool context before its consumer', () => {
  const catalog = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  assert.ok(catalog.indexOf('chatgpt_web_private_composer_tool_context.js') < catalog.indexOf('chatgpt_web_adapter_composer_tool_selection.js'));
});

function useMenuAssignedId(f) {
  f.node.id = 'radix-fixture-trigger';
  f.page.document.querySelector = selector =>
    ['[data-testid="composer-plus-btn"]', '#prompt-textarea'].includes(selector) ? f.node : null;
}

test('menu-assigned DOM id preserves private tool selection through the official test id', async () => {
  const f = fixture({ current: true }); useMenuAssignedId(f);
  assert.equal(f.list(), true); await flush();
  assert.equal(toolContext.state(f.page), 'ready');
  f.pick(f.choice('web_search').id);
  assert.equal(f.state.activeSystemHintType, 'search');
  f.pick(f.choice('web_search').id);
  assert.equal(f.state.activeSystemHintType, null);
  assert.equal(f.fallbacks, 0);
  assert.equal(f.timers.size, 0);
});

test('the same test-id anchor enforces the open official-menu guard', async () => {
  const f = fixture({ current: true }); useMenuAssignedId(f);
  f.expanded = true;
  assert.equal(f.list(), false); await flush();
  assert.equal(f.calls.length, 0);
  assert.equal(f.results.length, 0);
});

for (const invalid of ['detached', 'foreign owner']) {
  test('test-id lookup still rejects ' + invalid, async () => {
    const f = fixture({ current: true }); useMenuAssignedId(f);
    if (invalid === 'detached') f.node.isConnected = false;
    else f.ancestor.type = { name: 'Foreign' };
    assert.equal(f.list(), false); await flush();
    assert.equal(f.calls.length, 0);
    assert.equal(f.results.length, 0);
  });
}

for (const current of [false, true]) {
  test('guest search uses the current official store with image upsell excluded: ' + current, async () => {
    const f = fixture({ current });
    f.page.location.href = 'https://chatgpt.com/';
    f.page.__elonChatGptPrivateTransport = null;
    f.props.availableSystemHints[1].isLoggedOutUpsell = true;
    await f.page.__elonChatGptPrivateRuntimeBindings.load('shared');
    // A guest conversation can receive a server ID while staying on the homepage.
    assert.equal(f.conversation.serverId$(), CID);
    assert.equal(f.list(), true); await flush();
    assert.deepEqual(f.events.at(-1).map(item => item.semantic), ['web_search']);
    f.pick(f.choice('web_search').id);
    assert.equal(f.results.at(-1)[1], true);
    assert.equal(f.state.activeSystemHintType, 'search');
    const count = f.calls.length;
    f.list(); await flush();
    assert.equal(f.calls.length, count);
    f.pick(f.choice('web_search').id);
    assert.equal(f.state.activeSystemHintType, null);
    assert.equal(f.fallbacks, 0);
    assert.equal(f.timers.size, 0);
  });
}

test('study and canvas share the official live signal and idempotent selection owner', async () => {
  const f = fixture({ current: true });
  f.props.availableSystemHints.push({ systemHint: 'tatertot' }, { systemHint: 'canvas' });
  assert.equal(f.list(), true); await flush();
  assert.deepEqual(f.events.at(-1).map(item => item.semantic), ['web_search', 'image_generation', 'study', 'canvas']);
  for (const [semantic, hint] of [['study', 'tatertot'], ['canvas', 'canvas'], ['web_search', 'search']]) {
    const option = f.choice(semantic);
    assert.equal(f.pick(option.id), true);
    assert.equal(f.results.at(-1)[1], true);
    assert.equal(f.namespace.Ng(f.controller).activeSystemHintType, hint);
    assert.deepEqual(f.events.at(-1).filter(item => item.selected).map(item => item.semantic), [semantic]);
    const writes = f.calls.length;
    f.pick(option.id);
    assert.equal(f.calls.length, writes, 'same request never toggles or sends twice');
    assert.equal(f.calls.at(-1).command.skipComposerAutofocus, true);
  }
  f.pick(f.choice('web_search').id);
  assert.equal(f.namespace.Ng(f.controller).activeSystemHintType, null);
  assert.equal(f.fallbacks, 0);
  assert.equal(f.timers.size, 0);
});

for (const hint of ['tatertot', 'canvas']) {
  for (const restriction of ['filtered', 'disabled', 'upsell', 'hidden', 'local_action']) {
    test(hint + ' retains official ' + restriction + ' eligibility', async () => {
      const f = fixture();
      const entry = { systemHint: hint };
      f.props.availableSystemHints.push(entry);
      if (restriction === 'filtered') f.menu.availableSystemHints = f.props.availableSystemHints.slice(0, 2);
      if (restriction === 'disabled') entry.disabled = true;
      if (restriction === 'upsell') entry.isLoggedOutUpsell = true;
      if (restriction === 'hidden') entry.hideFromInitialSelection = true;
      if (restriction === 'local_action') f.menu.resolveSystemHintBehavior = h => h === entry ? { kind: 'local_action' } : undefined;
      f.list(); await flush();
      assert.deepEqual(f.events.at(-1).map(item => item.semantic), ['web_search', 'image_generation']);
      assert.equal(f.calls.length, 0);
    });
  }
  test(hint + ' stale model selection cannot mutate the replacement composer', async () => {
    const f = fixture(); f.props.availableSystemHints.push({ systemHint: hint });
    f.list(); await flush(); const id = f.choice(hint === 'tatertot' ? 'study' : 'canvas').id;
    f.props.currentModelId = 'other-model'; f.pick(id);
    assert.equal(f.results.at(-1)[1], false);
    assert.equal(f.calls.length, 0);
    assert.equal(f.fallbacks, 0);
  });
}

test('one unavailable hint does not remove the other eligible tool', async () => {
  const f = fixture(); f.menu.availableSystemHints = [f.props.availableSystemHints[0]];
  assert.equal(f.list(), true); await flush();
  assert.deepEqual(f.events.at(-1).map(item => item.semantic), ['web_search']);
  f.pick(f.choice('web_search').id); assert.equal(f.results.at(-1)[1], true);
});

test('nested compiled hook caches do not hide the unique tool menu cache', async () => {
  const f = fixture({ current: true });
  f.ancestor.updateQueue.memoCache.data = [Array(12), f.memo, Array(7)];
  assert.equal(f.list(), true); await flush();
  assert.equal(toolContext.state(f.page), 'ready');
  f.pick(f.choice('web_search').id);
  assert.equal(f.state.activeSystemHintType, 'search');
  assert.equal(f.fallbacks, 0);
});

for (const duplicate of [true, false]) {
  test('ambiguous or excessive compiled caches cannot enable a tool: ' + duplicate, () => {
    const f = fixture();
    f.ancestor.updateQueue.memoCache.data = duplicate ? [f.memo, [...f.memo]] : [f.memo, ...Array(64).fill([])];
    assert.equal(f.list(), false);
    assert.equal(toolContext.state(f.page), 'cache_unavailable');
    assert.equal(f.calls.length, 0);
  });
}

test('tool capture diagnostics are read-only and reset across documents', () => {
  const f = fixture();
  assert.equal(toolContext.state(f.page), 'not_observed');
  f.props.currentModelId = null;
  assert.equal(f.list(), false);
  assert.equal(toolContext.state(f.page), 'model_unavailable');
  assert.equal(f.imports, 0);
  assert.equal(f.calls.length, 0);
  f.page.__elonChatGptDocumentToken = 'doc_next_1';
  assert.equal(toolContext.state(f.page), 'not_observed');
  assert.doesNotMatch(toolContext.state(f.page), /Bearer|fixture|00000000/);
});

for (const [name, mutate] of Object.entries({
  'filtered out': f => { f.menu.availableSystemHints = []; },
  'lockdown': f => { f.menu.isConsumerLockdownModeEnabled = true; },
  'loading': f => { f.menu.isLoading = true; },
  'wrong owner version': f => { f.ancestor.type = { name: 'Unknown' }; },
  'missing compiled state': f => { delete f.ancestor.updateQueue; },
  'uncommitted eligibility': f => { f.memo[56] = []; },
  'voice active': f => { f.memo[65] = true; },
  'at tagging disabled': f => { f.memo[63] = true; },
  'local action': f => { f.menu.resolveSystemHintBehavior = () => ({ kind: 'local_action', select() { throw Error('never execute'); } }); }
})) {
  test('actual official menu restriction is retained: ' + name, async () => {
    const f = fixture(); mutate(f);
    assert.equal(f.list(), false); await flush();
    assert.equal(f.calls.length, 0); assert.equal(f.results.length, 0);
  });
}

test('guest login change invalidates an already issued search handle', async () => {
  const f = fixture(); f.page.location.href = 'https://chatgpt.com/';
  f.page.__elonChatGptPrivateTransport = null;
  await f.page.__elonChatGptPrivateRuntimeBindings.load('shared');
  f.list(); await flush(); const id = f.choice('web_search').id;
  f.guest.authStatus = 'logged_in'; f.guest.session = {};
  f.pick(id); assert.equal(f.results.at(-1)[1], false); assert.equal(f.calls.length, 0);
});

test('guest request credentials do not remove the positive guest route proof', async () => {
  const f = fixture({ current: true }); f.page.location.href = 'https://chatgpt.com/';
  await f.page.__elonChatGptPrivateRuntimeBindings.load('shared');
  f.list(); await flush(); f.pick(f.choice('web_search').id);
  assert.equal(f.results.at(-1)[1], true);
  f.guest.authStatus = 'logged_in'; f.guest.session = {};
  assert.equal(f.list(), false);
});

test('a new document invalidates handles and uses a newly resolved module', async () => {
  const f = fixture(); f.list(); await flush();
  const id = f.choice('web_search').id;
  f.page.document = { ...f.page.document }; f.page.__elonChatGptDocumentToken = 'doc_new_document';
  f.pick(id); assert.equal(f.calls.length, 0); assert.equal(f.results.at(-1)[1], false);
  f.list(); await flush(); f.pick(f.choice('web_search').id);
  assert.equal(f.results.at(-1)[1], true); assert.equal(f.imports, 2);
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
    const f = fixture();
    const original = f.namespace.Bg;
    f.namespace.Bg = (...args) => {
      if (mode === 'mutate then throw') original(...args);
      if (mode !== 'ignore') throw new Error('fixture');
    };
    f.list(); await flush();
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
