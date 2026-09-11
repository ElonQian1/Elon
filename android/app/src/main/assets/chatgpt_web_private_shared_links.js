(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 6, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateSharedLinks = api;
})(typeof window === 'object' ? window : null, function (page, contract, options) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const CANVAS_ID = /^[A-Za-z0-9_-]{1,128}$/;
  const LIST = '/backend-api/shared_conversations?order=created';
  const now = options?.now || Date.now;
  const caches = new Map();
  const canvasContent = page.__elonChatGptPrivateCanvasContent?.create({ now, current, request });
  let sequence = 0;

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

  function parse(payload, resource) {
    const canvas = resource === 'canvas', rows = canvas ? payload?.shared_textdocs : payload?.items;
    if (!Array.isArray(rows) || rows.length > 1000 || !canvas &&
        (!Number.isSafeInteger(payload.total) || payload.total < rows.length) ||
        canvas && payload.total !== undefined && (!Number.isSafeInteger(payload.total) || payload.total < rows.length)) {
      throw new Error('share_list_unconfirmed');
    }
    const ids = new Set();
    const items = rows.map(row => {
      const id = canvas ? row?.shared_textdoc_id : row?.id;
      const at = canvas ? row?.created_at : row?.create_time;
      if (typeof id !== 'string' || !(canvas ? CANVAS_ID : UUID).test(id) || !canvas && !UUID.test(row.conversation_id || '') ||
          canvas && row.conversation_id != null && !UUID.test(row.conversation_id) || ids.has(id) ||
          row.workspace_id != null && (typeof row.workspace_id !== 'string' || !row.workspace_id) ||
          at != null && (typeof at !== 'string' || at.length > 40 ||
          !/^\d{4}-\d{2}-\d{2}T[0-9:.]+(?:Z|[+-]\d{2}:\d{2})$/.test(at) ||
          !Number.isFinite(Date.parse(at)))) throw new Error('share_list_unconfirmed');
      ids.add(id);
      return Object.freeze({ id, conversationId: row.conversation_id ?? null,
        workspace: row.workspace_id != null, createdAt: at ?? null });
    });
    return { items, complete: (canvas && payload.total === undefined || items.length === payload.total) &&
      payload.has_more !== true && !payload.next_cursor };
  }

  async function request(binding, url, method, mode, timeoutMs = 7000) {
    if (!current(binding)) throw new Error('share_context_changed');
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders())) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
    }
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, url, {
      method, headers, credentials: 'include', cache: 'no-store', redirect: 'error',
      __elonPrivateTransport: 'conversation_shared_links_v1',
    }, { timeoutMs, maxBytes: 1024 * 1024, mode });
    if (!current(binding)) throw new Error('share_context_changed');
    return response;
  }

  function ticket() {
    const bytes = new Uint8Array(16);
    if (!page.crypto?.getRandomValues) throw new Error('share_context_unavailable');
    page.crypto.getRandomValues(bytes);
    return 'sl_' + Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('') + '_' + (++sequence).toString(36);
  }

  async function read(binding, force = false, resource = 'conversation') {
    const cached = caches.get(resource);
    if (!force && cached && current(cached.binding) && cached.binding.account === binding.account &&
        now() - cached.at >= 0 && now() - cached.at < 60000) return cached;
    caches.delete(resource);
    const url = resource === 'canvas' ? '/backend-api/shared_textdocs' : LIST;
    const value = parse((await request(binding, url, 'GET', 'json')).payload, resource);
    const result = { ...value, binding, resource, ticket: ticket(), at: now() };
    caches.set(resource, result);
    return result;
  }

  async function run(input, confirmed) {
    let attempted = false;
    const canvasUpdate = input?.operation === 'update_account' && input?.resource === 'canvas';
    try {
      const id = typeof input?.path === 'string' && input.path.startsWith('/c/') ? input.path.slice(3) : '';
      const accountList = input?.operation === 'list_account';
      const resource = input?.resource === undefined ? 'conversation' : input.resource;
      const canvasRevoke = input?.operation === 'revoke_account' && resource === 'canvas';
      const canvasRead = input?.operation === 'read_account' && resource === 'canvas';
      const canvasAction = canvasRevoke || canvasRead || canvasUpdate;
      if (!['conversation', 'canvas'].includes(resource) ||
          resource === 'canvas' && !accountList && !canvasAction ||
          !accountList && !canvasAction && (!UUID.test(id) || !['list', 'revoke'].includes(input?.operation)) ||
          canvasAction && input.path !== undefined) {
        throw new Error('share_invalid_selection');
      }
      const binding = bind();
      const cached = caches.get(resource);
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
          result = await read(binding, false, resource);
        }
        // This pages the already-returned collection, not an invented HTTP cursor.
        const personal = result.items.filter(row => !row.workspace);
        if (offset > 0 && offset >= personal.length) throw new Error('share_invalid_selection');
        return { ok: true, attempted: false, data: { schema: resource === 'canvas' ? 'elon.canvas_shares.v1' : 'elon.account_shares.v1',
          ticket: result.ticket, offset, nextOffset: offset + 100 < personal.length ? offset + 100 : null,
          complete: result.complete && personal.length === result.items.length,
          items: personal.slice(offset, offset + 100).map(({ id, conversationId, createdAt }) =>
            ({ id, path: conversationId ? '/c/' + conversationId : null, createdAt })) } };
      }
      if (input.operation === 'list') {
        const result = await read(binding), scoped = result.items.filter(row => row.conversationId === id);
        const matching = scoped.filter(row => !row.workspace);
        return { ok: true, attempted: false, data: { schema: 'elon.conversation_shares.v1', path: input.path,
          ticket: result.ticket, complete: result.complete && matching.length === scoped.length && matching.length <= 100,
          items: matching.slice(0, 100).map(({ id, createdAt }) => ({ id, createdAt })) } };
      }
      if (!canvasRead && confirmed !== true) throw new Error('user_confirmation_required');
      const selected = cached;
      if (!(canvasAction ? CANVAS_ID : UUID).test(input.id || '') || !selected || input.ticket !== selected.ticket ||
          now() - selected.at < 0 || now() - selected.at > 120000 || !current(selected.binding) ||
          !selected.items.some(row => row.id === input.id && (canvasAction || row.conversationId === id) && !row.workspace)) {
        throw new Error('share_selection_expired');
      }
      if (canvasRead) {
        if (!canvasContent) throw new Error('share_canvas_unavailable');
        const content = await canvasContent.read(binding, input.id);
        if (caches.get(resource) !== selected || !current(binding)) throw new Error('share_context_changed');
        return { ok: true, attempted: false, code: 'share_canvas_ready', content };
      }
      if (canvasUpdate) {
        if (!canvasContent?.prepareUpdate || !canvasContent?.update) throw new Error('share_canvas_unavailable');
        const deadline = now() + 18000;
        const before = await canvasContent.prepareUpdate(binding, input.id, deadline);
        if (caches.get(resource) !== selected || !current(binding)) throw new Error('share_context_changed');
        // Consume the ticket before POST, including when its outcome later becomes unknown.
        caches.delete(resource);
        options?.invalidateCreated?.();
        attempted = true;
        const content = await canvasContent.update(binding, input.id, before, deadline);
        return { ok: true, attempted: true, code: 'share_canvas_updated', content };
      }
      // Consume selection before the write. A timeout cannot trigger another DELETE.
      caches.delete(resource);
      canvasContent?.invalidate();
      options?.invalidateCreated?.();
      attempted = true;
      await request(binding, (canvasRevoke ? '/backend-api/textdoc/shared/' : '/backend-api/share/') + input.id, 'DELETE', 'none');
      const observed = await read(binding, true, resource);
      if (!observed.complete || observed.items.some(row => row.id === input.id)) throw new Error('share_revoke_unconfirmed');
      return { ok: true, code: 'share_link_revoked', attempted: true };
    } catch (error) {
      const code = String(error?.message || '');
      if (/^http_(401|403)$/.test(code)) {
        caches.clear();
        canvasContent?.invalidate();
        page.__elonChatGptPrivateAuthContext?.invalidate?.('shared_links_rejected');
      }
      return { ok: false, attempted, code: attempted ? (canvasUpdate ? 'share_canvas_update_unconfirmed' : 'share_revoke_unconfirmed') :
        /^(share_[a-z0-9_]+|user_confirmation_required)$/.test(code) ? code : 'share_list_unavailable' };
    }
  }

  return Object.freeze({ version: 6, run, invalidate: () => { caches.clear(); canvasContent?.invalidate(); } });
});
