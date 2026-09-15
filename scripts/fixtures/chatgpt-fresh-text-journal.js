'use strict';
const crypto = require('node:crypto');
const asset = name => require('../../android/app/src/main/assets/chatgpt_web_fresh_text_' + name);
const uid = value => '10000000-0000-4000-8000-' + String(value).padStart(12, '0');
const account = JSON.stringify(['fixture-user', 'fixture-account']);
const hash = value => crypto.createHash('sha256').update(value).digest('hex');
function memory() {
  const values = new Map();
  return { values, get length() { return values.size; }, key: index => [...values.keys()][index] ?? null,
    getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) };
}
function fixture(options = {}) {
  let current = true, identity = account;
  const storage = options.storage || memory(), calls = [];
  const page = { crypto, AbortController, setTimeout, clearTimeout, localStorage: storage,
    __elonChatGptFreshTextJournalStore: asset('journal_store'),
    __elonChatGptFreshTextReconcile: asset('reconcile'), __elonChatGptFreshTextUserIdentity: asset('user_identity'),
    __elonChatGptFreshTextAttachmentIdentity: asset('attachment_identity') };
  const binding = { conversationId: uid(2), parentId: uid(4), projectId: null, newConversation: false,
    temporary: false, historyDisabled: false, doNotRemember: false,
    owns: () => current, current: () => current, recoveryIdentity: () => identity,
    runtime: { async textHydrateHistory(id, value) {
      calls.push({ kind: 'history', id, value });
      if (options.hydrate) return options.hydrate(id, value);
      value.onConversationLoadedFromNetwork(payload);
      if (value.shouldApplyResponse()) calls.push({ kind: 'apply' });
    } } };
  const request = { turnId: uid(1), userMessageId: uid(3), recoveryAttachmentSignature: () => null };
  const payload = { conversation_id: uid(2), is_do_not_remember: false, is_temporary_chat: false,
    current_node: uid(5), async_status: 4, mapping: {
      [uid(3)]: { id: uid(3), parent: uid(4), message: { id: uid(3), author: { role: 'user' },
        content: { content_type: 'text', parts: ['Synthetic fixture question'] } } },
      [uid(5)]: { id: uid(5), parent: uid(3), message: { id: uid(5), author: { role: 'assistant' },
        content: { content_type: 'text', parts: ['Synthetic fixture answer'] }, status: 'finished_successfully', end_turn: true } },
    } };
  const controller = new AbortController();
  const create = () => asset('journal').create(page, { timeoutMs: options.timeoutMs ?? 100 });
  const api = create();
  return { page, api, create, storage, calls, binding, request, payload, controller,
    setCurrent: value => { current = value; }, setIdentity: value => { identity = value; },
    rows: () => asset('journal_store').create(storage).list(hash(account)),
    prepare: (target = api, operation = 'send') => target.prepare(binding, request, operation, controller.signal) };
}
module.exports = { fixture, memory, uid, account, hash, asset };
