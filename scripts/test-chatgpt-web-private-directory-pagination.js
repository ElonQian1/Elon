'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { assets, fixture, response, emptyPage, deferred, flush, pages } = require('./fixtures/chatgpt-directory-refresh');
const row = id => ({ id, title: 'Fixture ' + id, owner: {} });
const project = id => ({ gizmo: { gizmo: { id, display: { name: 'Fixture project ' + id } } }, conversations: null });

test('ordinary history follows response offsets through the terminal page', async () => {
  const f = fixture(async url => {
    if (url.includes('/snorlax/')) return response(emptyPage(url));
    const offset = Number(new URL(url, 'https://chatgpt.com').searchParams.get('offset'));
    return response({ items: [row('page-' + offset)], offset, limit: 28, total: 57 });
  });
  const result = await f.controller.refresh();
  assert.equal(result.conversationsComplete, true);
  assert.equal(result.projectsComplete, true);
  assert.equal(result.complete, false, 'global cache also contains independent project histories');
  assert.equal(f.calls.length, 4);
  const list = f.accepted.find(([meta]) => meta.family === 'conversations');
  assert.deepEqual(JSON.parse(list[1]).items.map(item => item.id), ['page-0', 'page-28', 'page-56']);
  assert.equal(list[0].replace, false);
});

test('project catalog follows opaque cursors and uses resource titles, not conversation titles', async () => {
  const cursor = 'page/2?plus=+&hash=#';
  const f = fixture(async url => {
    if (!url.includes('/snorlax/')) return response(emptyPage(url));
    const value = new URL(url, 'https://chatgpt.com').searchParams.get('cursor');
    if (value === null) return response({ items: [{ ...project('g-p-first'), title: 'Not a project title' }], cursor });
    assert.equal(value, cursor);
    return response({ items: [project('g-p-second')], cursor: null });
  });
  assert.equal((await f.controller.refresh()).projectsComplete, true);
  const list = f.accepted.find(([meta]) => meta.family === 'projects');
  assert.deepEqual(JSON.parse(list[1]).items.map(item => item.title), ['Fixture project g-p-first', 'Fixture project g-p-second']);
});

test('project history starts at cursor zero and waits for all pages before replacement', async () => {
  const later = deferred();
  const f = fixture(async url => url.endsWith('cursor=0')
    ? response({ items: [row('first')], cursor: 'next' }) : later.promise);
  const pending = f.controller.refresh('g-p-fixture');
  await flush();
  assert.equal(f.accepted.length, 0);
  later.resolve(response({ items: [row('second')], cursor: null }));
  const result = await pending;
  assert.equal(result.complete, true);
  assert.equal(result.pages, 2);
  assert.equal(f.accepted[0][0].replace, true);
  assert.equal(JSON.parse(f.accepted[0][1]).items.length, 2);
});

test('project second-page failure merges validated rows without erasing older rows', async () => {
  const f = fixture(async url => url.endsWith('cursor=0')
    ? response({ items: [row('first')], cursor: 'next' }) : response({}, 503));
  const result = await f.controller.refresh('g-p-fixture');
  assert.equal(result.ok, false);
  assert.equal(result.partial, true);
  assert.equal(f.accepted[0][0].replace, false);
});

test('repeated cursors stop without a replacement or unbounded requests', async () => {
  let n = 0;
  const f = fixture(async () => response({ items: [row('row-' + n++)], cursor: 'again' }));
  const result = await f.controller.refresh('g-p-fixture');
  assert.equal(result.code, 'directory_cursor_stalled');
  assert.equal(f.calls.length, 2);
  assert.equal(f.accepted[0][0].replace, false);
});

test('duplicate-only pages stop even if the server keeps inventing cursors', async () => {
  let n = 0;
  const f = fixture(async () => response({ items: [row('same')], cursor: 'next-' + n++ }));
  assert.equal((await f.controller.refresh('g-p-fixture')).code, 'directory_cursor_stalled');
  assert.equal(f.calls.length, 2);
});

test('bounded history truncation cannot be promoted to a complete replacement', async () => {
  let n = 0;
  const f = fixture(async () => response({ items: Array.from({ length: 28 }, () => row('row-' + n++)), cursor: 'next-' + n }));
  const result = await f.controller.refresh('g-p-fixture');
  assert.equal(result.complete, false);
  assert.equal(result.truncated, true);
  assert.equal(f.calls.length, 8);
  assert.equal(JSON.parse(f.accepted[0][1]).items.length, 200);
  assert.equal(f.accepted[0][0].replace, false);
});

for (const field of ['offset', 'limit', 'total']) {
  test(`missing ${field} is an incomplete directory, not a terminal empty directory`, () => {
    const payload = { items: [], offset: 0, limit: 28, total: 0 };
    delete payload[field];
    const parsed = pages.decode(pages.initial('conversations'), JSON.stringify(payload));
    assert.equal(parsed.knownEnd, false);
  });
}
test('missing/invalid cursor and wrong project payload never claim completeness', () => {
  for (const cursor of [undefined, '', 123, 'unsafe\nvalue']) {
    const parsed = pages.decode(pages.initial('g-p-fixture'), JSON.stringify({ items: [], cursor }));
    assert.equal(parsed.knownEnd, false);
  }
  assert.throws(() => pages.decode(pages.initial('g-p-fixture'), JSON.stringify({ items: [{ id: 'x', title: 'X' }], cursor: null })));
  assert.throws(() => pages.initial('../other'));
});

for (const change of ['account', 'document', 'transport', 'cancel']) {
  test(`project pagination discards all pending rows after ${change}`, async () => {
    const later = deferred(), f = fixture(async url => url.endsWith('cursor=0')
      ? response({ items: [row('first')], cursor: 'next' }) : later.promise);
    const pending = f.controller.refresh('g-p-fixture');
    await flush();
    if (change === 'account') f.headers['chatgpt-account-id'] = 'replacement';
    if (change === 'document') f.root.__elonChatGptDocumentToken = 'doc_replacement_fixture';
    if (change === 'transport') f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport };
    if (change === 'cancel') f.controller.cancel();
    later.resolve(response({ items: [row('second')], cursor: null }));
    assert.equal((await pending).ok, false);
    assert.equal(f.accepted.length, 0);
  });
}

test('actual cache preserves old project rows on partial reads, then removes them at a known end', async () => {
  let terminal = false;
  const f = fixture();
  f.root.fetch = async () => response({ items: [row('new-row')], ...(terminal ? { cursor: null } : {}) });
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_conversation_directory.js'), 'utf8'),
    { window: f.root, location: f.root.location, URL });
  const directory = f.root.__elonChatGptPrivateConversationDirectory;
  directory.acceptConversationMembership('old-row', 'Old fixture', 'g-p-fixture');
  directory.acceptConversationMembership('other-row', 'Other fixture', 'g-p-other');
  assert.equal((await directory.refreshScope('g-p-fixture')).complete, false);
  assert.equal(directory.snapshot().conversations.length, 3);
  terminal = true;
  assert.equal((await directory.refreshScope('g-p-fixture')).complete, true);
  assert.deepEqual(Array.from(directory.snapshot().conversations, item => item.id), ['other-row', 'new-row']);
});

test('passive project catalog cannot turn an embedded conversation into the project title', async () => {
  const f = fixture();
  f.root.fetch = async () => {
    const payload = { items: [{ conversations: { items: [{ id: 'a-chat', gizmo_id: 'g-p-fixture', title: 'Not the project' }] },
      gizmo: { gizmo: { id: 'g-p-fixture', display: { name: 'Actual fixture project' } } } }] };
    return { status: 200, clone: () => ({ text: async () => JSON.stringify(payload) }) };
  };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_conversation_directory.js'), 'utf8'),
    { window: f.root, location: f.root.location, URL });
  await f.root.fetch('/backend-api/gizmos/snorlax/sidebar');
  await flush();
  assert.equal(f.root.__elonChatGptPrivateConversationDirectory.snapshot().projects[0].title, 'Actual fixture project');
});

test('native dispatcher emits a complete empty project snapshot and never falls into DOM on a failed read', async () => {
  const f = fixture();
  const events = [], replies = [];
  let result = { ok: true, code: 'directory_ready', complete: true, pages: 1 };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'), { window: f.root });
  const controller = f.root.__elonChatGptConversationDirectoryRequests.create({
    conversationAdapter: { requestList: () => assert.fail('Unexpected DOM fallback') },
    privateDirectory: { refreshScope: async () => result, snapshot: () => ({ conversations: [], projects: [] }) },
    optional: (_, fn) => fn(), emitEvent: value => events.push(value),
  });
  controller.requestList({ projectScopeId: 'g-p-fixture' }, (...args) => replies.push(args));
  await flush();
  assert.equal(events.length, 1);
  assert.equal(events[0].collection.complete, true);
  assert.equal(events[0].collection.reachedEnd, true);
  assert.equal(replies[0][1], true);
  result = { ok: false, code: 'directory_identity_not_ready' };
  controller.requestList({ projectScopeId: 'g-p-fixture' }, (...args) => replies.push(args));
  await flush();
  assert.equal(events.length, 1);
  assert.equal(replies[1][1], false);
});

for (const projectScopeId of ['', 'g-p-fixture']) {
  test(`failed partial ${projectScopeId ? 'project' : 'global'} read does not settle native refresh ownership`, async () => {
    const f = fixture(), events = [], replies = [];
    let result = { ok: false, partial: true, code: 'directory_timeout', pages: 1 };
    const refresh = async () => result;
    vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'),
      { window: f.root });
    const controller = f.root.__elonChatGptConversationDirectoryRequests.create({
      conversationAdapter: { requestList: () => assert.fail('Unexpected DOM fallback') },
      privateDirectory: { refresh, refreshScope: refresh, snapshot: () => ({
        conversations: [{ id: 'retained', title: 'Fixture', projectId: projectScopeId }], projects: [],
      }) },
      optional: (_, fn) => fn(), emitEvent: value => events.push(value),
    });
    controller.requestList({ projectScopeId }, (...args) => replies.push(args));
    await flush();
    assert.equal(events.length, 0, 'failure receipt must still belong to the refreshing scope');
    assert.deepEqual(replies[0], ['list_conversations', false, 'directory_timeout']);
    result = { ok: true, complete: false, code: 'directory_partial', pages: 2 };
    controller.requestList({ projectScopeId }, (...args) => replies.push(args));
    await flush();
    assert.equal(events.length, 1, 'successful bounded refresh may publish retained cached rows');
    assert.equal(events[0].conversations[0].id, 'retained');
    assert.equal(events[0].collection.complete, false);
    assert.equal(replies[1][1], true);
  });

  test(`unchanged ${projectScopeId ? 'project' : 'global'} snapshots still finish every requested refresh`, async () => {
    const f = fixture(), events = [], replies = [];
    const refresh = async () => ({ ok: true, code: 'directory_ready', complete: Boolean(projectScopeId), pages: 1 });
    vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'),
      { window: f.root });
    const controller = f.root.__elonChatGptConversationDirectoryRequests.create({
      conversationAdapter: { requestList: () => assert.fail('Unexpected DOM fallback') },
      privateDirectory: { refresh, refreshScope: refresh, snapshot: () => ({
        conversations: [{ id: 'unchanged', title: 'Fixture', projectId: projectScopeId }], projects: [],
      }) },
      optional: (_, fn) => fn(), emitEvent: value => events.push(value),
    });
    controller.emitSnapshot(projectScopeId);
    controller.emitSnapshot(projectScopeId);
    assert.equal(events.length, 1, 'passive identical snapshots remain deduplicated');
    for (let n = 0; n < 2; n += 1) {
      controller.requestList({ projectScopeId }, (...args) => replies.push(args));
      await flush();
      assert.equal(events.length, n + 2, 'native completion cannot depend on changed rows');
      assert.equal(replies[n][1], true);
    }
    assert.equal(JSON.stringify(events[1]), JSON.stringify(events[2]));
  });
}
