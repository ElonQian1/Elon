'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const assets = '../android/app/src/main/assets/';
const blocks = require(assets + 'chatgpt_web_text_blocks');
const binding = require(assets + 'chatgpt_web_private_runtime_bindings');
const profiles = [require('./fixtures/chatgpt-runtime-bindings-sep15.cjs'),
  require('./fixtures/chatgpt-runtime-bindings-sep15b.cjs')];
const conversationId = '11111111-1111-4111-8111-111111111111';
const projectId = 'g-p-' + 'a'.repeat(32);

async function fixture(profile, route = 'personal', loaded = true) {
  const message = { id: 'message-1', author: { role: 'assistant' }, status: 'finished_successfully',
    content: { content_type: 'text', parts: [':::writing{id="doc-one" variant="standard"}\nOld\n:::'] },
    metadata: { writing_blocks: { 'doc-one': { content: 'Current saved body\n', variant: 'standard' } } } };
  const owner = { id: 'client-one', serverId$: () => conversationId };
  const thread = { owner: 'synthetic' };
  const pathname = route === 'temporary' ? '/?temporary-chat=true' :
    (route === 'project' ? '/g/' + projectId : '') + '/c/' + conversationId;
  const cdn = 'https://chatgpt.com/cdn/assets/';
  let anchor = profile.anchor, account = 'account-one', owners = [owner], imports = 0;
  const methods = {
    canvasConversations: () => owners,
    XM(key) { assert.equal(key, owner.id); return thread; },
    HM: { getGizmoId(value) { assert.equal(value, thread); return projectId; },
      getNodeIfExists(value, key) { assert.equal(value, thread); return { message: key === message.id ? message : null }; } }
  };
  const namespace = Object.fromEntries(Object.entries(profile.expectedExports.shared)
    .map(([name, exported]) => [exported, methods[name] || { name }]));
  const page = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com' + pathname },
    document: {}, __elonChatGptDocumentToken: 'doc_runtime', setTimeout, clearTimeout,
    performance: { getEntriesByName: url => url === cdn + anchor ? [{}] : [] },
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => account }) },
    __elonChatGptWritingBlockPolicy: { temporaryOwner: (_, shared) => shared ? owners[0] : null },
    fetch() { assert.fail('display must not fetch'); } };
  const runtime = binding.create(page, { loadRuntime: async url => {
    imports++; assert.equal(url, cdn + profile.files.shared); return namespace;
  } });
  page.__elonChatGptPrivateRuntimeBindings = runtime;
  if (loaded) await runtime.load('shared');
  return { page, message, runtime, read: () => blocks.runtimeProjection(page, message.id),
    imports: () => imports, setAccount: value => { account = value; },
    setOwners: value => { owners = value; }, replaceDocument: () => {
      anchor = 'c2675c8c-unknown.js'; page.__elonChatGptDocumentToken = 'doc_replaced';
    } };
}

for (const profile of profiles) {
  for (const route of ['personal', 'project', 'temporary']) {
    test(profile.id + ': ' + route + ' snapshots read current writing through the actual cached resolver', async () => {
      const f = await fixture(profile, route);
      const before = JSON.stringify(f.message), imports = f.imports();
      assert.equal(f.runtime.state().profile_id, profile.id);
      const result = f.read();
      assert.ok(result, 'reviewed runtime must retain current structured writing');
      assert.equal(result.text, 'Current saved body\n');
      assert.equal(result.parts[0].textBlock.content, 'Current saved body\n');
      assert.equal(result.parts[0].textBlock.sourceMessageId, f.message.id);
      assert.equal(result.writeSources, undefined);
      assert.equal(f.imports(), imports);
      assert.equal(JSON.stringify(f.message), before);
    });
  }
  test(profile.id + ': cold rendering does not import, fetch or wait for a composer', async () => {
    const f = await fixture(profile, 'personal', false);
    assert.equal(f.read(), null);
    assert.equal(f.imports(), 0);
  });
  test(profile.id + ': missing identity, duplicate owners and changed document cannot leak stale writing', async () => {
    const f = await fixture(profile);
    f.setAccount(null); assert.equal(f.read(), null);
    f.setAccount('account-one'); f.setOwners([
      { id: 'one', serverId$: () => conversationId }, { id: 'two', serverId$: () => conversationId }
    ]);
    assert.equal(f.read(), null);
    f.replaceDocument(); assert.equal(f.read(), null);
    assert.equal(f.imports(), 1);
  });
  test(profile.id + ': incomplete messages and foreign project routes remain read-only fallback', async () => {
    const f = await fixture(profile, 'project');
    f.message.status = 'in_progress'; assert.equal(f.read(), null);
    f.message.status = 'finished_successfully';
    f.page.location.href = f.page.location.href.replace(projectId, 'g-p-' + 'b'.repeat(32));
    assert.equal(f.read(), null);
    assert.equal(f.imports(), 1);
  });
}
