'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture } = require('./fixtures/chatgpt-model-state');
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

function restrictedFixture() {
  const f = fixture({ extended: true });
  f.model = 'fixture-thinking'; f.effort = 'high';
  const model = f.menu.modelsData.models.get(f.model);
  Object.assign(model, { title: 'Fixture model', defaultThinkingEffort: 'high',
    thinkingEfforts: [{ thinking_effort: 'low', short_label: 'Low' },
      { thinking_effort: 'high', full_label: 'High' },
      { thinking_effort: 'max', short_label: 'Extra High', full_label: 'Maximum' }],
    defaultServiceTier: 'standard', serviceTierOptions: [{ service_tier: 'standard' }, { service_tier: 'fast' }] });
  const picker = f.menu.composerIntelligencePickerState;
  picker.bucketSelections = undefined; picker.currentSelection = undefined;
  picker.currentBucket = undefined; picker.isRestrictedModelPickerState = true;
  picker.restrictedModelCapability = { kind: 'thinking-effort', modelConfig: model, thinkingEffortLane: 'thinking' };
  f.modules.conversation.Grn = (conversation, value) => {
    assert.equal(conversation, f.conversation); assert.equal(value, f.model);
    f.writes.push(['model', value]);
  };
  return { f, picker, model };
}

test('restricted efforts reuse the production picker, same-model writer and preferences', async () => {
  const { f } = restrictedFixture();
  f.request(); await flush();
  assert.deepEqual(f.events.at(-1).map(item => item.label), ['Low', 'High', 'Extra High', '高级']);
  assert.equal(f.choice('High').selected, true); assert.equal(f.fallback, 0);
  const id = f.choice('Extra High').id; f.select(id);
  assert.equal(f.effort, 'max'); assert.equal(f.results.at(-1)[1], true);
  assert.equal(f.choice('Extra High').selected, true);
  assert.deepEqual(f.writes, [['effort', 'max', f.model], ['model', f.model],
    ['preference', { modelSlug: f.model, thinkingEffort: 'max' }]]);
  f.select(id); assert.equal(f.writes.length, 3);
  f.request(); assert.equal(f.imports, 3); assert.equal(f.timers.size, 0);
  assert.doesNotMatch(JSON.stringify([f.events, f.results]), /Bearer|00000000|fixture-thinking/);
});

test('restricted Advanced keeps the same-model response tier and returns to efforts', async () => {
  const { f } = restrictedFixture();
  f.request(); await flush(); f.select(f.choice('高级').id);
  assert.equal(f.choice('标准响应速度').selected, true);
  assert.ok(f.choice('返回档位'));
  const id = f.choice('快速响应速度').id; f.select(id); f.select(id);
  assert.deepEqual(f.writes, [['tier', 'fast']]);
  assert.equal(f.choice('快速响应速度').selected, true);
  f.select(f.choice('返回档位').id);
  assert.equal(f.choice('High').selected, true); assert.equal(f.advanced, 0);
});

for (const [current, defaultEffort, selected] of [
  [undefined, 'high', 'High'], ['unknown', 'missing', 'Low'], ['max', 'high', 'Extra High']
]) test('restricted display resolves current, default, then first allowed effort: ' + selected, async () => {
  const { f, picker, model } = restrictedFixture();
  f.effort = current;
  // The fixture's setter simulates a render; restore this restricted picker state.
  picker.bucketSelections = undefined; picker.currentSelection = undefined;
  model.defaultThinkingEffort = defaultEffort;
  f.request(); await flush();
  assert.equal(f.choice(selected).selected, true);
  assert.equal(f.writes.length, 0, 'displaying the effective default is not persistence');
});

test('restricted Pro effort retains the official Pro defaults', async () => {
  const { f, picker } = restrictedFixture();
  picker.restrictedModelCapability.thinkingEffortLane = 'pro';
  f.request(); await flush(); f.select(f.choice('Extra High').id);
  assert.deepEqual(f.writes[0], ['pro-preference', { juices: { pro: 'max' } }]);
  assert.equal(f.results.at(-1)[1], true);
});

test('only allowed named efforts appear and selecting the same effort does not write', async () => {
  const { f, model } = restrictedFixture();
  model.thinkingEfforts.push({ thinking_effort: 'unlabelled' });
  f.modules.conversation.vRt = () => ['high', 'max', 'unlabelled'];
  f.request(); await flush();
  assert.deepEqual(f.events.at(-1).map(item => item.label), ['High', 'Extra High', '高级']);
  f.select(f.choice('High').id); assert.equal(f.writes.length, 0);
  assert.equal(f.results.at(-1)[1], true);
});

for (const [name, mutate] of Object.entries({
  other_kind: ({ picker }) => { picker.restrictedModelCapability.kind = 'fixed'; },
  other_model: ({ picker }) => { picker.restrictedModelCapability.modelConfig = { id: 'other' }; },
  unknown_categories: ({ f }) => { f.menu.modelsData.categories = undefined; },
  empty_categories: ({ f }) => { f.menu.modelsData.categories = []; },
  unknown_efforts: ({ model }) => { model.thinkingEfforts = null; },
  empty_efforts: ({ model }) => { model.thinkingEfforts = []; },
  too_many_efforts: ({ model }) => { model.thinkingEfforts = Array(31).fill(model.thinkingEfforts[0]); },
  duplicate_effort: ({ model }) => { model.thinkingEfforts.push(model.thinkingEfforts[0]); },
  no_allowed_efforts: ({ f }) => { f.modules.conversation.vRt = () => []; },
  permission_denied: ({ f }) => { f.menu.modelSwitcherDenialsBySlug[f.model] = [{}]; },
  missing_model: ({ f }) => { f.menu.modelsData.models.delete(f.model); },
  work_mode: ({ f }) => { f.work = true; },
})) test('restricted malformed or disallowed ' + name + ' cannot authorize effort mutation', () => {
  const data = restrictedFixture(); mutate(data);
  const { f } = data, binding = f.contract.capture(f.getTrigger);
  assert.equal(f.contract.catalog(binding, f.modules), null);
  assert.deepEqual(f.writes, []);
});

for (const [name, mutate] of Object.entries({
  capability: ({ picker }) => { picker.restrictedModelCapability.kind = 'fixed'; },
  allowed_efforts: ({ f }) => { f.modules.conversation.vRt = () => ['high']; },
  permissions: ({ f }) => { f.menu.modelSwitcherDenialsBySlug[f.model] = [{}]; },
  account: ({ f }) => { f.account = 'Bearer changed-synthetic-account'; },
  current_model: ({ f }) => { f.model = 'fixture-fast'; },
  document: ({ f }) => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
})) test('restricted ' + name + ' is revalidated before a stale selection can write', async () => {
  const data = restrictedFixture(), { f } = data;
  f.request(); await flush(); const id = f.choice('Extra High').id;
  mutate(data); f.select(id);
  assert.equal(f.results.at(-1)[1], false); assert.deepEqual(f.writes, []);
  assert.equal(f.fallback, 0); assert.equal(f.advanced, 0);
});

test('an ignored restricted effort mutation is not success or a DOM retry', async () => {
  const { f } = restrictedFixture();
  f.modules.conversation.yRt = () => ({ conversationThinkingEffort$: () => f.effort, setThinkingEffort() {} });
  f.request(); await flush(); f.select(f.choice('Extra High').id);
  assert.equal(f.results.at(-1)[1], false); assert.equal(f.fallback, 0); assert.equal(f.advanced, 0);
  assert.equal(f.effort, 'high');
});

test('an invalid restricted capability cannot reuse a stale normal-picker tier', () => {
  const { f, picker } = restrictedFixture();
  picker.currentSelection = { modelSlug: f.model, thinkingEffort: f.effort,
    serviceTierOptions: [{ service_tier: 'standard' }, { service_tier: 'fast' }] };
  picker.restrictedModelCapability.kind = 'fixed';
  const binding = f.contract.capture(f.getTrigger);
  const value = f.contract.advancedCatalog(binding, f.modules);
  assert.ok(value);
  assert.equal(value.choices.some(item => item.semantic === 'service_tier'), false);
  assert.equal(value.canGoBack, false);
  assert.deepEqual(f.writes, []);
});
