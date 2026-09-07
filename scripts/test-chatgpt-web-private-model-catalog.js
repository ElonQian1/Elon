'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const catalogModule = require('../android/app/src/main/assets/chatgpt_web_private_model_catalog');
const { fixture } = require('./fixtures/chatgpt-model-state');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');
const flush = async () => { for (let i = 0; i < 60; i++) await Promise.resolve(); };

function setup(count = 2, options = {}) {
  const f = fixture({ extended: true, modelCatalog: catalogModule, ...options });
  for (const model of f.menu.modelsData.models.values()) model.tags = [];
  const ids = Array.from({ length: count }, (_, i) => 'fixture:extra/' + i);
  for (const id of ids) f.menu.modelsData.models.set(id, { id, title: 'Extra ' + id.split('/').at(-1), tags: [] });
  f.menu.modelsData.groups = [{ group: 'alpha', label: 'Alpha', modelIds: ids }];
  let gate = false, approvals = 0, builders = 0;
  f.modules.shared.$3 = name => { assert.equal(name, '3016847258'); return gate; };
  f.modules.conversation.l1t = { ALPHA: 'alpha', EXPERIMENTS: 'experiments', DATA_CAMPAIGNS: 'data_campaigns', MAINLINE: 'mainline' };
  f.modules.conversation.u1t = ({ modelsData: data }) => {
    builders++;
    return data.groups.map(group => ({ categoryId: group.group, name: group.label,
      options: group.modelIds.flatMap(id => {
        const model = data.models.get(id);
        return !model || model.tags.includes('hidden') ? [] : [{ value: id, name: model.title }];
      }) }));
  };
  f.modules.conversation.iin = async id => { approvals++; assert.ok(ids.includes(id)); return true; };
  return Object.assign(f, { ids, getStats: () => ({ approvals, builders }), setGate: value => { gate = value; } });
}
async function open(f = setup()) {
  assert.equal(f.request(), true); await flush();
  f.select(f.choice('高级').id);
  if (f.choice('其他官网模型')) f.select(f.choice('其他官网模型').id);
  return f;
}

test('extra catalog stays native and selection uses the official model action exactly once', async () => {
  const f = await open();
  assert.ok(f.choice('Extra 0')); assert.ok(f.choice('返回高级'));
  assert.equal(f.choice('Extra 0').semantic, 'model_catalog');
  assert.equal(f.advanced, 0); assert.equal(f.writes.length, 0);
  const id = f.choice('Extra 0').id; f.select(id); f.select(id); await flush();
  assert.equal(f.results.at(-1)[1], true); assert.equal(f.model, f.ids[0]);
  assert.equal(f.effort, 'low'); assert.equal(f.version, 'fixture-version');
  assert.deepEqual(f.writes, [['official-model-action', f.ids[0], undefined], ['model', f.ids[0]]]);
  f.select(id); await flush(); assert.equal(f.writes.length, 2);
  assert.equal(f.choice('Extra 0').selected, true); assert.equal(f.imports, 3);
  assert.equal(f.getStats().approvals, 0); assert.equal(f.timers.size, 0);
  assert.doesNotMatch(JSON.stringify([f.events, f.results]), /Bearer|headers|fixture:extra|00000000/);
});

test('large catalogs paginate without silently dropping models or making network requests', async () => {
  const f = await open(setup(47));
  assert.equal(f.events.at(-1).filter(x => !x.opensSubmenu).length, 20);
  assert.ok(f.choice('Extra 0')); assert.ok(f.choice('Extra 19')); assert.equal(f.choice('Extra 20'), undefined);
  f.select(f.choice('下一页').id); assert.ok(f.choice('Extra 20')); assert.ok(f.choice('上一页'));
  f.select(f.choice('下一页').id); assert.ok(f.choice('Extra 46')); assert.equal(f.choice('下一页'), undefined);
  f.select(f.choice('上一页').id); assert.ok(f.choice('Extra 20'));
  f.select(f.choice('返回高级').id); assert.ok(f.choice('Version 2'));
  assert.equal(f.advanced, 0); assert.equal(f.getStats().approvals, 0); assert.equal(f.writes.length, 0);
});

test('only official additional groups and available visible catalog members are offered', async () => {
  const f = setup(5);
  f.menu.modelsData.models.get(f.ids[1]).tags = ['hidden'];
  f.menu.modelSwitcherDenialsBySlug[f.ids[2]] = [{}];
  f.menu.modelsData.models.delete(f.ids[3]);
  f.menu.modelsData.groups.push({ group: 'ordinary', label: 'Not extra', modelIds: ['fixture-fast'] });
  await open(f);
  assert.ok(f.choice('Extra 0')); assert.ok(f.choice('Extra 4'));
  for (const label of ['Extra 1', 'Extra 2', 'Extra 3', 'Not extra']) assert.equal(f.choice(label), undefined);
});

test('authoritative empty catalog hides the redundant entry but unknown schema retains the existing reader', async () => {
  const empty = await open(setup(0));
  assert.equal(empty.advanced, 0); assert.equal(empty.choice('其他官网模型'), undefined);
  assert.ok(empty.choice('Version 2'));
  const unknown = setup(); unknown.modules.conversation.u1t = undefined;
  await open(unknown); assert.equal(unknown.advanced, 1); assert.equal(unknown.writes.length, 0);
});

test('the active picker category options restrict the server catalog and nested groups remain selectable', async () => {
  const f = setup(3);
  f.menu.modelSwitcherCategoryOptions = [{ categoryId: 'alpha', options: [{ options: [
    { value: f.ids[1], name: 'Visible extra' }, { value: 'not-in-catalog', name: 'Unknown' }
  ] }] }];
  await open(f);
  assert.deepEqual(f.events.at(-1).filter(x => !x.opensSubmenu).map(x => x.label), ['Visible extra']);
  f.select(f.choice('Visible extra').id); await flush(); assert.equal(f.model, f.ids[1]);
});

test('cyclic, excessive or malformed model menus fail without guessing capabilities', async () => {
  for (const mutate of [
    f => { const cycle = {}; cycle.options = [cycle]; f.menu.modelSwitcherCategoryOptions = [{ categoryId: 'alpha', options: [cycle] }]; },
    f => { f.menu.modelsData.groups = Array(101).fill(f.menu.modelsData.groups[0]); },
    f => { f.modules.conversation.u1t = () => [{ categoryId: 'alpha', options: [{ name: 'Bad', value: 'invalid?model' }] }]; }
  ]) {
    const f = setup(); mutate(f); await open(f);
    assert.equal(f.advanced, 1); assert.equal(f.writes.length, 0);
  }
});

test('current profile aliases retain official catalog and privacy eligibility checker identities', async () => {
  const f = setup(), binding = attach(f.page, f.modules);
  f.page.__elonChatGptPrivateModelContract = require('../android/app/src/main/assets/chatgpt_web_private_model_contract');
  const actual = require('../android/app/src/main/assets/chatgpt_web_private_model_state').create(f.page);
  assert.ok(actual.request(f.getTrigger, rows => f.events.push(rows), (...row) => f.results.push(row), () => assert.fail('DOM')));
  await flush();
  const select = label => actual.select(f.choice(label).id, (...row) => f.results.push(row), () => {}, () => assert.fail('DOM'));
  select('高级'); select('其他官网模型'); f.setGate(true); select('Extra 1'); await flush();
  assert.equal(f.model, f.ids[1]); assert.equal(f.getStats().approvals, 1); assert.equal(binding.loads.length, 3);
});

for (const [name, mutate] of Object.entries({
  account: f => { f.account = 'Bearer different-fixture-account'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; },
  route: f => { f.page.location.href = 'https://chatgpt.com/'; },
  conversation: f => { f.menu.conversation = { serverId$: () => f.conversation.serverId$() }; },
  model: f => { f.model = 'fixture-thinking'; },
  effort: f => { f.effort = 'max'; },
  tier: f => { f.serviceTier = 'fast'; },
  version: f => { f.version = 'fixture-version-v2'; },
  work: f => { f.work = true; },
  permission: f => { f.menu.modelSwitcherDenialsBySlug[f.ids[0]] = [{}]; },
  hidden: f => { f.menu.modelsData.models.get(f.ids[0]).tags = ['hidden']; },
  detached: f => { f.node.isConnected = false; }
})) test('reject changed ' + name + ' after the asynchronous privacy check', async () => {
  const f = setup(); f.setGate(true); let approve;
  f.modules.conversation.iin = () => new Promise(resolve => { approve = resolve; });
  await open(f); f.select(f.choice('Extra 0').id); await flush();
  mutate(f); approve(true); await flush();
  assert.equal(f.writes.length, 0); assert.equal(f.results.at(-1)[1], false); assert.equal(f.advanced, 0);
});

test('privacy rejection and timeout never turn into a model write or a DOM replay', async () => {
  for (const result of [false, undefined, 'true', new Error('fixture failure'), 'timeout']) {
    const f = setup(); f.setGate(true); let complete;
    f.modules.conversation.iin = () => result === 'timeout' ? new Promise(resolve => { complete = resolve; }) :
      result instanceof Error ? Promise.reject(result) : Promise.resolve(result);
    await open(f); f.select(f.choice('Extra 0').id); await flush();
    if (result === 'timeout') { for (const timer of [...f.timers]) timer(); await flush(); complete(true); await flush(); }
    assert.equal(f.writes.length, 0); assert.equal(f.results.at(-1)[1], false);
    assert.equal(f.advanced, 0); assert.equal(f.timers.size, 0);
  }
});

test('closing or replacing the menu cancels an in-flight selection before it can mutate', async () => {
  for (const action of [f => f.runtime.dismiss(), f => f.request(), f => f.select(f.choice('返回高级').id)]) {
    const f = setup(); f.setGate(true); let approve;
    f.modules.conversation.iin = () => new Promise(resolve => { approve = resolve; });
    await open(f); f.select(f.choice('Extra 0').id); await flush();
    action(f); const count = f.results.length; approve(true); await flush();
    assert.equal(f.writes.length, 0); assert.equal(f.results.length, count); assert.equal(f.timers.size, 0);
  }
});

test('a newer choice replaces the old pending choice without delayed write replay', async () => {
  const f = setup(); f.setGate(true); const pending = [];
  f.modules.conversation.iin = () => new Promise(resolve => pending.push(resolve));
  await open(f); f.select(f.choice('Extra 0').id); f.select(f.choice('Extra 1').id); await flush();
  pending[0](true); await flush(); assert.equal(f.writes.length, 0);
  pending[1](true); await flush(); assert.equal(f.model, f.ids[1]); assert.equal(f.writes.length, 2);
});

test('a possibly applied official mutation failure is never retried through another transport', async () => {
  const f = await open();
  f.modules.composer.Ih = command => { f.model = command.modelId; throw Error('after write'); };
  f.select(f.choice('Extra 0').id); await flush();
  assert.equal(f.model, f.ids[0]); assert.equal(f.results.at(-1)[1], false); assert.equal(f.advanced, 0);
});

test('catalog assembly precedes the production model controller', () => {
  const source = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  assert.ok(source.indexOf('"chatgpt_web_private_model_catalog.js"') < source.indexOf('"chatgpt_web_private_model_state.js"'));
  const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
  const assets = path.join(__dirname, '../android/app/src/main/assets');
  const names = [...source.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+\.js)"/g)].map(m => m[1]);
  for (const name of names) new vm.Script(fs.readFileSync(path.join(assets, name), 'utf8'), { filename: name });
  const semantics = fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebComposerOptionSemantics.kt'), 'utf8');
  assert.match(semantics, /const val MODEL_CATALOG = "model_catalog"/);
  assert.match(semantics.split('val KNOWN = setOf(')[1], /MODEL_CATALOG,/);
});

test('production composer request, paging, private selection and dismiss stay on native controls', async () => {
  const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
  const assets = path.join(__dirname, '../android/app/src/main/assets');
  const f = setup(24), events = [], results = [], page = f.page;
  page.document.querySelector = selector => selector === '[data-testid="model-switcher"]' ? f.node : null;
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
  const select = label => composer.selectOption('model', events.at(-1).options.find(x => x.label === label).id,
    input, emit, result, () => {});
  composer.requestOptions('model', input, emit, result); await flush();
  select('高级'); select('其他官网模型'); select('下一页'); select('Extra 23'); await flush();
  assert.equal(f.model, f.ids[23]); assert.equal(results.at(-1)[1], true);
  assert.ok(events.at(-1).options.some(x => x.label === 'Extra 23' && x.selected));
  assert.ok(events.every(x => x.type !== 'web_touch_request'));
  assert.ok(events.every(x => !x.options || x.options.length <= 30));
  composer.dismissOpenMenu(input, emit, result); assert.equal(results.at(-1)[0], 'dismiss_composer_menu');
});
