'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, uid, asset, hash } = require('./fixtures/chatgpt-fresh-text-journal');
const identity = asset('attachment_identity');
function message() {
  return { content: { content_type: 'multimodal_text', parts: [
    { content_type: 'image_asset_pointer', asset_pointer: 'sediment://file_fixtureImage' }, 'Synthetic prompt'] },
    metadata: { attachments: [{ id: 'file-fixtureDocument', name: 'fixture.txt' },
      { id: 'file-fixtureMounted', mounted_library_file_id: 'fixtureMounted', mounted_library_mime_type: 'text/plain' }] } };
}
function attachments(f) {
  const value = message(), signature = identity.signature(value);
  f.request.recoveryAttachmentSignature = () => signature;
  Object.assign(f.payload.mapping[uid(3)].message, value);
  return value;
}

test('a restarted send cannot settle a missing selected attachment by user ID alone', async () => {
  const f = fixture();
  f.request.recoveryAttachmentSignature = () => JSON.stringify([['file', 'file-fixture', null, null]]);
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.rows().length, 1);
  assert.equal(f.calls.some(call => call.kind === 'apply'), false);
});

test('a restarted send cannot settle a different selected attachment by user ID alone', async () => {
  const f = fixture();
  f.request.recoveryAttachmentSignature = () => JSON.stringify([['file', 'file-fixture', null, null]]);
  f.payload.mapping[uid(3)].message.metadata = { attachments: [{ id: 'file-other' }] };
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.rows().length, 1);
  assert.equal(f.calls.some(call => call.kind === 'apply'), false);
});

test('matching document, image and mounted references settle only after guarded history', async () => {
  const f = fixture(), value = attachments(f);
  (await f.prepare()).persist();
  assert.equal(f.rows()[0].version, 2);
  assert.equal(f.rows()[0].attachmentSignatureHash, hash(identity.signature(value)));
  const stored = [...f.storage.values.values()].join('');
  for (const privateValue of ['fixtureDocument', 'fixtureMounted', 'fixtureImage', 'Synthetic prompt', 'fixture.txt']) {
    assert.equal(stored.includes(privateValue), false);
  }
  value.metadata.attachments.reverse();
  value.metadata.attachments[1].name = 'Server display title';
  value.metadata.extra_server_field = true;
  assert.equal((await f.prepare(f.create())).recapture, true);
  assert.equal(f.rows().length, 0);
  assert.equal(f.calls.filter(call => call.kind === 'history').length, 2);
  assert.equal(f.calls.filter(call => call.kind === 'apply').length, 1);
});

for (const kind of ['missing_file', 'extra_file', 'different_image', 'mounted_id', 'mounted_mime', 'duplicate_file']) {
  test('attachment recovery retains uncertainty for ' + kind, async () => {
    const f = fixture(), value = attachments(f);
    (await f.prepare()).persist();
    if (kind === 'missing_file') value.metadata.attachments.pop();
    if (kind === 'extra_file') value.metadata.attachments.push({ id: 'file-other' });
    if (kind === 'different_image') value.content.parts[0].asset_pointer = 'sediment://file_other';
    if (kind === 'mounted_id') value.metadata.attachments[1].mounted_library_file_id = 'other';
    if (kind === 'mounted_mime') value.metadata.attachments[1].mounted_library_mime_type = 'application/pdf';
    if (kind === 'duplicate_file') value.metadata.attachments.push(value.metadata.attachments[0]);
    await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
    assert.equal(f.rows().length, 1);
    assert.equal(f.calls.some(call => call.kind === 'apply'), false);
  });
}

test('a second history response cannot replace attachment evidence after the hash check', async () => {
  let f, reads = 0, applied = false;
  f = fixture({ hydrate: (_, options) => {
    if (++reads === 2) f.payload.mapping[uid(3)].message.metadata.attachments.pop();
    options.onConversationLoadedFromNetwork(f.payload);
    applied ||= options.shouldApplyResponse();
  } });
  attachments(f); (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(reads, 2); assert.equal(applied, false); assert.equal(f.rows().length, 1);
});

test('old records without attachment evidence remain visible and cannot be assumed plain text', async () => {
  const f = fixture(); (await f.prepare()).persist();
  const key = [...f.storage.values.keys()][0], old = JSON.parse(f.storage.getItem(key));
  old.version = 1; delete old.attachmentSignatureHash;
  f.storage.setItem(key, JSON.stringify(old));
  assert.equal(f.rows()[0].version, 1);
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.calls.length, 0); assert.equal(f.rows().length, 1);
});

test('plain-text recovery rejects an unexpected file association without a second read', async () => {
  const f = fixture(); (await f.prepare()).persist();
  f.payload.mapping[uid(3)].message.metadata = { attachments: [{ id: 'file-unselected' }] };
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.calls.filter(call => call.kind === 'history').length, 1);
  assert.equal(f.calls.some(call => call.kind === 'apply'), false);
});

test('legacy regeneration retains its existing full-user and observed-reply proof', async () => {
  const f = fixture(), user = f.payload.mapping[uid(3)].message;
  Object.assign(f.binding, { parentId: uid(3), historyParentId: uid(4),
    recoveryUserSignature: () => asset('user_identity').signature(user) });
  const lease = await f.prepare(f.api, 'regenerate'); lease.persist();
  lease.observePayload({ message: f.payload.mapping[uid(5)].message });
  const key = [...f.storage.values.keys()][0], old = JSON.parse(f.storage.getItem(key));
  old.version = 1; delete old.attachmentSignatureHash;
  f.storage.setItem(key, JSON.stringify(old));
  assert.equal((await f.prepare(f.create())).recapture, true);
  assert.equal(f.rows().length, 0);
});

test('the real request records the same references as its actual POST across recreation', async () => {
  const f = require('./fixtures/chatgpt-fresh-text-journal-transaction').fixture({ postError: true });
  const value = message();
  f.binding.attachments = { mimeTypes: ['text/plain', 'image/png'], message: () => structuredClone(value) };
  const sent = f.send(); assert.equal((await sent.completion).status, 'unknown');
  const post = f.calls.find(call => call.kind === 'post');
  assert.ok(post); assert.equal(f.rows().length, 1);
  const submitted = post.body.messages[0];
  assert.equal(f.rows()[0].attachmentSignatureHash, hash(identity.signature(submitted)));
  const restored = fixture({ storage: f.storage });
  delete restored.payload.mapping[uid(3)];
  restored.payload.mapping[submitted.id] = { id: submitted.id, parent: uid(4), message: submitted };
  restored.payload.mapping[uid(5)].parent = submitted.id;
  assert.equal((await restored.prepare()).recapture, true);
  assert.equal(restored.rows().length, 0);
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
});

test('attachment reference schema rejects malformed or unresolved content', () => {
  for (const mutate of [
    value => { value.metadata.attachments = Array(1); },
    value => { value.metadata.attachments[0].id = ['file-fixtureDocument']; },
    value => { value.metadata.attachments[1].mounted_library_file_id = null; },
    value => { value.metadata.attachments[1].mounted_library_mime_type = null; },
    value => { value.content.parts[0].asset_pointer = 'https://example.test/private'; },
    value => { value.content.parts.push({ content_type: 'unknown' }); },
    value => { value.content.content_type = 'unknown'; },
  ]) {
    const value = message(); mutate(value);
    assert.throws(() => identity.signature(value), /recovery_identity_unavailable/);
  }
});
