(function (root, factory) {
  'use strict';
  const api = { version: 2, create: factory };
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptScheduledTasks ||= factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  let core, busy = false;
  const identity = () => page.__elonChatGptPrivateConversationShareContract.create(page).identity();
  async function accountScope() {
    await page.__elonChatGptPrivateAuthContext.acquireRequestHeaders();
    const before = identity(), token = page.__elonChatGptDocumentToken;
    if (!before) throw Error('tasks_auth_required');
    // Cookie identity is authoritative even while the page's captured headers are hydrating.
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, '/api/auth/session', {
      method: 'GET', credentials: 'include', cache: 'no-store', redirect: 'error', headers: { Accept: 'application/json' },
    }, { timeoutMs: 5000, maxBytes: 256 * 1024, mode: 'json' });
    const user = response.payload?.user?.id, workspace = response.payload?.account?.id;
    const headers = page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders();
    const capturedAccount = Object.entries(headers).find(([key]) => key.toLowerCase() === 'chatgpt-account-id')?.[1];
    if (typeof user !== 'string' || !/^[A-Za-z0-9_-]{1,256}$/.test(user) ||
        typeof workspace !== 'string' || !/^[A-Za-z0-9_-]{1,256}$/.test(workspace)) throw Error('tasks_auth_required');
    if (capturedAccount && capturedAccount !== workspace) throw Error('tasks_account_changed');
    const bytes = await page.crypto.subtle.digest('SHA-256', new TextEncoder().encode(JSON.stringify(['group_tasks_v1', user, workspace])));
    const scope = Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, '0')).join('');
    if (identity() !== before || page.__elonChatGptDocumentToken !== token) throw Error('tasks_context_changed');
    return scope;
  }
  function handle(action, command, respond, emit) {
    if (action !== 'scheduled_tasks') return false;
    if (busy) { respond(action, false, 'tasks_busy'); return true; }
    busy = true;
    (async () => {
      const input = JSON.parse(command.value), scope = await accountScope(), owner = identity();
      if (input.expectedScope && input.expectedScope !== scope) throw Error('tasks_account_changed');
      delete input.expectedScope;
      core ||= page.__elonChatGptPrivateTasks.create(page);
      const result = await core.run(input);
      if (identity() !== owner) throw Error('tasks_context_changed');
      // Private content goes only to the dedicated consumer, never command receipts.
      emit({ type: 'scheduled_tasks', requestId: respond.requestId, scope, result });
      respond(action, result.ok, result.code);
    })().catch(error => respond(action, false, /^tasks_[a-z_]+$/.test(error?.message || '') ? error.message : 'tasks_unavailable'))
      .finally(() => { busy = false; });
    return true;
  }
  return Object.freeze({ version: 2, handle });
});
