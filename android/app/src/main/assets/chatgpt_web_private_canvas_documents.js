(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasDocuments = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const policy = options.policy || page.__elonChatGptPrivateCanvasDocumentPolicy;
  const now = options.now || Date.now, caches = new Map();
  const context = (options.context || page.__elonChatGptPrivateCanvasEditContext).create(page, { current });
  const identity = page.__elonChatGptPrivateConversationShareContract.create(page).identity;
  const PATH = /^\/(?:g\/[A-Za-z0-9_-]{1,200}\/)?c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i;
  const fail = code => { throw Error('canvas_' + code); };
  let active = false, sequence = 0, uncertain = null, scope = null;

  function sameSession(binding) {
    try {
      return page.document === binding.document &&
        page.location.origin === 'https://chatgpt.com' && page.__elonChatGptDocumentToken === binding.token &&
        identity() === binding.account;
    } catch (_) { return false; }
  }

  function current(binding) { return sameSession(binding) && page.location.href === binding.href; }

  function bind(path, readSnapshot) {
    const match = typeof path === 'string' && PATH.exec(path), url = new URL(page.location.href);
    if (!match || url.origin !== 'https://chatgpt.com' || url.pathname !== path || url.search || url.hash ||
        url.username || url.password) fail('context_unavailable');
    const binding = { id: match[1], path, href: url.href, document: page.document,
      token: page.__elonChatGptDocumentToken, account: identity(), readSnapshot };
    if (!binding.account || !/^doc_[a-z0-9_]{3,80}$/.test(binding.token || '')) fail('auth_unavailable');
    if (!current(binding)) fail('context_changed');
    if (!scope || !sameSession(scope.binding)) scope = { binding, token: ticket() };
    return binding;
  }

  function budget(deadline) {
    const remaining = Math.min(7000, deadline - now());
    if (remaining <= 0) fail('timeout');
    return remaining;
  }

  async function request(binding, path, method, deadline, body, dispatch) {
    if (!current(binding)) fail('context_changed');
    const timeoutMs = budget(deadline), headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders())) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
    }
    const init = { method, headers, credentials: 'include', cache: 'no-store', redirect: 'error',
      __elonPrivateTransport: 'canvas_documents_v1' };
    if (body !== undefined) { headers['Content-Type'] = 'application/json'; init.body = JSON.stringify(body); }
    dispatch?.();
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, path, init,
      { timeoutMs, maxBytes: 1024 * 1024, mode: 'json' });
    if (!current(binding)) fail('context_changed');
    budget(deadline);
    return response.payload;
  }

  function ticket() {
    const bytes = new Uint8Array(16);
    if (!page.crypto?.getRandomValues) fail('context_unavailable');
    page.crypto.getRandomValues(bytes);
    return 'cd_' + Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('') + '_' + (++sequence).toString(36);
  }

  function remember(binding, documents) {
    const entry = { binding, documents, ticket: ticket(), at: now() };
    caches.set(binding.id, entry);
    for (const [id, value] of caches) if (!current(value.binding)) caches.delete(id);
    while (caches.size > 2) caches.delete(caches.keys().next().value);
    return entry;
  }

  async function fetchDocuments(binding, deadline) {
    return policy.parse(await request(binding, '/backend-api/conversation/' + binding.id + '/textdocs', 'GET', deadline));
  }

  function selected(binding, input) {
    const entry = caches.get(binding.id);
    if (!entry || !current(entry.binding) || input.ticket !== entry.ticket ||
        input.scope != null && input.scope !== scope.token || now() < entry.at || now() - entry.at > 1800000) {
      fail('selection_expired');
    }
    const document = entry.documents.find(value => value.id === input.id);
    if (!document) fail('selection_invalid');
    return { entry, document };
  }

  function result(entry, code = 'canvas_ready') {
    return { ok: true, attempted: false, code, path: entry.binding.path,
      ticket: entry.ticket, scope: scope.token, documents: entry.documents, unconfirmedWrite: !!uncertain && current(uncertain.binding) };
  }

  async function read(binding, force, deadline) {
    if (uncertain && !sameSession(uncertain.binding)) uncertain = null;
    const entry = caches.get(binding.id);
    if (!force && !uncertain && entry && current(entry.binding) && now() >= entry.at && now() - entry.at < 60000) {
      return result(entry);
    }
    return result(remember(binding, await fetchDocuments(binding, deadline)));
  }

  async function save(binding, input, confirmed, deadline) {
    if (confirmed !== true) fail('confirmation_required');
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const { document: before } = selected(binding, input), expected = policy.draft(before, input);
    const owner = await context.capture(binding, before.id);
    const fresh = await fetchDocuments(binding, deadline), original = fresh.find(value => value.id === before.id);
    if (!original || !policy.same(original, before)) fail('version_conflict');
    context.check(owner);
    if (policy.matches(original, before, expected, before.documentVersion)) return result(remember(binding, fresh), 'canvas_unchanged');
    // Consumed immediately before the single write. Neither timeout nor readback failure may replay it.
    let attempted = false;
    try {
      const response = await request(binding, '/backend-api/textdoc/' + before.id, 'POST', deadline,
        { version: before.documentVersion, content: expected.content, comments: expected.comments }, () => {
          context.check(owner);
          caches.delete(binding.id);
          uncertain = { binding, before, expected, version: null, owner };
          attempted = true;
        });
      if (!Number.isSafeInteger(response?.version) || response.version <= before.documentVersion) fail('write_unconfirmed');
      uncertain.version = response.version;
      const documents = await fetchDocuments(binding, deadline), saved = documents.find(value => value.id === before.id);
      if (!policy.matches(saved, before, expected, response.version)) fail('write_unconfirmed');
      const entry = remember(binding, documents);
      uncertain = null;
      // Native confirmation is the server readback, not a DOM refresh or a query refetch completing.
      context.reconcile(owner).catch(() => {});
      return { ...result(entry, 'canvas_saved'), attempted: true };
    } catch (error) {
      if (/^http_(401|403)$/.test(error?.message || '')) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      if (attempted) return { ok: false, code: 'canvas_write_unconfirmed', attempted: true };
      throw error;
    }
  }

  async function verify(binding, input, confirmed, deadline) {
    if (!uncertain || !current(uncertain.binding)) fail('selection_expired');
    const pending = uncertain, { document } = selected(binding, input);
    if (document.id !== pending.before.id) fail('selection_invalid');
    const documents = await fetchDocuments(binding, deadline), saved = documents.find(value => value.id === document.id);
    const version = pending.version ?? saved?.documentVersion;
    const matches = version > pending.before.documentVersion && policy.matches(saved, pending.before, pending.expected, version);
    if (!matches && confirmed !== true) return result(remember(binding, documents), 'canvas_verification_pending');
    if (!matches && (!saved || !policy.same(document, saved))) fail('version_conflict');
    // Clearing a mismatch only acknowledges an explicit comparison, never submits a second POST.
    uncertain = null;
    if (matches) context.reconcile(pending.owner).catch(() => {});
    return result(remember(binding, documents), matches ? 'canvas_saved' : 'canvas_result_acknowledged');
  }

  async function run(input, confirmed, readSnapshot) {
    if (active) return { ok: false, code: 'canvas_busy', attempted: false };
    active = true;
    try {
      if (!input || !['list', 'save', 'verify'].includes(input.operation) || !policy ||
          !page.__elonChatGptPrivateJsonRequest?.request || !page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders) {
        fail('request_invalid');
      }
      const binding = bind(input.path, readSnapshot), deadline = now() + 18000;
      if (input.operation === 'list') return await read(binding, input.force === true, deadline);
      if (input.operation === 'save') return await save(binding, input, confirmed, deadline);
      return await verify(binding, input, confirmed, deadline);
    } catch (error) {
      const code = String(error?.message || '');
      if (/^http_(401|403)$/.test(code)) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      return { ok: false, attempted: false,
        code: /^canvas_[a-z_]+$/.test(code) ? code : /^http_\d{3}$/.test(code) ? 'canvas_' + code : 'canvas_unavailable' };
    } finally { active = false; }
  }

  return Object.freeze({ version: 2, run, busy: () => active });
});
