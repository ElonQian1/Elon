'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const runtimeModule = require(path.join(assets, 'chatgpt_web_private_temporary_chat.js'));
const pagePolicy = require(path.join(assets, 'chatgpt_web_adapter_page_semantic_policy.js'));
const CID = '00000000-0000-0000-0000-000000000001';
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
// The inspected AKt action, with synthetic dependencies below; no real account or network.
const actionSource = '()=>{cg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(gB.reset(c),qg()&&$p.delete(n),!o&&!$p(n)&&OKt(s),u(jKt,{replace:!0})):oD(l,{params:o?void 0:new URLSearchParams({[zm]:`true`})})}';

function fixture(options = {}) {
  let time = 0, nextTimer = 1, imports = 0, fallbacks = 0, snapshots = 0, observed = true;
  let account = 'Bearer synthetic-test-not-a-credential';
  const timers = new Map(), results = [], effects = [];
  const state = { newChat: options.newChat !== false, selected: options.selected === true,
    work: false, mode: options.mode || 'normal', privacy: options.selected === true,
    files: ['synthetic-file'], personalized: true, tools: ['synthetic-personalized-tool'] };
  const page = { location: { href: 'https://chatgpt.com/' + (state.newChat ? '' : 'c/' + CID) +
    (state.selected ? '?temporary-chat=true' : '') }, __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    performance: { getEntriesByName: () => observed ? [{}] : [] },
    document: { querySelector: () => null, querySelectorAll: () => [state.node] },
    setTimeout(fn, ms) { const id = nextTimer++; timers.set(id, { fn, at: time + ms }); return id; },
    clearTimeout(id) { timers.delete(id); } };
  function render(newConversation = false) {
    if (newConversation || !state.conversation) {
      if (state.node) state.node.isConnected = false;
      const id = state.newChat ? 'client-created-fixture-' + (newConversation ? '2' : '1') : CID;
      const serverId = state.newChat ? null : CID;
      state.conversation = { id, serverId$: () => serverId, config: {} };
    }
    const id = state.conversation.id;
    const commit = selected => {
      if (state.mode === 'ignore') return;
      const url = new URL(page.location.href); url.search = selected ? '?temporary-chat=true' : '';
      page.location.href = url.href; state.selected = selected;
      if (state.mode === 'normal' || state.mode === 'throw-after-success') state.privacy = selected;
      render();
      if (state.mode.startsWith('throw')) throw new Error('synthetic-handler-failure');
    };
    const personalize = () => state.personalized;
    personalize.delete = value => { assert.equal(value, id); state.personalized = false; effects.push('personalization-clear'); };
    const action = new Function('cg', 'a', 'o', 'gB', 'c', 'qg', '$p', 'n', 'OKt', 's', 'u', 'jKt', 'oD', 'l', 'zm',
      'return ' + actionSource)(
      { logEvent: () => effects.push('official-action') }, state.newChat, state.selected,
      { reset: () => { state.files = []; effects.push('attachments-reset'); } }, {}, () => true, personalize, id,
      () => { state.tools = []; effects.push('personalized-tools-reset'); }, {},
      (updater, config) => {
        assert.equal(config.replace, true); effects.push('router-replace');
        const query = updater(new URL(page.location.href).searchParams);
        commit(query.get('temporary-chat') === 'true');
      },
      query => { query.has('temporary-chat') ? query.delete('temporary-chat') : query.set('temporary-chat', 'true'); return query; },
      (_router, config) => {
        effects.push('new-conversation');
        if (state.mode === 'ignore') return;
        state.newChat = true;
        page.location.href = 'https://chatgpt.com/';
        render(true); commit(config.params?.get('temporary-chat') === 'true');
      }, {}, 'temporary-chat');
    const root = { stateNode: {} }; root.stateNode.current = root;
    const conversationFiber = { memoizedProps: { conversation: state.conversation }, return: root };
    const owner = { type: function AKt() {}, memoizedProps: { clientThreadId: id }, return: conversationFiber };
    const memo = Array(30);
    memo[0] = id; memo[3] = state.newChat; memo[4] = state.selected; memo[7] = action;
    memo[19] = action; memo[20] = state.selected && !state.newChat; memo[21] = state.selected;
    owner.updateQueue = { memoCache: { data: [memo], index: 1 } };
    const button = { memoizedProps: { onClick: state.selected && !state.newChat ? undefined : action }, return: owner };
    if (!state.node || !state.node.isConnected) {
      state.node = { isConnected: true, disabled: false, getAttribute: () => null,
        click: () => effects.push('dom-click'), getBoundingClientRect: () => ({ left: 0, top: 0, width: 40, height: 40 }) };
    }
    state.node.__reactFiber$fixture = { return: button };
    state.root = root; state.owner = owner; state.button = button;
  }
  render();
  const modules = { cX: () => new URL(page.location.href).searchParams.get('temporary-chat') === 'true',
    XM: id => id === state.conversation.id ? { is_do_not_remember: state.privacy, isNew: state.newChat } : null,
    uo: () => state.work, HM: { getIsNewConversation: thread => thread.isNew } };
  const runtime = runtimeModule.create(page, { now: () => time, loadRuntime: async url => {
    imports++; assert.equal(url, 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js');
    return options.loadRuntime ? options.loadRuntime(modules) : modules;
  } });
  const values = desiredSelected => ({ node: state.node, desiredSelected,
    control: { semantic: 'temporary_chat', selected: false }, controlId: 'control_temporary_chat',
    pageSemanticPolicy: pagePolicy, isVisible: () => true, isInViewport: () => true,
    result: (...row) => results.push(row), emitEvent: () => effects.push('touch'), emitSnapshot: () => { snapshots++; } });
  const select = desired => runtime.setSelected(values(desired), () => { fallbacks++; });
  function advance(ms) {
    const target = time + ms;
    for (let count = 0; count < 100; count++) {
      const entry = [...timers].filter(([, value]) => value.at <= target).sort((a, b) => a[1].at - b[1].at)[0];
      if (!entry) break;
      time = entry[1].at; timers.delete(entry[0]); entry[1].fn();
    }
    time = target;
  }
  return { page, state, runtime, modules, timers, results, effects, render, values, select, advance,
    get imports() { return imports; }, get fallbacks() { return fallbacks; }, get snapshots() { return snapshots; },
    set observed(value) { observed = value; }, set account(value) { account = value; } };
}

test('empty chat uses the official transaction, preserves cleanup and reuses the warm runtime', async () => {
  const f = fixture();
  assert.equal(f.select(true), true); await flush();
  assert.equal(f.results.at(-1)[1], true); assert.equal(f.state.privacy, true);
  assert.deepEqual(f.effects, ['official-action', 'attachments-reset', 'personalization-clear', 'personalized-tools-reset', 'router-replace']);
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: true, stateSettable: true });
  f.select(false);
  assert.equal(f.results.at(-1)[1], true); assert.equal(f.state.privacy, false); assert.equal(f.imports, 1);
  assert.equal(f.fallbacks, 0); assert.equal(f.timers.size, 0);
});

test('an existing regular conversation starts a new temporary conversation, not a privacy rewrite', async () => {
  const f = fixture({ newChat: false }), original = f.state.conversation;
  f.select(true); await flush();
  assert.equal(f.results.at(-1)[1], true); assert.notEqual(f.state.conversation, original);
  assert.equal(new URL(f.page.location.href).pathname, '/');
  assert.deepEqual(f.effects, ['official-action', 'new-conversation']);
});

test('readonly existing temporary chat is observed correctly and is not converted to a normal saved chat', async () => {
  const f = fixture({ newChat: false, selected: true });
  f.runtime.observe(f.state.node); await flush();
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: true, stateSettable: false });
  f.select(false); assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0);
  f.select(true); assert.equal(f.results.at(-1)[1], true); assert.equal(f.effects.length, 0);
});

test('stale native selected state cannot toggle twice', async () => {
  const f = fixture(); f.select(true); await flush();
  f.select(true); assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.effects.filter(item => item === 'official-action').length, 1);
});

test('route change is not success until official thread privacy also agrees', async () => {
  const f = fixture({ mode: 'deferred' }); f.select(true); await flush();
  assert.equal(f.results.length, 0);
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: false, stateSettable: false });
  f.select(true); assert.equal(f.results.length, 0);
  f.state.privacy = true; f.advance(100);
  assert.equal(f.results.length, 2); assert.ok(f.results.every(row => row[1]));
  assert.equal(f.effects.filter(item => item === 'official-action').length, 1);
  assert.equal(f.timers.size, 0);
});

test('opposite intent during a transition cannot silently undo it', async () => {
  const f = fixture({ mode: 'deferred' }); f.select(true); await flush();
  f.select(false); assert.equal(f.results.at(-1)[1], false);
  f.state.privacy = true; f.advance(100); assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.effects.filter(item => item === 'official-action').length, 1);
});

for (const mode of ['ignore', 'deferred', 'throw-after-route']) test(mode + ' times out without DOM replay or false confirmation', async () => {
  const f = fixture({ mode }); f.select(true); await flush(); f.advance(2500);
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.fallbacks, 0); assert.equal(f.timers.size, 0);
  assert.equal(f.effects.filter(item => item === 'official-action').length, 1);
  assert.equal(f.runtime.observe(f.state.node)?.selected, false);
  if (mode !== 'ignore') {
    f.state.privacy = true;
    assert.deepEqual(f.runtime.observe(f.state.node), { selected: true, stateSettable: true });
  }
});

test('throwing after a fully applied action may confirm, but never repeats', async () => {
  const f = fixture({ mode: 'throw-after-success' }); f.select(true); await flush();
  assert.equal(f.results.at(-1)[1], true); assert.equal(f.fallbacks, 0);
});

for (const [name, mutate] of Object.entries({
  account: f => { f.account = 'Bearer other-synthetic-account'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
  route: f => { f.page.location.href = 'https://chatgpt.com/c/' + CID; },
  detached: f => { f.state.node.isConnected = false; },
  owner: f => { f.state.owner.memoizedProps.clientThreadId = 'other'; },
  action: f => { f.state.button.memoizedProps.onClick = () => {}; },
  commit: f => { f.state.root.stateNode.current = {}; }
})) test('changed ' + name + ' during import cannot activate in a different context', async () => {
  let complete;
  const f = fixture({ loadRuntime: modules => new Promise(resolve => { complete = () => resolve(modules); }) });
  f.select(true); await flush(); mutate(f); complete(); await flush();
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0); assert.equal(f.fallbacks, 0);
});

test('navigation and account change after dispatch cancel bounded observation', async () => {
  for (const kind of ['route', 'account']) {
    const f = fixture({ mode: 'deferred' }); f.select(true); await flush();
    if (kind === 'route') f.page.location.href = 'https://chatgpt.com/c/' + CID;
    else f.account = 'Bearer other-synthetic-account';
    f.advance(100); assert.equal(f.results.at(-1)[1], false); assert.equal(f.timers.size, 0);
  }
});

test('known runtime waits for initialized privacy instead of toggling stale DOM state', async () => {
  const f = fixture(); f.state.privacy = true; f.select(true); await flush();
  assert.equal(f.effects.length, 0); assert.equal(f.fallbacks, 0);
  f.advance(1600); assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0);
});

test('stale empty-chat closure waits for a fresh render before creating a new temporary conversation', async () => {
  const f = fixture(); f.state.newChat = false;
  f.select(true); await flush();
  assert.equal(f.effects.length, 0); assert.equal(f.fallbacks, 0);
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: false, stateSettable: false });
  f.render(); f.advance(100);
  assert.equal(f.results.at(-1)[1], true);
  assert.deepEqual(f.effects, ['official-action', 'new-conversation']);
  assert.equal(f.timers.size, 0);
});

test('stale temporary-state closure cannot turn an existing temporary chat into a saved chat', async () => {
  const f = fixture({ newChat: false });
  f.page.location.href += '?temporary-chat=true'; f.state.selected = f.state.privacy = true;
  f.select(false); await flush(); assert.equal(f.effects.length, 0);
  f.render(); f.advance(100);
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0); assert.equal(f.fallbacks, 0);
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: true, stateSettable: false });
});

test('a stale compiler closure times out without any private or DOM mutation', async () => {
  const f = fixture(); f.state.newChat = false;
  f.select(true); await flush(); f.advance(1600);
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0); assert.equal(f.fallbacks, 0);
  assert.equal(f.timers.size, 0);
});

test('unknown compiler cache schema or mismatched closure cannot claim a private transition', () => {
  for (const mutate of [
    f => { f.state.owner.updateQueue = {}; },
    f => { f.state.owner.updateQueue.memoCache.data.push(Array(30)); },
    f => { f.state.owner.updateQueue.memoCache.data[0].push(null); },
    f => { f.state.owner.updateQueue.memoCache.data[0][0] = 'other'; },
    f => { f.state.owner.updateQueue.memoCache.data[0][3] = undefined; },
    f => { f.state.owner.updateQueue.memoCache.data[0][4] = undefined; },
    f => { f.state.owner.updateQueue.memoCache.data[0][19] = () => {}; },
    f => { f.state.owner.updateQueue.memoCache.data[0][20] = true; },
    f => { f.state.owner.updateQueue.memoCache.data[0][21] = true; },
    f => { f.state.button.memoizedProps.onClick = new Function('return ' + actionSource)(); },
    f => { f.state.button.memoizedProps.onClick = undefined; }
  ]) {
    const f = fixture(); mutate(f);
    assert.equal(f.select(true), false); assert.equal(f.imports, 0); assert.equal(f.effects.length, 0);
  }
});

test('unknown asset and module schema retain the existing path with bounded loading', async () => {
  const missing = fixture(); missing.observed = false;
  assert.equal(missing.select(true), false); assert.equal(missing.imports, 0);
  for (const mode of ['schema', 'timeout']) {
    const f = fixture({ loadRuntime: () => mode === 'schema' ? {} : new Promise(() => {}) });
    f.select(true); await flush(); f.advance(1600); await flush();
    assert.equal(f.fallbacks, 1); assert.equal(f.effects.length, 0); assert.equal(f.timers.size, 0);
    assert.equal(f.select(true), false); assert.equal(f.imports, 1);
  }
});

test('the compiler-cache bridge requires the inspected React runtime to be loaded too', () => {
  const f = fixture();
  f.page.performance.getEntriesByName = url => url.includes('2340486e-') ? [] : [{}];
  assert.equal(f.select(true), false); assert.equal(f.imports, 0); assert.equal(f.effects.length, 0);
});

test('unknown callback, guest identity and unsupported routes do not claim a private transition', () => {
  for (const mutate of [
    f => { f.state.button.memoizedProps.onClick = () => {}; },
    f => { f.account = ''; },
    f => { f.page.location.href = 'https://chatgpt.com/work'; },
    f => { f.page.location.href = 'https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/project'; },
    f => { f.page.location.href += '?model=fixture'; },
    f => { f.page.__elonChatGptDocumentToken = ''; }
  ]) {
    const f = fixture(); mutate(f);
    assert.equal(f.select(true), false); assert.equal(f.imports, 0); assert.equal(f.effects.length, 0);
  }
});

test('work-mode policy stays on the existing flow without private mutations', async () => {
  const f = fixture(); f.state.work = true; f.select(true); await flush();
  assert.equal(f.fallbacks, 1); assert.equal(f.effects.length, 0); assert.equal(f.timers.size, 0);
});

test('private observation accepts a committed alternate and never reactivates a disabled control', async () => {
  const f = fixture();
  const host = f.state.node.__reactFiber$fixture;
  f.state.node.__reactFiber$fixture = { return: { stateNode: f.state.root.stateNode }, alternate: host };
  f.state.node.disabled = true;
  f.select(true); await flush();
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.effects.length, 0);
  assert.deepEqual(f.runtime.observe(f.state.node), { selected: false, stateSettable: false });
});

test('production temporary adapter uses runtime selection and authoritative state without touch or click', async () => {
  const f = fixture(); f.page.__elonChatGptPrivateTemporaryChat = { create: () => f.runtime };
  const context = { window: f.page };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_temporary_chat.js'), 'utf8'), context);
  const adapter = f.page.__elonChatGptTemporaryChat;
  adapter.setSelected(f.values(true)); await flush();
  assert.equal(f.results.at(-1)[1], true);
  const descriptor = adapter.describe(pagePolicy, { signal: 'Temporary chat', node: f.state.node });
  assert.equal(descriptor.selected, true);
  assert.equal(f.effects.includes('dom-click'), false); assert.equal(f.effects.includes('touch'), false);
  assert.doesNotMatch(JSON.stringify([f.results, descriptor]), /Bearer|00000000|client-created|headers|token/);
  const layout = fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_layout.js'), 'utf8');
  assert.match(layout, /temporaryChatAdapter\.describe\(window\.__elonChatGptPageSemanticPolicy,\s*\{\s*node,/);
  const kotlin = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  assert.ok(kotlin.indexOf('"chatgpt_web_private_temporary_chat.js"') < kotlin.indexOf('"chatgpt_web_adapter_temporary_chat.js"'));
});
