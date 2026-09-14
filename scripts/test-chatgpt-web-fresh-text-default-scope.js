'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const assets = '../android/app/src/main/assets/';
const transaction = require(assets + 'chatgpt_web_fresh_text_transaction');

async function admission({ composer = {}, enabled, trial = false } = {}) {
  const page = { document: {}, __elonChatGptDocumentToken: 'synthetic-default-scope',
    location: { href: 'https://chatgpt.com/' }, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTextTransactionsEnabled: true };
  if (enabled !== undefined) page.__elonChatGptFreshTextNewConversationsEnabled = enabled;
  let captured;
  const api = transaction.create(page, {
    context: { stamp: () => 'synthetic-owner', async capture(_, __, scope) {
      captured = scope;
      throw Error('scope_unsupported');
    } },
    requests: require(assets + 'chatgpt_web_fresh_text_request'),
    receipts: require(assets + 'chatgpt_web_fresh_text_receipts'),
    reconciliation: {}, recovery: {}
  });
  try {
    if (trial) assert.equal(api.trialControl('start').armed, true);
    const result = api.send({ requestId: 'mcp_scope', prompt: 'Synthetic scope probe', expectedDraft: '',
      composer, readDraft: () => '' });
    assert.equal(result.handled, true);
    assert.equal((await result.completion).status, 'unavailable');
    assert.equal(api.trialControl('state').dispatched, false);
    return captured;
  } finally {
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(api.dispose(), true);
  }
}

test('defaults keep extended scopes off and expose separately gated personal Search and Image', async () => {
  assert.deepEqual(await admission(), { allowNewConversations: true, allowProjects: false,
    allowTools: false, allowPersonalSearch: true, allowPersonalImage: true, allowTemporary: false, allowAttachments: false });
});

test('new-conversation default does not promote the composer-free scope', async () => {
  assert.equal((await admission({ composer: null })).allowNewConversations, false);
});

test('explicit opt-out keeps new-conversation default disabled', async () => {
  assert.equal((await admission({ enabled: false })).allowNewConversations, false);
  assert.equal((await admission({ enabled: false, composer: null })).allowNewConversations, false);
});

test('existing explicit and one-shot research admissions remain available', async () => {
  assert.equal((await admission({ enabled: true, composer: null })).allowNewConversations, true);
  assert.equal((await admission({ trial: true, enabled: false, composer: null })).allowNewConversations, true);
});
