(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextJournalStore = api;
})(typeof window === 'object' ? window : null, function (storage) {
  'use strict';
  // Keep the storage namespace so older unresolved records cannot disappear
  // when the record schema changes.
  const prefix = 'elon.fresh.pending.v1.';
  const uuid = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const hash = /^[a-f0-9]{64}$/;
  const project = /^g-p-[a-f0-9]{32}$/i;
  const isUuid = value => typeof value === 'string' && uuid.test(value);
  const isHash = value => typeof value === 'string' && hash.test(value);
  const baseFields = ['version', 'accountHash', 'attemptId', 'conversationId', 'userMessageId',
    'parentId', 'projectId', 'newConversation', 'operation', 'historyParentId', 'userSignatureHash',
    'replyIds', 'stopAttempted', 'stopAcknowledged', 'createdAtMs'];
  const fail = code => { throw Error(code); };
  const call = fn => {
    try { return fn(); } catch (error) {
      if (error?.message?.startsWith('recovery_')) throw error;
      fail('recovery_storage_unavailable');
    }
  };
  function normalize(value) {
    const fields = value?.version === 2 ? [...baseFields, 'attachmentSignatureHash'] : baseFields;
    if (!value || typeof value !== 'object' || Array.isArray(value) ||
        Object.keys(value).length !== fields.length || fields.some(key => !Object.prototype.hasOwnProperty.call(value, key)) ||
        ![1, 2].includes(value.version) || !isHash(value.accountHash) ||
        value.version === 2 && !(value.attachmentSignatureHash === null || isHash(value.attachmentSignatureHash)) ||
        !isUuid(value.attemptId) || !isUuid(value.userMessageId) ||
        !(value.conversationId === null || isUuid(value.conversationId)) ||
        !(value.projectId === null || typeof value.projectId === 'string' && project.test(value.projectId)) ||
        typeof value.newConversation !== 'boolean' ||
        !['send', 'regenerate'].includes(value.operation) ||
        !Number.isSafeInteger(value.createdAtMs) || value.createdAtMs < 0 ||
        typeof value.stopAttempted !== 'boolean' || typeof value.stopAcknowledged !== 'boolean' ||
        value.stopAcknowledged && !value.stopAttempted ||
        !Array.isArray(value.replyIds) || value.replyIds.length > 64 ||
        Array.from(value.replyIds).some(id => !isUuid(id)) ||
        new Set(value.replyIds).size !== value.replyIds.length) fail('recovery_record_invalid');
    if (value.operation === 'regenerate') {
      if (!isHash(value.userSignatureHash) || value.attachmentSignatureHash != null ||
          value.newConversation || !value.conversationId || value.projectId !== null ||
          value.parentId !== value.userMessageId || typeof value.historyParentId !== 'string' ||
          !(uuid.test(value.historyParentId) || ['', 'client-created-root'].includes(value.historyParentId))) {
        fail('recovery_record_invalid');
      }
    } else if (value.userSignatureHash !== null || value.historyParentId !== null ||
        (value.newConversation ? value.parentId !== 'client-created-root'
          : !isUuid(value.parentId) || !value.conversationId)) fail('recovery_record_invalid');
    return Object.freeze(Object.fromEntries(fields.map(key =>
      [key, key === 'replyIds' ? Object.freeze([...value.replyIds]) : value[key]])));
  }
  function encode(value) { return JSON.stringify(normalize(value)); }
  function key(value) { return prefix + value.accountHash + '.' + value.attemptId; }
  function get(key) { return call(() => storage.getItem(key)); }
  function decode(raw, expectedKey) {
    if (typeof raw !== 'string' || raw.length > 16384) fail('recovery_record_invalid');
    let parsed;
    try { parsed = JSON.parse(raw); } catch (_) { fail('recovery_record_invalid'); }
    const value = normalize(parsed);
    if (key(value) !== expectedKey) fail('recovery_record_invalid');
    return value;
  }
  function list(accountHash) {
    if (!isHash(accountHash)) fail('recovery_identity_unavailable');
    return call(() => {
      const size = storage.length;
      if (!Number.isInteger(size) || size < 0 || size > 8192) fail('recovery_storage_unavailable');
      const values = [], scopedPrefix = prefix + accountHash + '.';
      for (let index = 0; index < size; index++) {
        const name = storage.key(index);
        if (typeof name !== 'string' || !name.startsWith(scopedPrefix)) continue;
        const raw = storage.getItem(name);
        if (raw === null) continue;
        if (values.length >= 16) fail('recovery_capacity');
        values.push(decode(raw, name));
      }
      return values;
    });
  }
  function write(record) {
    const next = normalize(record), serialized = encode(next), name = key(next), previous = get(name);
    if (previous !== null && previous !== serialized) fail('recovery_record_changed');
    if (previous === null && list(next.accountHash).length >= 16) fail('recovery_capacity');
    call(() => storage.setItem(name, serialized));
    if (get(name) !== serialized) fail('recovery_storage_unavailable');
    return next;
  }
  function update(record, patch) {
    const before = normalize(record);
    if (!patch || Object.keys(patch).some(field =>
      !['conversationId', 'replyIds', 'stopAttempted', 'stopAcknowledged'].includes(field))) fail('recovery_record_invalid');
    const after = normalize({ ...before, ...patch });
    if (before.conversationId !== null && after.conversationId !== before.conversationId ||
        before.replyIds.some(id => !after.replyIds.includes(id)) ||
        before.stopAttempted && !after.stopAttempted || before.stopAcknowledged && !after.stopAcknowledged) {
      fail('recovery_record_invalid');
    }
    if (get(key(before)) !== encode(before)) fail('recovery_record_changed');
    const serialized = encode(after);
    call(() => storage.setItem(key(after), serialized));
    if (get(key(after)) !== serialized) fail('recovery_storage_unavailable');
    return after;
  }
  function remove(record) {
    const value = normalize(record), name = key(value), raw = get(name);
    if (raw === null) return true;
    if (raw !== encode(value)) fail('recovery_record_changed');
    call(() => storage.removeItem(name));
    if (get(name) !== null) fail('recovery_storage_unavailable');
    return true;
  }
  return Object.freeze({ normalize, list, write, update, remove });
});
