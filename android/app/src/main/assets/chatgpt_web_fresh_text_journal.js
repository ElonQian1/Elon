(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextJournal = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const history = options.history || page.__elonChatGptFreshTextReconcile.create();
  const userSignature = (options.userIdentity || page.__elonChatGptFreshTextUserIdentity)?.signature;
  const uuid = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const fail = code => { throw Error(code); };
  let store = null, identityCache = null, code = 'idle', recovered = 0;
  const digest = value => page.crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)).then(bytes =>
    Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join(''));

  async function identity(binding, signal) {
    if (signal.aborted || binding.owns() !== true) fail('context_changed');
    const account = binding.recoveryIdentity?.();
    if (typeof account !== 'string' || account.length > 600) fail('recovery_identity_unavailable');
    let ids;
    try { ids = JSON.parse(account); } catch (_) { fail('recovery_identity_unavailable'); }
    if (!Array.isArray(ids) || ids.length !== 2 || ids.some(id =>
      typeof id !== 'string' || !/^[A-Za-z0-9_-]{1,256}$/.test(id))) fail('recovery_identity_unavailable');
    if (identityCache?.account !== account) {
      identityCache = { account, value: digest(account) };
    }
    const accountHash = await identityCache.value;
    if (signal.aborted || binding.owns() !== true || binding.recoveryIdentity() !== account) fail('context_changed');
    return { accountHash, userId: ids[0] };
  }

  function proofBinding(record, candidateId, userId, matchesOriginalUser) {
    return { conversationId: candidateId, parentId: record.parentId, projectId: record.projectId,
      projectUserId: userId, newConversation: record.newConversation, temporary: false,
      operation: record.operation, historyParentId: record.historyParentId,
      matchesOriginalUser,
      isOwnedResponse: message => record.replyIds.includes(message?.id) };
  }

  async function resolve(record, binding, owner, signal) {
    const candidate = record.conversationId || binding.conversationId;
    if (!uuid.test(candidate || '') || candidate !== binding.conversationId ||
        record.projectId !== (binding.projectId ?? null)) fail('recovery_previous_unresolved');
    if (record.operation === 'regenerate' && record.replyIds.length === 0) fail('recovery_previous_unresolved');
    let originalSignature = null;
    const proof = proofBinding(record, candidate, owner.userId, message =>
      originalSignature !== null && userSignature(message) === originalSignature);
    let active = true, verified = false, timer, abort;
    const controller = new page.AbortController();
    const current = () => active && !signal.aborted && !controller.signal.aborted && binding.owns() === true;
    const cancelled = new Promise((_, reject) => {
      abort = () => { controller.abort(); reject(Error('context_changed')); };
      signal.addEventListener('abort', abort, { once: true });
      timer = page.setTimeout(() => {
        controller.abort(); reject(Error('recovery_history_timeout'));
      }, options.timeoutMs ?? 12000);
    });
    try {
      if (!current()) fail('context_changed');
      code = 'reading';
      await Promise.race([Promise.resolve().then(async () => {
        if (record.operation === 'regenerate') {
          if (!userSignature) fail('recovery_previous_unresolved');
          // A regenerated response reuses an existing user ID. Prove its saved
          // content digest in a read-only pass before the official loader may apply it.
          let signature = null;
          await binding.runtime.textHydrateHistory(candidate, {
            forceNetworkFetch: true, includeMessageId: record.userMessageId, signal: controller.signal,
            skipIfExisting: false, source: 'native_fresh_journal_v1', shouldApplyResponse: () => false,
            onConversationLoadedFromNetwork(payload) {
              if (current()) signature = userSignature(payload?.mapping?.[record.userMessageId]?.message);
            },
          });
          if (!current()) fail('context_changed');
          if (typeof signature !== 'string' || signature.length > 50000 ||
              await digest(signature) !== record.userSignatureHash) fail('recovery_previous_unresolved');
          originalSignature = signature;
        }
        if (!current()) fail('context_changed');
        return binding.runtime.textHydrateHistory(candidate, {
          forceNetworkFetch: true, includeMessageId: record.userMessageId, signal: controller.signal,
          skipIfExisting: false, source: 'native_fresh_journal_v1',
          onConversationLoadedFromNetwork(payload) {
            verified = current() && payload?.is_do_not_remember === false && payload.is_temporary_chat !== true &&
              history.ownsResponse(payload, proof, record.userMessageId, record.stopAttempted, record.stopAcknowledged);
          },
          shouldApplyResponse: () => verified && current(),
        });
      }), cancelled]);
      if (!current()) fail('context_changed');
      if (!verified) fail('recovery_previous_unresolved');
      store.remove(record);
      recovered = Math.min(65535, recovered + 1);
      code = 'recovered';
    } catch (error) {
      if (/^(?:context_changed|recovery_[a-z_]{1,40})$/.test(error?.message || '')) throw error;
      fail('recovery_history_unavailable');
    } finally {
      active = false;
      controller.abort();
      page.clearTimeout(timer);
      signal.removeEventListener('abort', abort);
    }
  }

  async function prepare(binding, request, operation, signal) {
    // Never put temporary chats or history-disabled contexts in durable storage.
    if (binding.temporary === true || binding.historyDisabled === true || binding.doNotRemember === true) return null;
    try {
      const owner = await identity(binding, signal);
      store ||= (options.store || page.__elonChatGptFreshTextJournalStore).create(options.storage || page.localStorage);
      const pending = store.list(owner.accountHash).filter(record =>
        record.conversationId === null || record.conversationId === binding.conversationId);
      if (pending.length) {
        for (const record of pending) await resolve(record, binding, owner, signal);
        // Hydration may replace the parent or runtime owner. Capture a fresh
        // binding before preparing a new POST, never reuse the old parent.
        return { recapture: true };
      }
      if (!binding.current() || signal.aborted) fail('context_changed');
      const signature = operation === 'regenerate' ? binding.recoveryUserSignature?.() : null;
      if (operation === 'regenerate' && (typeof signature !== 'string' || signature.length > 50000)) {
        fail('recovery_identity_unavailable');
      }
      const userSignatureHash = operation === 'regenerate' ? await digest(signature) : null;
      if (!binding.current() || signal.aborted) fail('context_changed');
      let record = store.normalize({ version: 1, accountHash: owner.accountHash, attemptId: request.turnId,
        conversationId: binding.conversationId, userMessageId: request.userMessageId, parentId: binding.parentId,
        projectId: binding.projectId ?? null, newConversation: binding.newConversation === true,
        operation, historyParentId: operation === 'regenerate' ? binding.historyParentId : null, userSignatureHash,
        replyIds: [], stopAttempted: false, stopAcknowledged: false, createdAtMs: Date.now() });
      let persisted = false;
      const update = patch => {
        if (!persisted) return;
        try { record = store.update(record, patch); } catch (error) {
          code = safeCode(error); // Keep the last durable uncertainty barrier.
        }
      };
      return Object.freeze({ recapture: false,
        persist() {
          if (signal.aborted || binding.current() !== true) fail('context_changed');
          // The synchronous storage commit precedes the only POST boundary.
          // A crash afterwards cannot be interpreted as an unsent draft.
          record = store.write(record);
          persisted = true;
          code = 'pending';
        },
        adoptConversation(id) {
          if (binding.owns() === true && uuid.test(id || '') && record.conversationId === null) update({ conversationId: id });
        },
        observePayload(payload) {
          const message = payload?.message;
          if (operation !== 'regenerate' || binding.owns() !== true || message?.author?.role !== 'assistant' ||
              !uuid.test(message.id || '') || record.replyIds.includes(message.id) || record.replyIds.length >= 64) return;
          update({ replyIds: [...record.replyIds, message.id] });
        },
        markStop(acknowledged = false) {
          update({ stopAttempted: true, stopAcknowledged: record.stopAcknowledged || acknowledged });
        },
        settled() {
          if (!persisted) return true;
          try { store.remove(record); persisted = false; code = 'settled'; return true; }
          catch (error) { code = safeCode(error); return false; }
        },
      });
    } catch (error) { code = safeCode(error); throw Error(code); }
  }

  function safeCode(error) {
    return /^(?:context_changed|recovery_[a-z_]{1,40})$/.test(error?.message || '')
      ? error.message : 'recovery_storage_unavailable';
  }
  return Object.freeze({ prepare, state: () => ({ code, recovered }) });
});
