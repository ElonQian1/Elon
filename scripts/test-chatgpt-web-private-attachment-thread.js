'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const projectModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_project.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');
const PROJECT = 'g-p-0123456789abcdef0123456789abcdef';
const CONVERSATION = '01234567-89ab-4cde-8123-456789abcdef';
const LEAF = '11111111-2222-4333-8444-555555555555';
const OTHER_LEAF = '99999999-2222-4333-8444-555555555555';
const URL = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';

function fixture(options = {}) {
  let loads = 0, loaded = true;
  const thread = { isLoading: false, is_do_not_remember: false, projectId: PROJECT, leaf: LEAF,
    nodes: new Set([LEAF, OTHER_LEAF]) };
  const root = { location: { href: 'https://chatgpt.com/c/' + CONVERSATION },
    __elonChatGptDocumentToken: 'doc_synthetic_thread',
    performance: { getEntriesByName: (url, type) => loaded && url === URL && type === 'resource' ? [{}] : [] },
    document: { querySelector: () => null },
    setTimeout: options.fastTimeout ? fn => setTimeout(fn, 10) : setTimeout, clearTimeout };
  const namespace = { XM: id => id === CONVERSATION ? thread : undefined,
    HM: { getGizmoId: value => value.projectId, getCurrentLeafId: value => value.leaf,
      hasNode: (value, id) => value.nodes.has(id) } };
  const project = projectModule.create(root, { loadRuntime: async url => {
    loads++;
    assert.equal(url, URL);
    return options.load ? options.load(namespace) : namespace;
  } });
  const binding = { conversationId: CONVERSATION, isTemporaryChat: false, href: root.location.href,
    token: root.__elonChatGptDocumentToken };
  return { root, thread, namespace, project, binding, loads: () => loads, setLoaded: value => { loaded = value; } };
}

test('the current official leaf, not a server latest-node guess, binds project metadata', async () => {
  const f = fixture();
  const thread = await f.project.captureThread(f.binding, PROJECT);
  assert.equal(thread.leafId, LEAF);
  assert.equal(thread.current(), true);
  const file = new File(['synthetic'], 'fixture.txt', { type: 'text/plain' });
  const context = f.project.uploadContext({ projectId: PROJECT, usesInjestPath: false, thread }, file);
  const body = protocol.processBody('file-fixture', file, context);
  assert.deepEqual(body.metadata.library_file_info, { gizmo_id: PROJECT, is_project: true,
    should_upload_to_project: true, origination_thread_id: CONVERSATION, origination_message_id: LEAF });
  assert.equal(body.gizmo_id, PROJECT);
  assert.equal(body.use_case, 'gizmo');
});

test('current branch reads reuse the same loaded runtime and survive upload cleanup', async () => {
  const f = fixture(), controller = new AbortController();
  const first = await f.project.captureThread(f.binding, PROJECT, controller.signal);
  controller.abort();
  assert.equal(first.current(), true);
  f.setLoaded(false);
  await f.project.captureThread(f.binding, PROJECT);
  assert.equal(f.loads(), 1);
});

test('changing leaf, project, document or conversation revokes branch ownership', async () => {
  for (const change of [f => { f.thread.leaf = OTHER_LEAF; }, f => { f.thread.projectId += '0'; },
    f => { f.root.__elonChatGptDocumentToken = 'doc_replaced'; },
    f => { f.root.location.href = 'https://chatgpt.com/'; }, f => { f.thread.nodes.delete(LEAF); }]) {
    const f = fixture(), branch = await f.project.captureThread(f.binding, PROJECT);
    change(f);
    assert.equal(branch.current(), false);
  }
});

test('unknown module, loading thread, temporary state and missing leaf cannot authorize an origin', async () => {
  for (const change of [f => f.setLoaded(false), f => { f.thread.isLoading = true; },
    f => { f.thread.is_do_not_remember = true; }, f => { delete f.thread.is_do_not_remember; },
    f => { f.thread.leaf = 'unknown'; }, f => { delete f.namespace.HM.hasNode; },
    f => { f.binding.isTemporaryChat = true; }]) {
    const f = fixture();
    change(f);
    await assert.rejects(f.project.captureThread(f.binding, PROJECT));
  }
});

test('a slow official module is bounded and cancelled without creating a replacement state store', async () => {
  const f = fixture({ fastTimeout: true, load: () => new Promise(() => {}) });
  await assert.rejects(f.project.captureThread(f.binding, PROJECT), /composer_context_timeout/);
  const controller = new AbortController();
  const pending = f.project.captureThread(f.binding, PROJECT, controller.signal);
  controller.abort();
  await assert.rejects(pending, /cancelled/);
  assert.equal(f.loads(), 1);
});

test('late module completion cannot bind a replacement page', async () => {
  let release;
  const f = fixture({ load: namespace => new Promise(resolve => { release = () => resolve(namespace); }) });
  const pending = f.project.captureThread(f.binding, PROJECT);
  await Promise.resolve();
  f.root.__elonChatGptDocumentToken = 'doc_changed';
  release();
  await assert.rejects(pending, /composer_changed/);
});

test('project origins are a strict pair and cannot add unrelated processing fields', () => {
  const file = new File(['synthetic'], 'fixture.txt', { type: 'text/plain' });
  const f = fixture(), base = f.project.uploadContext({ projectId: PROJECT, usesInjestPath: false }, file);
  for (const patch of [{ origination_thread_id: CONVERSATION }, { origination_message_id: LEAF },
    { origination_thread_id: '../other', origination_message_id: LEAF }, { hidden_field: true }]) {
    assert.throws(() => protocol.processBody('file-fixture', file,
      { ...base, libraryFileInfo: { ...base.libraryFileInfo, ...patch } }), /unsupported_upload_context/);
  }
  const info = protocol.projectInfo({ ...base, libraryFileInfo: { ...base.libraryFileInfo,
    origination_thread_id: CONVERSATION, origination_message_id: LEAF } });
  assert.equal(Object.isFrozen(info), true);
});
