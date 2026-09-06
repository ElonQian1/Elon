'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const contractModule = require(path.join(assets, 'chatgpt_web_private_model_contract.js'));
const runtimeModule = require(path.join(assets, 'chatgpt_web_private_model_state.js'));
const CID = '00000000-0000-0000-0000-000000000001';
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

function fixture(options = {}) {
  const writes = [], events = [], results = [], timers = new Set();
  let model = 'fixture-fast', effort = 'low', tier, account = 'Bearer fixture-not-a-credential', loaded = true, work = false;
  const conversation = { serverId$: () => CID };
  const fast = { id: 'fixture-fast' }, thinking = { id: 'fixture-thinking' };
  const menu = { conversation, modelSwitcherDenialsBySlug: {},
    modelsData: { models: new Map([[fast.id, fast], [thinking.id, thinking]]) },
    composerIntelligencePickerState: {
      selectedVersionEntry: { id: 'fixture-version' },
      bucketSelections: [
        { bucket: 0, title: 'Low', availability: { status: 'available' }, modelSlug: fast.id,
          modelConfig: fast, thinkingEffort: 'low', category: { modelLane: 'instant' } },
        { bucket: 1, title: 'High', availability: { status: 'available' }, modelSlug: thinking.id,
          modelConfig: thinking, thinkingEffort: 'high', category: { modelLane: 'thinking' } },
        { bucket: 2, title: 'Extra High', availability: { status: 'available' }, modelSlug: thinking.id,
          modelConfig: thinking, thinkingEffort: 'max', category: { modelLane: 'pro' } }
      ]
    } };
  const props = { ariaDisabled: false, dropdownOpen: false, dropdownContent: { props: menu } };
  const top = { stateNode: {} }; top.stateNode.current = top;
  const ancestor = { memoizedProps: props, return: top }, host = { return: ancestor };
  const node = { isConnected: true, __reactFiber$fixture: host,
    getAttribute: name => name === 'aria-expanded' ? String(props.dropdownOpen) : null,
    getBoundingClientRect: () => ({ width: 80, height: 40, left: 0, top: 0 }) };
  const store = { conversationThinkingEffort$: () => effort,
    setThinkingEffort(value, slug) { writes.push(['effort', value, slug]); effort = value; } };
  const modules = {
    shared: { M$: fn => fn(), uo: () => work, t4: { PRO: 'pro' },
      RW: () => ({ mutate: data => writes.push(['preference', data]) }) },
    conversation: {
      Nrn: () => ({ id: model }), yRt: () => store, l0: () => ({ getDraftServiceTier: () => tier }),
      Grn(conv, value) { assert.equal(conv, conversation); writes.push(['model', value]); model = value; },
      vRt: () => ['low', 'high', 'max'],
      p8t: ({ modelSlug, modelSwitcherDenialsBySlug: denials }) => ({ status: denials[modelSlug] ? 'unavailable' : 'available' }),
      M1t: () => ({ mutate: data => writes.push(['pro-preference', data]) }),
      Rdn: { CHATGPT_MODEL_PICKER_SURFACE_COMPOSER: 'CHATGPT_MODEL_PICKER_SURFACE_COMPOSER' }
    },
    composer: { Ih(command) {
      writes.push(['official-model-action', command.modelId, command.thinkingEffort]);
      command.applyModelSelection(command.conversation, command.modelId);
    } }
  };
  const page = { location: { href: 'https://chatgpt.com/c/' + CID },
    __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    document: { querySelector: () => null }, performance: { getEntriesByName: () => loaded ? [{}] : [] },
    setTimeout(fn) { timers.add(fn); return fn; }, clearTimeout(fn) { timers.delete(fn); } };
  const contract = contractModule.create(page), getTrigger = () => node;
  let imports = 0, fallback = 0, snapshots = 0, advanced = 0;
  const runtime = runtimeModule.create(page, { contract: contractModule, loadRuntime: async url => {
    imports++;
    const key = Object.keys(contract.urls).find(key => contract.urls[key] === url);
    assert.ok(key, 'no guessed runtime URL');
    return options.loadRuntime ? options.loadRuntime(key, modules[key]) : modules[key];
  } });
  const result = (...row) => results.push(row), emit = value => events.push(value);
  const request = () => runtime.request(getTrigger, emit, result, () => { fallback++; });
  const select = id => runtime.select(id, result, () => { snapshots++; }, () => { advanced++; });
  return { page, node, host, top, ancestor, props, menu, modules, writes, events, results, timers,
    conversation, contract, runtime, getTrigger, request, select,
    choice: label => events.at(-1).find(item => item.label === label),
    get model() { return model; }, set model(v) { model = v; },
    get effort() { return effort; }, set effort(v) { effort = v; },
    set tier(v) { tier = v; }, set account(v) { account = v; }, set loaded(v) { loaded = v; }, set work(v) { work = v; },
    get imports() { return imports; }, get fallback() { return fallback; },
    get advanced() { return advanced; }, get snapshots() { return snapshots; } };
}

test('model and effort use official stores, fresh handles and no extra warm import', async () => {
  const f = fixture(); assert.equal(f.request(), true); await flush();
  assert.equal(f.imports, 3);
  const option = f.choice('High'); f.select(option.id);
  assert.equal(f.model, 'fixture-thinking'); assert.equal(f.effort, 'high');
  assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.choice('High').selected, true);
  assert.deepEqual(f.writes, [['effort', 'high', 'fixture-thinking'],
    ['official-model-action', 'fixture-thinking', 'high'], ['model', 'fixture-thinking']]);
  const count = f.writes.length; f.select(option.id);
  assert.equal(f.writes.length, count); assert.equal(f.results.at(-1)[1], true);
  f.request(); assert.equal(f.imports, 3); assert.equal(f.timers.size, 0);
  assert.doesNotMatch(JSON.stringify([f.events, f.results]), /Bearer|conversation|headers|fixture-thinking|00000000/);
  for (const item of f.events.flat()) assert.match(item.id, /^[a-z][a-z0-9_]{1,63}$/);
});

test('same-model effort change retains official preference update and Pro default behavior', async () => {
  const f = fixture(); f.model = 'fixture-thinking'; f.effort = 'high'; f.tier = 'fixture-tier';
  f.request(); await flush(); f.select(f.choice('Extra High').id);
  assert.equal(f.results.at(-1)[1], true);
  assert.deepEqual(f.writes, [
    ['pro-preference', { juices: { pro: 'max' } }], ['effort', 'max', 'fixture-thinking'],
    ['model', 'fixture-thinking'], ['preference', { modelSlug: 'fixture-thinking', thinkingEffort: 'max' }]
  ]);
});

for (const [name, mutate] of Object.entries({
  route: f => { f.page.location.href = 'https://chatgpt.com/'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
  account: f => { f.account = 'Bearer different-fixture-account'; },
  conversation: f => { f.menu.conversation = { serverId$: () => CID }; },
  detached: f => { f.node.isConnected = false; },
  uncommitted: f => { f.top.stateNode.current = {}; },
  disabled: f => { f.props.ariaDisabled = true; },
  version: f => { f.menu.composerIntelligencePickerState.selectedVersionEntry.id = 'other-version'; },
  permission: f => { f.menu.modelSwitcherDenialsBySlug['fixture-thinking'] = [{}]; },
  effort: f => { f.modules.conversation.vRt = () => ['low']; },
  concurrent: f => { f.effort = 'max'; },
  tier: f => { f.tier = 'changed-by-user'; },
  work: f => { f.work = true; }
})) test('reject changed ' + name + ' before private mutation', async () => {
  const f = fixture(); f.request(); await flush(); const id = f.choice('High').id;
  mutate(f); f.select(id);
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.writes.length, 0); assert.equal(f.fallback, 0);
});

test('committed alternate resolves the live picker without invoking React hooks', async () => {
  const f = fixture();
  const old = { memoizedProps: { ...f.props, ariaDisabled: true }, return: { stateNode: f.top.stateNode } };
  f.node.__reactFiber$fixture = { return: old, alternate: f.host };
  f.request(); await flush(); assert.equal(f.results.at(-1)[1], true);
});

for (const [name, pathname, serverId] of [
  ['new chat', '/', null],
  ['temporary chat', '/?temporary-chat=true', null],
  ['project chat', '/g/g-p-' + 'a'.repeat(32) + '-fixture/c/' + CID, CID],
  ['project new chat', '/g/g-p-' + 'a'.repeat(32) + '-fixture/project', null]
]) test(name + ' binds the current official conversation before selection', async () => {
  const f = fixture();
  f.page.location.href = 'https://chatgpt.com' + pathname;
  f.conversation.serverId$ = () => serverId;
  assert.equal(f.request(), true); await flush();
  f.select(f.choice('High').id);
  assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.model, 'fixture-thinking');
});

test('mismatched route, unsupported contexts and ambiguous live pickers stay on the existing path', () => {
  for (const mutate of [
    f => { f.page.location.href = 'https://chatgpt.com/'; },
    f => { f.page.location.href += '?model=fixture'; },
    f => { f.page.location.href = 'https://chatgpt.com/g/g-fixture'; },
    f => { f.page.location.href = 'https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/project?temporary-chat=true'; },
    f => { f.ancestor.return = { memoizedProps: { ...f.props, dropdownContent: { props: { ...f.menu } } }, return: f.top }; }
  ]) {
    const f = fixture(); mutate(f);
    assert.equal(f.request(), false); assert.equal(f.imports, 0); assert.equal(f.writes.length, 0);
  }
});

test('overlapping catalog requests share one import and only the newest publishes', async () => {
  const completions = [];
  const f = fixture({ loadRuntime: (_key, value) => new Promise(resolve => completions.push(() => resolve(value))) });
  f.request(); f.request(); await flush();
  assert.equal(f.imports, 3); assert.equal(f.results.length, 1); assert.equal(f.results[0][1], false);
  completions.forEach(complete => complete()); await flush();
  assert.equal(f.events.length, 1); assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.timers.size, 0);
});

test('advanced models keep the existing native menu path without selecting a model', async () => {
  const f = fixture(); f.request(); await flush();
  f.select(f.events.at(-1).find(item => item.opensSubmenu).id);
  assert.equal(f.advanced, 1); assert.equal(f.writes.length, 0);
  assert.equal(f.runtime.dismiss(), false);
});

test('missing modules and restricted picker remain unknown instead of an unsupported error', async () => {
  const f = fixture(); f.loaded = false;
  assert.equal(f.request(), false); assert.equal(f.results.length, 0);
  f.loaded = true; f.menu.composerIntelligencePickerState.bucketSelections = null;
  f.request(); await flush(); assert.equal(f.fallback, 1); assert.equal(f.writes.length, 0);
});

test('schema mismatch and timeout do not import again during cooldown', async () => {
  for (const mode of ['schema', 'timeout']) {
    const f = fixture({ loadRuntime: () => mode === 'schema' ? {} : new Promise(() => {}) });
    f.request(); await flush();
    if (mode === 'timeout') { [...f.timers][0](); await flush(); }
    assert.equal(f.fallback, 1); assert.equal(f.request(), false); assert.equal(f.imports, 3);
    assert.equal(f.timers.size, 0); assert.equal(f.writes.length, 0);
  }
});

test('dismiss and context change during loading cannot open a menu in a different chat', async () => {
  for (const mode of ['dismiss', 'route']) {
    const completions = [];
    const f = fixture({ loadRuntime: (_key, value) => new Promise(resolve => completions.push(() => resolve(value))) });
    f.request(); await flush();
    if (mode === 'dismiss') f.runtime.dismiss(); else f.page.location.href = 'https://chatgpt.com/';
    completions.forEach(complete => complete()); await flush();
    assert.equal(f.events.length, 0); assert.equal(f.fallback, 0); assert.equal(f.results.at(-1)[1], false);
  }
});

for (const mode of ['ignore', 'throw', 'mutate then throw', 'tier mutation']) {
  test('uncertain ' + mode + ' is not replayed through a webpage click', async () => {
    const f = fixture(); f.request(); await flush(); const original = f.modules.composer.Ih;
    f.modules.composer.Ih = command => {
      if (mode.startsWith('mutate') || mode === 'tier mutation') original(command);
      if (mode === 'tier mutation') f.tier = 'unexpected';
      else if (mode !== 'ignore') throw new Error('fixture');
    };
    f.select(f.choice('High').id);
    assert.equal(f.results.at(-1)[1], false); assert.equal(f.fallback, 0); assert.equal(f.advanced, 0);
  });
}

test('blocked presets are never writable and allowed presets remain available', async () => {
  const f = fixture(); f.menu.composerIntelligencePickerState.bucketSelections[1].availability.status = 'rate_limited';
  f.request(); await flush(); assert.equal(f.choice('High'), undefined);
  assert.ok(f.choice('Low')); assert.ok(f.events.at(-1).some(item => item.opensSubmenu));
});

test('production model request/select/dismiss is registered and does not emit webpage touches', async () => {
  const f = fixture(), events = [], results = [], page = f.page;
  page.document.querySelector = name => name === '[data-testid="model-switcher"]' ? f.node : null;
  page.document.querySelectorAll = () => [];
  page.getComputedStyle = () => ({ display: 'block', visibility: 'visible' });
  page.__elonChatGptDictationActions = { create: () => ({}) };
  page.__elonChatGptComposerSubmenu = { createRecovery: () => ({}) };
  page.__elonChatGptComposerDismissPolicy = require(path.join(assets, 'chatgpt_web_adapter_composer_dismiss_policy.js'));
  page.__elonChatGptPrivateModelState = { create: () => f.runtime };
  const input = { closest: () => page.document }, emit = event => events.push(event), result = (...row) => results.push(row);
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_composer.js'), 'utf8'), {
    window: page, document: page.document, location: { origin: 'https://chatgpt.com' }
  });
  const composer = page.__elonChatGptComposer;
  composer.requestOptions('model', input, emit, result); await flush();
  composer.selectOption('model', events.at(-1).options.find(item => item.label === 'High').id, input, emit, result, () => {});
  assert.equal(f.model, 'fixture-thinking'); assert.equal(f.effort, 'high');
  assert.equal(results.at(-1)[1], true);
  composer.dismissOpenMenu(input, emit, result);
  assert.equal(results.at(-1)[0], 'dismiss_composer_menu');
  assert.equal(events.some(event => event.type === 'web_touch_request'), false);
  const adapter = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  assert.ok(adapter.indexOf('"chatgpt_web_private_model_contract.js"') < adapter.indexOf('"chatgpt_web_private_model_state.js"'));
  assert.ok(adapter.indexOf('"chatgpt_web_private_model_state.js"') < adapter.indexOf('"chatgpt_web_adapter_composer.js"'));
});
