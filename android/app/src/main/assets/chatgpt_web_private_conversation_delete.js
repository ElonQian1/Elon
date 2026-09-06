(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateConversationDelete) {
    root.__elonChatGptPrivateConversationDelete = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const PATH = /^(?:\/c\/([A-Za-z0-9_-]{1,160})|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/([A-Za-z0-9_-]{1,160}))$/;
  const idFor = path => { const match = PATH.exec(path || ''); return match && (match[1] || match[2]); };
  const transport = root.__elonChatGptPrivateTransport;
  const directory = root.__elonChatGptPrivateConversationDirectory;
  let active = null;
  let cooldownUntil = 0;

  function identity(headers) {
    const values = {};
    for (const [name, value] of Object.entries(headers || {})) values[name.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function current(job) {
    return root.location.origin === 'https://chatgpt.com' && root.location.href === job.href &&
      root.__elonChatGptDocumentToken === job.token &&
      (!job.account || identity(transport.copySameOriginRequestHeaders?.()) === job.account);
  }

  function result(ok, code, attempted) { return Object.freeze({ ok, code, attempted }); }

  function currentConversationRejection(job) {
    if (job.id !== idFor(root.location.pathname)) return '';
    let snapshot;
    try { snapshot = job.readSnapshot?.(); } catch (_) {}
    if (!snapshot || snapshot.url !== root.location.origin + root.location.pathname) return 'delete_context_unavailable';
    if (!snapshot.composerReady || snapshot.streaming || snapshot.dictationActive ||
        snapshot.dictationCaptureActive || snapshot.dictationCapturePending) return 'delete_conversation_busy';
    if (snapshot.draft || !Array.isArray(snapshot.attachments) || snapshot.attachments.length) return 'delete_draft_present';
    return '';
  }

  async function execute(job) {
    let timer;
    let attempted = false;
    try {
      const source = await Promise.race([
        transport.acquireSameOriginRequestHeaders(),
        new Promise((_, reject) => { timer = root.setTimeout(() => reject(new Error('auth_timeout')), 7000); }),
      ]).finally(() => root.clearTimeout(timer));
      if (!current(job)) return result(false, 'delete_context_changed', false);
      if (identity(source) !== job.account || !current(job)) return result(false, 'delete_auth_unavailable', false);
      const rejection = currentConversationRejection(job);
      if (rejection) return result(false, rejection, false);
      const headers = { Accept: 'application/json', 'Content-Type': 'application/json' };
      for (const [name, value] of Object.entries(source || {})) {
        if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
          'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
      }
      const request = (url, init, mode) => root.__elonChatGptPrivateJsonRequest.request(root, url, {
        credentials: 'include', cache: 'no-store', redirect: 'error', headers, ...init,
      }, { timeoutMs: 9000, maxBytes: 1024 * 1024, mode });
      attempted = true;
      try {
        // Official mYi legacy deletion branch, not archive and never a bulk endpoint.
        await request('/backend-api/conversation/' + encodeURIComponent(job.id), {
          method: 'PATCH', body: JSON.stringify({ is_visible: false }),
          __elonPrivateTransport: 'conversation_delete_v1',
        }, 'none');
      } catch (error) {
        if (!current(job)) return result(false, 'delete_result_unconfirmed', true);
        if (/^http_/.test(String(error?.message))) throw error;
        // A timed-out write is never replayed. Absence/404 is not proof of deletion.
        let metadata;
        try {
          metadata = (await request('/backend-api/conversations/' + encodeURIComponent(job.id), {
            method: 'GET', __elonPrivateTransport: 'conversation_delete_reconcile_v1',
          }, 'json')).payload;
        } catch (_) {}
        if (metadata?.is_visible !== false || String(metadata.conversation_id || metadata.id || '') !== job.id) {
          return result(false, 'delete_result_unconfirmed', true);
        }
      }
      if (!current(job)) return result(false, 'delete_result_unconfirmed', true);
      directory.acceptDeletedState(job.id, false);
      return result(true, 'delete_server_acknowledged', true);
    } catch (error) {
      const message = String(error?.message || '');
      if (/^http_(401|403)$/.test(message)) root.__elonChatGptPrivateAuthContext?.invalidate?.('conversation_delete_rejected');
      return result(false, attempted ? (/^http_\d{3}$/.test(message) ? 'delete_' + message :
        'delete_result_unconfirmed') : 'delete_auth_unavailable', attempted);
    }
  }

  function start(path, confirmed, readSnapshot) {
    const id = idFor(path);
    if (confirmed !== true) return Promise.resolve(result(false, 'user_confirmation_required', false));
    if (!id) return Promise.resolve(result(false, 'invalid_conversation_path', false));
    if (root.__elonChatGptPrivateConversationMutationsEnabled !== true || !transport ||
        !root.__elonChatGptPrivateJsonRequest || !directory?.acceptDeletedState ||
        !/^doc_[a-z0-9_]{3,80}$/.test(root.__elonChatGptDocumentToken || '')) {
      return Promise.resolve(result(false, 'delete_unavailable', false));
    }
    if (!directory.snapshot().conversations.some(row => row.id === id)) {
      return Promise.resolve(result(false, 'delete_selection_expired', false));
    }
    if (active || root.__elonChatGptPrivateConversationMutation?.state?.().state === 'busy') {
      return Promise.resolve(result(false, 'delete_busy', false));
    }
    if (Date.now() < cooldownUntil) return Promise.resolve(result(false, 'delete_cooldown', false));
    const account = identity(transport.copySameOriginRequestHeaders?.());
    if (!account) return Promise.resolve(result(false, 'delete_auth_unavailable', false));
    const job = { id, href: root.location.href, token: root.__elonChatGptDocumentToken, account, readSnapshot };
    const rejection = currentConversationRejection(job);
    if (rejection) return Promise.resolve(result(false, rejection, false));
    active = job;
    return execute(job).then(outcome => {
      if (!outcome.ok && outcome.attempted) cooldownUntil = Date.now() + 45000;
      return outcome;
    }).finally(() => { if (active === job) active = null; });
  }

  function handle(action, command, respond, changed, directoryRequests, readSnapshot) {
    if (action !== 'delete_conversation') return false;
    start(command?.value, command?.selected, readSnapshot).then(outcome => {
      // Native must receive the terminal receipt before a current-chat deletion navigates away.
      respond(action, outcome.ok, outcome.code);
      if (outcome.ok) directoryRequests?.emitSnapshot?.(null);
      if (outcome.ok) changed?.(true);
    }).catch(() => respond(action, false, 'delete_result_unconfirmed'));
    return true;
  }

  return Object.freeze({ version: 2, start, handle, busy: () => Boolean(active) });
});
