'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture: attachmentFixture } = require('./fixtures/chatgpt-fresh-text-attachments');
const { fixture: inputFixture } = require('./fixtures/chatgpt-private-text-input');
const tick = () => new Promise(resolve => setImmediate(resolve));

async function fixture(enabled) {
  const base = await attachmentFixture();
  base.page.__elonChatGptPrivateTransport = base.attachmentFixture.root.__elonChatGptPrivateTransport;
  const f = inputFixture(base);
  f.page.__elonChatGptFreshTextAttachmentsEnabled = enabled;
  // The real attachment owner is mounted; the text composer is not.
  assert.equal(f.view.dom.isConnected, false);
  assert.equal(f.attachmentFixture.input.isConnected, true);
  let consumed = 0, requests = 0;
  const prepare = f.page.__elonChatGptPrivateAttachmentSend.prepareSubmit;
  f.page.__elonChatGptPrivateAttachmentSend.prepareSubmit = store => {
    const lease = prepare(store);
    return { ...lease, consumeAccepted: () => { consumed++; return lease.consumeAccepted(); } };
  };
  f.page.fetch = () => { requests++; throw Error('readiness must not make requests'); };
  return Object.assign(f, { consumed: () => consumed, requests: () => requests });
}

test('owned ready TXT PDF and PNG attachments no longer block an enabled memory draft', async () => {
  const f = await fixture(true);
  f.api.snapshot(null); await tick();
  assert.deepEqual(f.api.snapshot(null), { ready: true, draft: f.draft() });
  assert.equal(f.api.setDraft('Synthetic attachment draft', f.draft()), true);
  assert.deepEqual(f.api.snapshot(null), { ready: true, draft: 'Synthetic attachment draft' });
  assert.deepEqual(f.files.files$(), f.selectedFiles);
  assert.equal(f.consumed(), 0);
  assert.equal(f.requests(), 0);
});

test('attachment input is not promoted by defaults, truthy values or tool admission', async () => {
  for (const enabled of [undefined, false, 'true', 1]) {
    const f = await fixture(enabled);
    f.page.__elonChatGptFreshTextToolsEnabled = true;
    f.api.snapshot(null); await tick();
    assert.equal(f.api.snapshot(null).ready, false);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.equal(f.edits.length, 0);
    assert.deepEqual(f.files.files$(), f.selectedFiles);
    assert.equal(f.consumed(), 0);
    assert.equal(f.requests(), 0);
  }
});

test('changing the explicit attachment switch retires stale readiness and cooldown', async () => {
  const f = await fixture(false);
  f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, false);
  f.page.__elonChatGptFreshTextAttachmentsEnabled = true;
  f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, true, 'enabling must not wait for the old failed capture');
  f.page.__elonChatGptFreshTextAttachmentsEnabled = false;
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.equal(f.api.snapshot(null).ready, false);
  await tick();
  assert.equal(f.consumed(), 0);
});

test('changed attachment ownership revokes the cached draft before editing', async () => {
  for (const mutate of [
    f => f.attachmentFixture.setAccount('Bearer synthetic-other-account'),
    f => f.attachmentFixture.setModel('synthetic-other-model'),
    f => { f.attachmentFixture.input.isConnected = false; },
    f => { f.files.files$()[0].status = 'uploading'; },
    f => { f.files.files$()[0].fileSpec.size++; },
  ]) {
    const f = await fixture(true);
    f.api.snapshot(null); await tick();
    assert.equal(f.api.snapshot(null).ready, true);
    mutate(f);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.equal(f.api.snapshot(null).ready, false);
    await tick();
    assert.equal(f.api.snapshot(null).ready, false);
    assert.equal(f.edits.length, 0);
    assert.equal(f.consumed(), 0);
    assert.equal(f.requests(), 0);
  }
});

test('input and sender apply identical scope switches with no mounted composer', async () => {
  const input = require('../android/app/src/main/assets/chatgpt_web_private_text_input');
  const transaction = require('../android/app/src/main/assets/chatgpt_web_fresh_text_transaction');
  const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request');
  const receipts = require('../android/app/src/main/assets/chatgpt_web_fresh_text_receipts');
  const names = ['Tools', 'Projects', 'NewConversations', 'Temporary', 'Attachments'];
  for (const enabled of [undefined, false, true]) {
    for (const selected of [null, ...names]) {
      const page = { document: {}, __elonChatGptDocumentToken: 'synthetic-scope',
        location: { href: 'https://chatgpt.com/' }, AbortController, setTimeout, clearTimeout,
        __elonChatGptPrivateTextTransactionsEnabled: true };
      if (selected) page['__elonChatGptFreshText' + selected + 'Enabled'] = enabled;
      let observed;
      const context = { stamp: () => 'synthetic-owner', async capture(_, __, scope) {
        observed = scope; throw Error('scope_unsupported');
      } };
      const api = input.create(page, { context });
      api.snapshot(null); await tick();
      const expected = observed;
      const sender = transaction.create(page, { context, requests, receipts, reconciliation: {}, recovery: {} });
      try {
        const result = sender.send({ requestId: 'mcp_scope', prompt: 'Synthetic probe', expectedDraft: '',
          composer: null, readDraft: () => '' });
        assert.equal(result.handled, true);
        assert.equal((await result.completion).status, 'unavailable');
        assert.deepEqual(expected, observed, selected + ': ' + enabled);
        assert.equal(sender.trialControl('state').dispatched, false);
      } finally {
        await tick();
        assert.equal(sender.dispose(), true);
      }
    }
  }
});
