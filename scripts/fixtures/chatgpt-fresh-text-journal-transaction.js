'use strict';
const { fixture: journalFixture, asset, uid } = require('./chatgpt-fresh-text-journal');
function fixture(options = {}) {
  const f = journalFixture(options), { page, binding, calls } = f;
  let draft = '';
  Object.assign(page, { document: {}, __elonChatGptDocumentToken: 'doc_journal_test',
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + uid(2) },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshTextJournalEnabled: options.enabled !== false,
    __elonChatGptFreshTextJournal: asset('journal'), __elonChatGptFreshTextStream: asset('stream') });
  Object.assign(binding, { model: 'fixture-model', effort: null, serviceTier: null,
    draft: { read: () => draft, clear: expected => { if (draft === expected) draft = ''; } },
    reconciled: () => options.reconciled === true, canReconcile: () => true,
    shared: { textApi: { safePost: async () => {
      calls.push({ kind: 'prepare' }); return { conduit_token: 'fixture-conduit' };
    } }, textSecurityHeaders: () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'fixture-requirements' }),
    textTopic() { throw Error('unexpected_topic'); } } });
  binding.runtime.textSecurity = () => ({ chatReq: { token: 'fixture-requirements' } });
  binding.runtime.textStream = (_, value) => (async function* () {
    value.onBeforeRequestStart();
    calls.push({ kind: 'post', body: value.body, rows: f.rows().length });
    if (options.postError) throw Error('fixture_network_lost');
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
  })();
  page.__elonChatGptPrivateStreamTransport = {
    preparePrivateSend: () => true, beginPrivateStream: () => ({ push() {}, finish() {} }),
    finishPrivateSend() {},
  };
  let captures = 0;
  const api = asset('transaction').create(page, { requests: asset('request'), receipts: asset('receipts'),
    context: { stamp: () => options.noIdentity ? '' : 'fixture-account-stamp', capture: async () => {
      captures++; if (options.captureError) throw Error('context_unavailable'); return binding;
    } },
    reconciliation: {}, recovery: { recover: async () => ({ status: 'unknown', code: 'history_unavailable' }) },
    prepareTimeoutMs: 1000, streamTimeoutMs: 1000 });
  const send = (id = 'mcp_journal1') => {
    draft = 'Synthetic next prompt';
    return api.send({ requestId: id, prompt: draft, expectedDraft: draft, readDraft: () => draft });
  };
  return { ...f, api, send, draft: () => draft, captures: () => captures };
}
module.exports = { fixture };
