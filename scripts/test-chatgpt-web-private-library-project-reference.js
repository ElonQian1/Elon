'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fixture = require('./fixtures/chatgpt-library-append');
const assets = '../android/app/src/main/assets/';
const composerModule = require(assets + 'chatgpt_web_private_attachment_composer');
const senderModule = require(assets + 'chatgpt_web_private_attachment_send');
const projectModule = require(assets + 'chatgpt_web_private_attachment_project');
const PROJECT = 'g-p-' + 'a'.repeat(32), OTHER = 'g-p-' + 'b'.repeat(32);
const CID = '00000000-0000-4000-8000-000000000001';
const LEAF = '00000000-0000-4000-8000-000000000002';
const NEXT = '00000000-0000-4000-8000-000000000003';

function setup(options = {}) {
  const f = fixture(), requests = [], responses = [];
  const thread = { projectId: PROJECT, leaf: LEAF, isLoading: false, is_do_not_remember: false };
  const context = { conversationId: CID, projectId: PROJECT, nodeIds: [LEAF, NEXT] };
  const project = { id: PROJECT, current_user_permission: { can_write: options.canWrite !== false }, use_injest_path: false };
  Object.assign(f.props, { isProjectEnabledForGizmo: true, gizmoId: PROJECT,
    isProjectThread: true, libraryEligibilityReason: 'eligible', isFileUploadEnabled: true });
  f.root.location.href = 'https://chatgpt.com' + (options.path || '/g/' + PROJECT + '/c/' + CID);
  Object.assign(f.namespace, { XM: id => id === CID ? thread : null,
    HM: { getGizmoId: t => t.projectId, getCurrentLeafId: t => t.leaf,
      hasNode: (t, id) => id === t.leaf },
    t6: () => ({ loadingStatus: 'Ready', getFeatureGate: name => ({ name, value: false,
      details: { reason: 'Network:Recognized' } }) }) });
  f.root.__elonChatGptPrivateRuntimeBindings.observed = () => true;
  Object.assign(f.root, { __elonChatGptPrivateAttachmentProject: projectModule,
    fetch: async (url, init) => {
      requests.push({ url, method: init.method });
      assert.equal(init.method, 'GET', 'reference must not upload, copy or mutate a project file');
      assert.equal(url, '/backend-api/gizmos/' + PROJECT);
      await options.beforePermission?.(f, thread);
      return Response.json({ gizmo: project });
    } });
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => {
    await options.beforeContext?.(f, thread);
    return context;
  };
  const composer = composerModule.create(f.root);
  const sender = senderModule.create(f.root, { composer });
  const attach = async (handle, requestId = 'mcp_project' + responses.length) => {
    await sender.attachLibrary({ selected: true, requestId, value: JSON.stringify({ fileHandle: handle }) },
      (...r) => responses.push(r), () => {});
    return responses.at(-1);
  };
  return { ...f, composer, sender, attach, responses, requests, thread, context, project };
}

test('eligible existing and new project chats reuse a personal file without project upload', async () => {
  for (const path of ['/g/' + PROJECT + '/project', '/g/' + PROJECT + '-fixture/c/' + CID, '/c/' + CID]) {
    for (const canWrite of [true, false]) {
      const f = setup({ path, canWrite }), handle = f.source(1, { name: 'raster.png', mime_type: 'image/png',
        library_artifact_type: 'image_gen', file_size_bytes: 662362 });
      assert.equal((await f.attach(handle))[1], true, path);
      const [item] = f.store.files$();
      assert.equal(item.file.size, 0);
      assert.equal(item.fileSpec.size, 662362);
      assert.equal(item.libraryArtifactType, 'image_gen');
      assert.equal(item.libraryFileId, 'libfile_1');
      assert.equal(item.fileId, 'file-1');
      assert.equal(item.source, 'library');
      for (const key of ['projectGizmoId', 'libraryFileInfo', 'storeInLibrary']) {
        assert.equal(Object.hasOwn(item, key), false, 'official reference does not create project upload metadata');
      }
      assert.equal(f.requests.length, 1);
      assert.equal(f.validations.length, 2, 'project first reference is also checked against live official limits');
      const lease = f.sender.prepareSubmit(f.store);
      assert.equal(lease.current(), true);
      assert.equal(lease.readyFiles[0].libraryFileId, 'libfile_1');
      assert.equal(lease.consumeAccepted(), true);
      assert.equal(lease.consumeAccepted(), true);
      assert.equal(f.store.files$().length, 0);
    }
  }
});

test('project gate, recall setting, missing identity and temporary state do not grant a reference', async () => {
  for (const change of [
    f => { f.props.libraryEligibilityReason = 'project_recall_gate_disabled'; },
    f => { f.props.libraryEligibilityReason = 'project_recall_disabled'; },
    f => { delete f.props.libraryEligibilityReason; },
    f => { f.props.isLibraryEnabled = false; },
    f => { f.props.isProjectEnabledForGizmo = false; },
    f => { f.props.gizmoId = OTHER; },
    f => { f.props.gizmoId = null; },
    f => { f.props.isProjectThread = false; },
    f => { f.props.isTemporaryChat = true; },
  ]) {
    const f = setup(); change(f);
    assert.equal((await f.attach(f.source(1)))[1], false);
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.requests.length, 0);
  }
});

test('server conversation, project and selected branch must agree with composer eligibility', async () => {
  for (const change of [
    f => { f.context.projectId = OTHER; },
    f => { f.context.conversationId = NEXT; },
    f => { f.context.nodeIds = [NEXT]; },
    f => { delete f.context.projectId; f.context.ordinary = true; },
    f => { f.thread.projectId = OTHER; },
    f => { f.thread.isLoading = true; },
    f => { f.project.id = OTHER; },
  ]) {
    const f = setup(); change(f);
    assert.equal((await f.attach(f.source(1)))[1], false);
    assert.equal(f.store.files$().length, 0);
  }
});

test('project first reference respects official count and size rejection', async () => {
  for (const change of [f => { f.props.maxLibraryAttachmentCount = 0; }, f => f.reject()]) {
    const f = setup(); change(f);
    assert.equal((await f.attach(f.source(1)))[1], false);
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.requests.length, 0);
  }
});

test('reference receipts identify the failing scope check without exposing server data', async () => {
  for (const [change, code] of [
    [f => { f.context.nodeIds = []; }, 'project_branch_mismatch'],
    [f => { f.thread.isLoading = true; }, 'project_branch_unconfirmed'],
    [f => { f.context.projectId = OTHER; }, 'project_identity_mismatch'],
    [f => { delete f.context.projectId; }, 'project_membership_unconfirmed'],
    [f => { delete f.project.current_user_permission; }, 'project_permission_unconfirmed'],
    [f => { f.project.use_injest_path = 'unknown'; }, 'project_upload_policy_unconfirmed'],
    [f => { f.project.id = OTHER; }, 'project_identity_unconfirmed'],
  ]) {
    const f = setup(); change(f);
    const result = await f.attach(f.source(1));
    assert.equal(result[1], false);
    assert.equal(result[2], 'library_attachment_' + code);
    assert.equal(f.store.files$().length, 0);
    assert.equal(JSON.stringify(result).includes(PROJECT), false);
  }
});

test('project append, duplicate reference, native removal and submit revocation share one owner', async () => {
  const f = setup(), a = f.source(1), b = f.source(2);
  assert.equal((await f.attach(a))[1], true);
  assert.equal((await f.attach(a))[1], true);
  assert.equal(f.store.files$().length, 1);
  assert.equal((await f.attach(b))[1], true);
  assert.equal(f.requests.length, 1, 'confirmed project scope is reused within the owned collection');
  assert.equal(f.store.files$().length, 2);
  const lease = f.sender.prepareSubmit(f.store), [native] = f.sender.merge([]);
  assert.equal(lease.current(), true);
  assert.equal(f.sender.remove(native.id, () => {}, () => {}), true);
  assert.equal(lease.current(), false);
  assert.equal(f.store.files$().length, 1);
});

test('eligibility, account, document, model or branch drift revokes pending reference and submission', async () => {
  const changes = [f => { f.props.libraryEligibilityReason = 'project_recall_disabled'; },
    f => { f.props.gizmoId = OTHER; }, f => f.setAccount('Bearer synthetic-changed-token'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_changed'; },
    f => f.setModel('changed'), f => { f.thread.leaf = NEXT; }];
  for (const change of changes) {
    const f = setup();
    assert.equal((await f.attach(f.source(1)))[1], true);
    const lease = f.sender.prepareSubmit(f.store);
    change(f);
    assert.equal(lease.current(), false);
    assert.equal(f.sender.prepareSubmit(f.store), null);
    assert.equal((await f.attach(f.source(2)))[1], false);
    assert.equal(f.store.files$().length, 1, 'do not erase the existing input on failure');
  }
  for (const change of changes.slice(0, 5)) {
    const f = setup({ beforePermission: f => change(f) });
    assert.equal((await f.attach(f.source(1)))[1], false);
    assert.equal(f.store.files$().length, 0);
  }
});

test('branch switch during project permission read cannot attach to a stale leaf', async () => {
  const f = setup({ beforePermission: (_, thread) => { thread.leaf = NEXT; } });
  assert.equal((await f.attach(f.source(1)))[1], false);
  assert.equal(f.store.files$().length, 0);
});

test('local append reuses the existing project upload context instead of library persistence', async () => {
  for (const canWrite of [true, false]) {
    const f = setup({ canWrite });
    assert.equal((await f.attach(f.source(1)))[1], true);
    const append = f.composer.captureUpload(), file = new File(['local fixture'], 'local.txt', { type: 'text/plain' });
    assert.throws(() => f.composer.uploadContext(append.binding, file), /unsupported_upload_context/);
    assert.equal(await f.composer.prepare(append.binding, new AbortController().signal, file), true);
    const context = f.composer.uploadContext(append.binding, file);
    assert.equal(context.projectScopeId, PROJECT);
    assert.equal(context.storeInLibrary, false);
    assert.equal(context.useCase, canWrite ? 'gizmo' : 'ace_upload');
    assert.equal(context.libraryFileInfo.origination_thread_id, CID);
    assert.equal(context.libraryFileInfo.origination_message_id, LEAF);
    assert.equal(context.libraryFileInfo.should_upload_to_project === true, canWrite);
    append.associateMany([{ leaseId: 'fixture_local', file, result: { ok: true, stage: 'processed', associated: false,
      binding: append.binding, fileId: 'file-local', fileName: file.name, fileSize: file.size, mimeType: file.type,
      isTemporaryChat: false, projectId: PROJECT, projectWriteRequested: canWrite, metadata: {} } }]);
    const lease = f.sender.prepareSubmit(f.store);
    assert.equal(lease.current(), true);
    assert.equal(lease.readyFiles.length, 2);
    assert.equal(lease.readyFiles[0].source, 'library');
    assert.equal(lease.readyFiles[1].source, 'local');
    assert.equal(lease.readyFiles[1].isProjectThread, true);
  }
});

test('library image references do not need the unrelated new-upload image-index gate', async () => {
  for (const path of ['/g/' + PROJECT + '/project', '/g/' + PROJECT + '/c/' + CID]) {
    const f = setup({ path });
    f.project.use_injest_path = true;
    let gateReads = 0;
    f.namespace.t6 = () => ({ loadingStatus: 'Ready', getFeatureGate: name => {
      gateReads++;
      return { name, value: false, details: { reason: 'Unrecognized' } };
    } });
    const file = new File(['image'], 'local.png', { type: 'image/png' });
    assert.equal((await f.attach(f.source(1, { name: 'reference.png', mime_type: 'image/png' })))[1], true);
    assert.equal(gateReads, 0);
    const append = f.composer.captureUpload();
    assert.throws(() => f.composer.uploadContext(append.binding, file), /unsupported_upload_context/);
    assert.equal(await f.composer.prepare(append.binding, new AbortController().signal, file), null);
    assert.equal(gateReads, 1, 'real upload still evaluates its own policy');
    assert.equal(f.store.files$().length, 1, 'failed promotion retains the reference');
    assert.throws(() => f.composer.uploadContext(append.binding, file), /composer_changed/);
  }
});

test('upload promotion retains the owned input lease across an asynchronous scope read', async () => {
  let reads = 0;
  const f = setup({ beforePermission: f => {
    if (++reads === 2) f.store.files$.set([]);
  } });
  assert.equal((await f.attach(f.source(1)))[1], true);
  const append = f.composer.captureUpload(), file = new File(['text'], 'local.txt', { type: 'text/plain' });
  await assert.rejects(f.composer.prepare(append.binding, new AbortController().signal, file), /composer_changed/);
  assert.equal(f.store.files$().length, 0, 'user removal is not undone');
  assert.equal(append.current(), false);
});
