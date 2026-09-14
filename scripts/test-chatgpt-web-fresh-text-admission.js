'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-fresh-text-context');
function setup() {
  const f = fixture();
  Object.assign(f.page, { setTimeout, clearTimeout });
  f.shared.textApi.safePost = () => { throw Error('inspection_must_not_post'); };
  f.conversation.textSecurity = () => { throw Error('inspection_must_not_prepare'); };
  f.conversation.textStream = () => { throw Error('inspection_must_not_stream'); };
  return f;
}
test('admission uses the actual context without posting, streaming or arming a trial', async () => {
  const f = setup();
  assert.deepEqual(await f.api.inspect(f.node), { schema: 'elon.fresh_text_admission.v1', code: 'ready', stage: 'ready' });
  assert.equal(f.page.__elonChatGptFreshTextTransaction, undefined);
});
test('admission reports bounded failures instead of raw provider exceptions', async () => {
  const f = setup();
  f.selected.config = { foreignMode: true };
  assert.deepEqual(await f.api.inspect(f.node), { schema: 'elon.fresh_text_admission.v1', code: 'scope_unsupported', stage: 'base_config' });
  f.page.__elonChatGptPrivateRuntimeBindings.load = async () => { throw Object.assign(Error('private response'), { admissionStage: 'private header' }); };
  assert.deepEqual(await f.api.inspect(f.node), { schema: 'elon.fresh_text_admission.v1', code: 'read_failed', stage: 'base_context' });
});
test('one inspector coalesces concurrent reads and releases its deadline', async () => {
  const f = setup();
  const timers = new Set();
  f.page.setTimeout = (fn, ms) => { const id = setTimeout(fn, ms); timers.add(id); return id; };
  f.page.clearTimeout = id => { timers.delete(id); clearTimeout(id); };
  const first = f.api.inspect(f.node);
  assert.equal(f.api.inspect(f.node), first);
  await first;
  assert.equal(timers.size, 0);
  assert.notEqual(f.api.inspect(f.node), first);
  await f.api.inspect(f.node);
  assert.equal(timers.size, 0);
});
test('a stalled runtime read has a bounded non-writing timeout', async () => {
  const f = setup(); let expire, cleared = false;
  f.page.setTimeout = fn => { expire = fn; return 1; };
  f.page.clearTimeout = id => { assert.equal(id, 1); cleared = true; };
  f.page.__elonChatGptPrivateRuntimeBindings.load = () => new Promise(() => {});
  const pending = f.api.inspect(f.node); expire();
  assert.deepEqual(await pending, { schema: 'elon.fresh_text_admission.v1', code: 'timeout', stage: 'timeout' });
  assert.equal(cleared, true);
});
test('project-specific read failures retain only their allowlisted stages', async () => {
  const f = setup(), project = 'g-p-' + 'a'.repeat(32);
  f.tree.mode = { kind: 'gizmo_interaction', gizmo_id: project };
  f.tree.isLoading = false; f.tree.is_do_not_remember = false;
  f.shared.HM.getGizmoId = () => project; f.shared.HM.getConversationTurns = () => [];
  Object.assign(f.shared, { textBusinessContext: () => ({}), textProjectHeaders: () => undefined,
    textLockedProjectId: () => null, textLockedChatPin: () => undefined, canvasQueryClient: () => ({}) });
  assert.equal((await f.api.inspect(f.node)).stage, 'project_business');
  f.shared.textBusinessContext = () => null;
  f.shared.textProjectHeaders = () => ({ authorization: 'must-not-export' });
  assert.equal((await f.api.inspect(f.node)).stage, 'project_headers');
  f.shared.textProjectHeaders = () => undefined;
  const cases = [
    ['mode', { kind: 'foreign', gizmo_id: project }, 'project_mode'],
    ['isLoading', true, 'project_loading'], ['is_do_not_remember', true, 'project_privacy'],
    ['sharedProjectConversationOwner', {}, 'project_shared'],
    ['contextScopes', ['GLOBAL', 'HEALTH'], 'project_scopes']
  ];
  for (const [key, value, stage] of cases) {
    const previous = f.tree[key]; f.tree[key] = value;
    assert.equal((await f.api.inspect(f.node)).stage, stage);
    f.tree[key] = previous;
  }
  f.tree.contextScopes = ['GLOBAL'];
  assert.equal((await f.api.inspect(f.node)).code, 'ready');
});
