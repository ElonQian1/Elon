'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path');
const assets = '../android/app/src/main/assets/';
const transaction = require(assets + 'chatgpt_web_fresh_text_transaction');
const receipts = require(assets + 'chatgpt_web_fresh_text_receipts');
const wire = JSON.parse(fs.readFileSync(path.join(__dirname, 'fixtures/chatgpt-fresh-trial-wire.json'), 'utf8'));

test('actual page transaction output matches the shared Android receipt fixtures', async () => {
  const unavailable = async () => { throw Error('context_unavailable'); };
  const page = { document: {}, __elonChatGptDocumentToken: 'doc_wire_fixture',
    location: { href: 'https://chatgpt.com/' }, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshRegenerationEnabled: true };
  const api = transaction.create(page, { context: { capture: unavailable, stamp: () => 'fixture' },
    regeneration: { capture: unavailable }, receipts, now: () => 1000,
    requests: { create: () => ({ create() { throw Error('unexpected_request'); } }) }, reconciliation: {} });
  const observed = {};
  function capture(name, mode = 'state') { observed[name] = api.trialControl(mode); }
  try {
    capture('idle'); capture('armed', 'start');
    let index = 0;
    for (const operation of ['send', 'regenerate']) {
      const result = api[operation]({ requestId: 'mcp_' + ++index, prompt: 'Synthetic fixture',
        expectedDraft: '', readDraft: () => '' });
      assert.equal(result.handled, true);
      capture(operation + '_preparing');
      assert.equal((await result.completion).status, 'unavailable');
      await new Promise(resolve => setImmediate(resolve));
      capture(operation + '_rejected');
    }
    capture('ended', 'end');
    assert.deepEqual(Object.keys(observed), Object.keys(wire.cases));
    for (const [name, overrides] of Object.entries(wire.cases)) {
      assert.deepEqual(observed[name], { ...wire.common, ...overrides }, name);
    }
  } finally { assert.equal(api.dispose(), true); }
});
