(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, hydrate: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptWritingLibraryRead = api;
})(typeof window === 'object' ? window : null, async function (page, options, source, store, deadline) {
  'use strict';
  const fail = code => { throw Error('writing_' + code); };
  const now = options.now || Date.now, shared = options.shared, id = source.libraryFileId;
  function account() { return shared.writingLibraryAccount?.() ?? shared.mq?.(); }
  function scope(value) {
    return value && typeof value.id === 'string' && value.id.trim() === value.id &&
      /^[A-Za-z0-9_-]{1,160}$/.test(value.id) &&
      /^[A-Za-z0-9_-]{1,160}$/.test(value.normalizedAccountUserId || '') &&
      Array.isArray(value.features) && value.features.includes('library_shared_content_available');
  }
  function check() {
    if (!options.current()) fail('context_changed');
    if (now() >= deadline) fail('timeout');
    if (new URL(page.location.href).origin !== 'https://chatgpt.com' ||
        options.bindings.state().profile_id !== 'web_20260912' ||
        options.bindings.peek('shared') !== shared ||
        options.bindings.peek('conversation')?.writingLibrarySessions?.() !== store) fail('runtime_unavailable');
    const value = account();
    if (!scope(value) || value.id !== owner.id || value.normalizedAccountUserId !== owner.user) fail('scope_unconfirmed');
  }
  function idleSeed(session) {
    return session == null || session.libraryFileId === id && session.hydratedFromLibrary === false &&
      session.dirty === false && session.hasPendingEditorChanges === false && session.inFlightSaveSequence === null &&
      session.draftContent === source.content && session.lastSavedContent === source.content &&
      Number.isSafeInteger(session.latestIssuedSaveSequence) && session.latestIssuedSaveSequence >= 0;
  }
  function stamp(session) {
    return session == null ? null : JSON.stringify([session.libraryFileId, session.draftContent, session.lastSavedContent,
      session.baseVersionNumber, session.fileId, session.draftSource, session.hydratedFromLibrary, session.dirty,
      session.hasPendingEditorChanges, session.latestIssuedSaveSequence, session.inFlightSaveSequence]);
  }
  const value = account();
  if (!scope(value)) fail('scope_unconfirmed');
  const owner = { id: value.id, user: value.normalizedAccountUserId };
  check();
  if (!/^libfile[_-][A-Za-z0-9_-]{1,152}$/.test(id || '')) fail('selection_unavailable');
  if (typeof shared.writingLibraryReadHeaders !== 'function' ||
      typeof store.hydrateSessionFromLibrary !== 'function' ||
      typeof page.__elonChatGptPrivateJsonRequest?.request !== 'function') fail('runtime_unavailable');
  const before = store.getSessionSnapshot(id), beforeStamp = stamp(before);
  if (!idleSeed(before)) fail('web_edit_pending');
  const scopeHeaders = shared.writingLibraryReadHeaders(owner.id);
  if (!scopeHeaders || Array.isArray(scopeHeaders) || typeof scopeHeaders !== 'object' ||
      Object.keys(scopeHeaders).length === 0) fail('scope_unconfirmed');
  const headers = {};
  // Merge case-insensitively so a captured header cannot retain another account's scope.
  for (const input of [page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), scopeHeaders]) {
    for (const [name, content] of Object.entries(input)) {
      if (!/^[A-Za-z0-9-]+$/.test(name) || typeof content !== 'string' || /[\r\n]/.test(content)) fail('scope_unconfirmed');
      headers[name.toLowerCase()] = content;
    }
  }
  headers.accept = 'application/json';
  headers['cache-control'] = 'no-cache, no-store';
  check();
  const response = await page.__elonChatGptPrivateJsonRequest.request(page,
    '/backend-api/files/library/shared/files/' + encodeURIComponent(id) + '/text', {
      method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'error', headers,
      __elonPrivateTransport: 'writing_library_read_v1'
    }, { timeoutMs: Math.min(6000, deadline - now()), maxBytes: 1024 * 1024, mode: 'json' });
  check();
  const payload = response?.payload;
  if (!payload || Array.isArray(payload) || typeof payload !== 'object' || payload.library_file_id !== id ||
      typeof payload.content !== 'string' || payload.content.length > 120000 ||
      !Number.isSafeInteger(payload.current_version) || payload.current_version < 0 ||
      !['habitat', 'sediment'].includes(payload.content_backing_kind)) fail('selection_unavailable');
  if (payload.content !== source.content) fail('version_conflict');
  const after = store.getSessionSnapshot(id);
  if (after !== before || stamp(after) !== beforeStamp || !idleSeed(after)) fail('web_edit_pending');
  // Only the reviewed versioned read may hydrate the existing official session.
  // A legacy download URL has no version and cannot grant write authority.
  const result = store.hydrateSessionFromLibrary({ libraryFileId: id, content: payload.content,
    versionNumber: payload.current_version });
  if (result !== 'applied' && result !== 'confirmed')
    fail(result === 'skipped_local_changes' ? 'web_edit_pending' : 'version_conflict');
  check();
  const hydrated = store.getSessionSnapshot(id);
  if (hydrated?.libraryFileId !== id || hydrated.hydratedFromLibrary !== true ||
      hydrated.baseVersionNumber !== payload.current_version || hydrated.lastSavedContent !== payload.content ||
      hydrated.draftContent !== payload.content) fail('version_conflict');
});
