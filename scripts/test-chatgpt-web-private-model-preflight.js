'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture } = require('./fixtures/chatgpt-model-state');
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

async function ready(pro = true) {
  const f = fixture();
  f.model = 'fixture-thinking';
  f.effort = 'low';
  f.request(); await flush();
  return { f, id: f.choice(pro ? 'Extra High' : 'High').id };
}

function rejectedWithoutWrites(f, id) {
  f.select(id);
  assert.equal(f.results.at(-1)[1], false);
  assert.deepEqual(f.writes, []);
  assert.equal(f.model, 'fixture-thinking');
  assert.equal(f.effort, 'low');
  assert.equal(f.fallback, 0);
  assert.equal(f.advanced, 0);
}

for (const pro of [false, true]) {
  for (const [name, create] of Object.entries({
    absent: () => null,
    malformed: () => ({ mutate: true }),
    throws: () => { throw new Error('fixture_preference_unavailable'); }
  })) test(`${pro ? 'Pro' : 'normal'} effort checks ${name} preference before any state write`, async () => {
    const { f, id } = await ready(pro);
    f.modules.shared.RW = create;
    rejectedWithoutWrites(f, id);
  });
}

for (const [name, change] of Object.entries({
  account: f => { f.account = 'Bearer fixture-another-account'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
  route: f => { f.page.location.href = 'https://chatgpt.com/'; },
  conversation: f => { f.menu.conversation = { ...f.conversation }; },
  permission: f => { f.menu.modelSwitcherDenialsBySlug['fixture-thinking'] = [{}]; },
  version: f => { f.menu.composerIntelligencePickerState.selectedVersionEntry.id = 'changed-version'; }
})) test(`preference acquisition rechecks ${name} before its first write`, async () => {
  const { f, id } = await ready();
  const create = f.modules.shared.RW;
  f.modules.shared.RW = () => { const mutation = create(); change(f); return mutation; };
  rejectedWithoutWrites(f, id);
});

for (const kind of ['effort', 'tier']) test(`preference acquisition preserves concurrent ${kind} selection`, async () => {
  const { f, id } = await ready();
  const create = f.modules.shared.RW;
  f.modules.shared.RW = () => {
    f[kind] = kind === 'effort' ? 'high' : 'fast';
    return create();
  };
  f.select(id);
  assert.equal(f.results.at(-1)[1], false); assert.deepEqual(f.writes, []);
  assert.equal(f.effort, kind === 'effort' ? 'high' : 'low');
  assert.equal(f.tierStore.getDraftServiceTier(), kind === 'tier' ? 'fast' : undefined);
  assert.equal(f.fallback, 0);
});

test('unused preference factories are not required by a different-model selection', async () => {
  const f = fixture();
  f.modules.shared.RW = f.modules.conversation.M1t = () => {
    throw new Error('unused_fixture_factory');
  };
  f.request(); await flush(); f.select(f.choice('High').id);
  assert.equal(f.results.at(-1)[1], true);
  assert.deepEqual(f.writes, [['effort', 'high', 'fixture-thinking'],
    ['official-model-action', 'fixture-thinking', 'high'], ['model', 'fixture-thinking']]);
});

test('confirmed preset remains idempotent without acquiring preference writers', async () => {
  const f = fixture(); f.model = 'fixture-thinking'; f.effort = 'max';
  f.modules.shared.RW = f.modules.conversation.M1t = () => {
    throw new Error('unused_fixture_factory');
  };
  f.request(); await flush(); f.select(f.choice('Extra High').id);
  assert.equal(f.results.at(-1)[1], true); assert.deepEqual(f.writes, []);
});

test('a writer that throws after dispatch remains uncertain and is never replayed', async () => {
  const { f, id } = await ready();
  let calls = 0;
  f.modules.shared.RW = () => ({ mutate() {
    calls++; throw new Error('fixture_after_dispatch');
  } });
  f.select(id);
  assert.equal(f.results.at(-1)[1], false); assert.equal(calls, 1);
  assert.equal(f.effort, 'max');
  f.select(id);
  assert.equal(f.results.at(-1)[1], false); assert.equal(calls, 1);
  assert.equal(f.fallback, 0); assert.equal(f.advanced, 0);
});
