(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 3, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateSharedLinks = api;
})(typeof window === 'object' ? window : null, function (page, contract, options) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const LIST = '/backend-api/shared_conversations?order=created';
  const now = options?.now || Date.now;
  let cached = null, sequence = 0;

  function current(binding) {
    try {
      return page.location.origin === 'https://chatgpt.com' &&
        page.__elonChatGptDocumentToken === binding.document &&
        contract.identity() === binding.account;
    } catch (_) { return false; }
  }

  function bind() {
    const binding = { document: page.__elonChatGptDocumentToken, account: contract.identity() };
    if (!/^doc_[a-z0-9_]{3,80}$/.test(binding.document || '') || !binding.account) {
      throw new Error('share_auth_unavailable');
    }
    // The authenticated list supplies ownership; publication has its own runtime contract.
    if (!current(binding)) throw new Error('share_scope_unconfirmed');
    return binding;
  }

  function parse(payload) {
    if (!Array.isArray(payload?.items) || payload.items.length > 1000 ||
        !Number.isSafeInteger(payload.total) || payload.total < payload.items.length) {
      throw new Error('share_list_unconfirmed');
    }
    const ids = new Set();
    const items = payload.items.map(row => {
      if (!UUID.test(row?.id || '') || !UUID.test(row.conversation_id || '') || ids.has(row.id) ||
          row.workspace_id != null && (typeof row.workspace_id !== 'string' || !row.workspace_id) ||
          row.create_time != null && (typeof row.create_time !== 'string' || row.create_time.length > 40 ||
          !/^\d{4}-\d{2}-\d{2}T[0-9:.]+(?:Z|[+-]\d{2}:\d{2})$/.test(row.create_time) ||
          !Number.isFinite(Date.parse(row.create_time)))) throw new Error('share_list_unconfirmed');
      ids.add(row.id);
      return Object.freeze({ id: row.id, conversationId: row.conversation_id,
        workspace: row.workspace_id != null, createdAt: row.create_time ?? null });
    });
    return { items, complete: items.length === payload.total };
  }

  async function request(binding, url, method, mode) {
    if (!current(binding)) throw new Error('share_context_changed');
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders())) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
    }
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, url, {
      method, headers, credentials: 'include', cache: 'no-store', redirect: 'error',
      __elonPrivateTransport: 'conversation_shared_links_v1',
    }, { timeoutMs: 7000, maxBytes: 1024 * 1024, mode });
    if (!current(binding)) throw new Error('share_context_changed');
    return response;
  }

  function ticket() {
    const bytes = new Uint8Array(16);
    if (!page.crypto?.getRandomValues) throw new Error('share_context_unavailable');
    page.crypto.getRandomValues(bytes);
    return 'sl_' + Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('') + '_' + (++sequence).toString(36);
  }

  async function read(binding, force = false) {
    if (!force && cached && current(cached.binding) && cached.binding.account === binding.account &&
        now() - cached.at >= 0 && now() - cached.at < 60000) return cached;
    cached = null;
    const value = parse((await request(binding, LIST, 'GET', 'json')).payload);
    cached = { ...value, binding, ticket: ticket(), at: now() };
    return cached;
  }

  async function run(input, confirmed) {
    let attempted = false;
    try {
      const id = typeof input?.path === 'string' && input.path.startsWith('/c/') ? input.path.slice(3) : '';
      const accountList = input?.operation === 'list_account';
      if (!accountList && (!UUID.test(id) || !['list', 'revoke'].includes(input?.operation))) {
        throw new Error('share_invalid_selection');
      }
      const binding = bind();
      if (accountList) {
        const offset = input.offset === undefined ? 0 : input.offset;
        if (input.path !== undefined || input.id !== undefined || !Number.isSafeInteger(offset) ||
            offset < 0 || offset > 900 || offset % 100 !== 0) throw new Error('share_invalid_selection');
        let result;
        if (input.ticket !== undefined) {
          if (!cached || input.ticket !== cached.ticket || !current(cached.binding) ||
              now() - cached.at < 0 || now() - cached.at > 120000) throw new Error('share_selection_expired');
          result = cached;
        } else {
          if (offset !== 0) throw new Error('share_selection_expired');
          result = await read(binding);
        }
        // This pages the already-returned collection, not an invented HTTP cursor.
        const personal = result.items.filter(row => !row.workspace);
        if (offset > 0 && offset >= personal.length) throw new Error('share_invalid_selection');
        return { ok: true, attempted: false, data: { schema: 'elon.account_shares.v1',
          ticket: result.ticket, offset, nextOffset: offset + 100 < personal.length ? offset + 100 : null,
          complete: result.complete && personal.length === result.items.length,
          items: personal.slice(offset, offset + 100).map(({ id, conversationId, createdAt }) =>
            ({ id, path: '/c/' + conversationId, createdAt })) } };
      }
      if (input.operation === 'list') {
        const result = await read(binding), scoped = result.items.filter(row => row.conversationId === id);
        const matching = scoped.filter(row => !row.workspace);
        return { ok: true, attempted: false, data: { schema: 'elon.conversation_shares.v1', path: input.path,
          ticket: result.ticket, complete: result.complete && matching.length === scoped.length && matching.length <= 100,
          items: matching.slice(0, 100).map(({ id, createdAt }) => ({ id, createdAt })) } };
      }
      if (confirmed !== true) throw new Error('user_confirmation_required');
      const selected = cached;
      if (!UUID.test(input.id || '') || !selected || input.ticket !== selected.ticket ||
          now() - selected.at < 0 || now() - selected.at > 120000 || !current(selected.binding) ||
          !selected.items.some(row => row.id === input.id && row.conversationId === id && !row.workspace)) {
        throw new Error('share_selection_expired');
      }
      // Consume selection before the write. A timeout cannot trigger another DELETE.
      cached = null;
      options?.invalidateCreated?.();
      attempted = true;
      await request(binding, '/backend-api/share/' + input.id, 'DELETE', 'none');
      const observed = await read(binding, true);
      if (!observed.complete || observed.items.some(row => row.id === input.id)) throw new Error('share_revoke_unconfirmed');
      return { ok: true, code: 'share_link_revoked', attempted: true };
    } catch (error) {
      const code = String(error?.message || '');
      if (/^http_(401|403)$/.test(code)) {
        cached = null;
        page.__elonChatGptPrivateAuthContext?.invalidate?.('shared_links_rejected');
      }
      return { ok: false, attempted, code: attempted ? 'share_revoke_unconfirmed' :
        /^(share_[a-z0-9_]+|user_confirmation_required)$/.test(code) ? code : 'share_list_unavailable' };
    }
  }

  return Object.freeze({ version: 3, run, invalidate: () => { cached = null; } });
});
