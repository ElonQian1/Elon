(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 6, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasDocuments = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const policy = options.policy || page.__elonChatGptPrivateCanvasDocumentPolicy;
  const now = options.now || Date.now, caches = new Map(), histories = new Map();
  const context = (options.context || page.__elonChatGptPrivateCanvasEditContext).create(page, { current });
  const identity = page.__elonChatGptPrivateConversationShareContract.create(page).identity;
  const PATH = /^\/(?:g\/[A-Za-z0-9_-]{1,200}\/)?c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i;
  const fail = code => { throw Error('canvas_' + code); };
  let active = false, sequence = 0, uncertain = null, scope = null;
  let sharing;

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
    if (!scope || !sameSession(scope.binding)) { scope = { binding, token: ticket() }; histories.clear(); }
    return binding;
  }

  function budget(deadline) {
    const remaining = Math.min(7000, deadline - now());
    if (remaining <= 0) fail('timeout');
    return remaining;
  }

  async function request(binding, path, method, deadline, body, dispatch, mode = 'json') {
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
      { timeoutMs, maxBytes: 1024 * 1024, mode });
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

  function exportFile(binding, input) {
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const { entry, document } = selected(binding, input);
    const exporter = page.__elonChatGptPrivateCanvasExport;
    if (!exporter?.register) fail('export_unavailable');
    const value = exporter.register(page, binding, document, input.format, async generate => {
      if (active) fail('busy');
      active = true;
      let prepared;
      try {
        if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
        const deadline = now() + 25000, owner = await context.capture(binding, document.id);
        const checkOriginal = async () => {
          const original = (await fetchDocuments(binding, deadline)).find(row => row.id === document.id);
          if (!original || !policy.same(original, document)) fail('version_conflict');
          context.check(owner);
        };
        await checkOriginal();
        prepared = await generate(() => context.check(owner));
        // This endpoint has no version parameter. Do not save a result if the source changed while rendering.
        await checkOriginal();
        return prepared;
      } catch (error) {
        try { await prepared?.response?.body?.cancel(); } catch (_) {}
        throw error;
      } finally { active = false; }
    });
    return { ...result(entry), exportFile: value };
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
    return persist(binding, before, before, expected, owner, '/backend-api/textdoc/' + before.id,
      { version: before.documentVersion, content: expected.content, comments: expected.comments }, deadline);
  }

  async function persist(binding, before, expectedBase, expected, owner, path, body, deadline, dismissing = false) {
    // Consumed immediately before the single write. Neither timeout nor readback failure may replay it.
    let attempted = false;
    try {
      const response = await request(binding, path, dismissing ? 'DELETE' : 'POST', deadline, body, () => {
          context.check(owner);
          caches.delete(binding.id);
          histories.clear();
          uncertain = { kind: dismissing ? 'comment_dismissal' : 'content', binding, before, expectedBase, expected, version: null, owner };
          attempted = true;
        });
      if (!Number.isSafeInteger(response?.version) || response.version <= before.documentVersion) fail('write_unconfirmed');
      uncertain.version = response.version;
      const documents = await fetchDocuments(binding, deadline), saved = documents.find(value => value.id === before.id);
      if (!policy.matches(saved, expectedBase, expected, response.version)) fail('write_unconfirmed');
      const entry = remember(binding, documents);
      uncertain = null;
      // Native confirmation is the server readback, not a DOM refresh or a query refetch completing.
      context.reconcile(owner).catch(() => {});
      return { ...result(entry, dismissing ? 'canvas_comment_dismissed' : 'canvas_saved'), attempted: true };
    } catch (error) {
      if (/^http_(401|403)$/.test(error?.message || '')) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      if (attempted) return { ok: false, code: 'canvas_write_unconfirmed', attempted: true };
      throw error;
    }
  }

  async function verify(binding, input, confirmed, deadline) {
    if (!uncertain || !current(uncertain.binding)) fail('selection_expired');
    if (uncertain.kind === 'share') fail('share_verification_required');
    const pending = uncertain, { document } = selected(binding, input);
    if (document.id !== pending.before.id) fail('selection_invalid');
    const documents = await fetchDocuments(binding, deadline), saved = documents.find(value => value.id === document.id);
    const version = pending.version ?? saved?.documentVersion;
    const renaming = pending.kind === 'rename';
    const matches = renaming ? policy.renamed(saved, pending.before, pending.title) :
      version > pending.before.documentVersion && policy.matches(saved, pending.expectedBase, pending.expected, version);
    if (!matches && confirmed !== true) return result(remember(binding, documents), 'canvas_verification_pending');
    if (!matches && (!saved || !policy.same(document, saved))) fail('version_conflict');
    // Clearing a mismatch only acknowledges an explicit comparison, never submits a second POST.
    uncertain = null;
    if (matches) context.reconcile(pending.owner).catch(() => {});
    const verifiedCode = renaming ? 'canvas_renamed' : pending.kind === 'comment_dismissal' ? 'canvas_comment_dismissed' : 'canvas_saved';
    return result(remember(binding, documents), matches ? verifiedCode : 'canvas_result_acknowledged');
  }

  async function dismissComment(binding, input, confirmed, deadline) {
    if (confirmed !== true) fail('confirmation_required');
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const { document: before } = selected(binding, input), expected = policy.dismissComment(before, input.commentId);
    const owner = await context.capture(binding, before.id);
    const fresh = await fetchDocuments(binding, deadline), original = fresh.find(value => value.id === before.id);
    if (!original || !policy.same(original, before)) fail('version_conflict');
    context.check(owner);
    // Official DISMISS is a version-bound comment DELETE, not a body save or ACCEPT/AI edit.
    const path = '/backend-api/textdoc/' + before.id + '/' + before.documentVersion + '/comment/' + input.commentId + '?reason=dismiss';
    return persist(binding, before, before, expected, owner, path, undefined, deadline, true);
  }

  async function rename(binding, input, confirmed, deadline) {
    if (confirmed !== true) fail('confirmation_required');
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const { document: before } = selected(binding, input), title = policy.renameTitle(input.title);
    const owner = await context.capture(binding, before.id);
    const fresh = await fetchDocuments(binding, deadline), original = fresh.find(value => value.id === before.id);
    if (!original || !policy.same(original, before)) fail('version_conflict');
    context.check(owner);
    if (title === before.title) return result(remember(binding, fresh), 'canvas_unchanged');
    let attempted = false;
    try {
      // The official rename owner consumes no result body. A 2xx/204 alone is not
      // success here: confirm the title and unchanged source by canonical readback.
      await request(binding, '/backend-api/textdoc/' + before.id + '/rename', 'POST', deadline, { title }, () => {
        context.check(owner);
        caches.delete(binding.id);
        histories.clear();
        uncertain = { kind: 'rename', binding, before, title, owner };
        attempted = true;
      }, 'none');
      const documents = await fetchDocuments(binding, deadline), saved = documents.find(value => value.id === before.id);
      if (!policy.renamed(saved, before, title)) fail('write_unconfirmed');
      uncertain = null;
      context.reconcile(owner).catch(() => {});
      return { ...result(remember(binding, documents), 'canvas_renamed'), attempted: true };
    } catch (error) {
      if (/^http_(401|403)$/.test(error?.message || '')) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      if (attempted) return { ok: false, code: 'canvas_write_unconfirmed', attempted: true };
      throw error;
    }
  }

  async function readHistory(binding, document, beforeVersion, deadline) {
    if (!Number.isSafeInteger(beforeVersion) || beforeVersion < 1 || beforeVersion > document.documentVersion) fail('history_invalid');
    if (beforeVersion === 1) return [];
    const payload = await request(binding, '/backend-api/textdoc/' + document.id + '/history?before_version=' + beforeVersion, 'GET', deadline);
    if (!Array.isArray(payload?.previous_doc_states) || payload.previous_doc_states.length > 200) fail('history_unconfirmed');
    const versions = payload.previous_doc_states.map(row => policy.parse([row])[0]);
    let previous = beforeVersion;
    for (const version of versions) {
      if (version.id !== document.id || version.documentVersion >= previous) fail('history_unconfirmed');
      previous = version.documentVersion;
    }
    return versions;
  }

  async function history(binding, input, deadline) {
    const { entry, document } = selected(binding, input);
    const versions = await readHistory(binding, document, input.beforeVersion, deadline);
    const selection = { binding, before: document, versions, beforeVersion: input.beforeVersion, token: ticket(), at: now() };
    histories.set(document.id, selection);
    while (histories.size > 2) histories.delete(histories.keys().next().value);
    const oldest = versions.at(-1)?.documentVersion;
    return { ...result(entry), history: { documentId: document.id, ticket: selection.token, beforeVersion: input.beforeVersion,
      nextBeforeVersion: oldest > 1 ? oldest : null, versions } };
  }

  async function restore(binding, input, confirmed, deadline) {
    if (confirmed !== true) fail('confirmation_required');
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const { document: before } = selected(binding, input), selection = histories.get(before.id);
    if (!selection || !current(selection.binding) || selection.token !== input.historyTicket ||
        !policy.same(selection.before, before) || now() < selection.at || now() - selection.at > 1800000) fail('history_selection_expired');
    const target = selection.versions.find(value => value.documentVersion === input.restoreVersion);
    if (!target) fail('history_selection_invalid');
    const owner = await context.capture(binding, before.id);
    const fresh = await fetchDocuments(binding, deadline), original = fresh.find(value => value.id === before.id);
    if (!original || !policy.same(original, before)) fail('version_conflict');
    const versions = await readHistory(binding, before, selection.beforeVersion, deadline);
    const verified = versions.find(value => value.documentVersion === target.documentVersion);
    if (!verified || !policy.same(verified, target)) fail('history_changed');
    context.check(owner);
    const expected = { content: target.content, comments: policy.comments(target.comments, target.content, true) };
    return persist(binding, before, target, expected, owner, '/backend-api/textdoc/' + before.id + '/restore',
      { version: before.documentVersion, restore_from_version: target.documentVersion }, deadline);
  }

  async function share(binding, input, confirmed, deadline) {
    if (input.operation === 'share_ack' && uncertain && sameSession(uncertain.binding) && uncertain.kind !== 'share') fail('write_unconfirmed');
    const { entry, document: before } = selected(binding, input);
    sharing ||= page.__elonChatGptPrivateCanvasDocumentSharing.create(page, { request, current, now });
    const value = await sharing.lookup(binding, before.id, deadline);
    if (uncertain && sameSession(uncertain.binding) && uncertain.kind === 'share' && uncertain.before.id === before.id) {
      if (sharing.matches(value, uncertain.before) || input.operation === 'share_ack' && confirmed === true) {
        context.reconcile(uncertain.owner, 'share').catch(() => {});
        uncertain = null;
      }
    }
    const ready = () => ({ ...result(entry, 'canvas_share_ready'), share: sharing.display(before.id, value) });
    if (input.operation !== 'share_create' || value.state !== 'missing') return ready();
    if (confirmed !== true) fail('confirmation_required');
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
    if (uncertain && sameSession(uncertain.binding)) fail('write_unconfirmed');
    const owner = await context.capture(binding, before.id);
    const fresh = await fetchDocuments(binding, deadline), original = fresh.find(value => value.id === before.id);
    if (!original || !policy.same(original, before)) fail('version_conflict');
    context.check(owner);
    let attempted = false;
    try {
      const created = await sharing.create(binding, before, deadline, () => {
        context.check(owner);
        caches.delete(binding.id);
        uncertain = { kind: 'share', binding, before, owner };
        attempted = true;
      });
      uncertain = null;
      context.reconcile(owner, 'share').catch(() => {});
      return { ...result(remember(binding, fresh), 'canvas_share_created'), attempted: true, share: sharing.display(before.id, created) };
    } catch (error) {
      if (/^http_(401|403)$/.test(error?.message || '')) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      if (attempted) return { ok: false, code: 'canvas_share_write_unconfirmed', attempted: true };
      throw error;
    }
  }

  async function run(input, confirmed, readSnapshot) {
    if (active) return { ok: false, code: 'canvas_busy', attempted: false };
    active = true;
    try {
      if (!input || !['list', 'save', 'rename', 'dismiss_comment', 'prepare_export', 'verify', 'history', 'restore', 'share_lookup', 'share_create', 'share_ack'].includes(input.operation) || !policy ||
          !page.__elonChatGptPrivateJsonRequest?.request || !page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders) {
        fail('request_invalid');
      }
      const binding = bind(input.path, readSnapshot), deadline = now() + 18000;
      if (input.operation === 'list') return await read(binding, input.force === true, deadline);
      if (input.operation === 'prepare_export') return exportFile(binding, input);
      if (input.operation === 'save') return await save(binding, input, confirmed, deadline);
      if (input.operation === 'rename') return await rename(binding, input, confirmed, deadline);
      if (input.operation === 'dismiss_comment') return await dismissComment(binding, input, confirmed, deadline);
      if (input.operation === 'history') return await history(binding, input, deadline);
      if (input.operation === 'restore') return await restore(binding, input, confirmed, deadline);
      if (input.operation.startsWith('share_')) return await share(binding, input, confirmed, deadline);
      return await verify(binding, input, confirmed, deadline);
    } catch (error) {
      const code = String(error?.message || '');
      if (/^http_(401|403)$/.test(code)) page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_rejected');
      return { ok: false, attempted: false,
        code: /^canvas_[a-z_]+$/.test(code) ? code : /^http_\d{3}$/.test(code) ? 'canvas_' + code : 'canvas_unavailable' };
    } finally { active = false; }
  }

  return Object.freeze({ version: 6, run, busy: () => active });
});
