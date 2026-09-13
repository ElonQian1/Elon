(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptWritingLibrarySession = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const fail = code => { throw Error('writing_' + code); };
  const now = options.now || Date.now;
  let captured, pending;
  const same = page.__elonChatGptWritingBlockPolicy.same;
  function check(deadline) {
    if (!options.current()) fail('context_changed');
    if (now() >= deadline) fail('timeout');
  }
  function stamp(session) {
    return { fileId: session.fileId ?? null, version: session.baseVersionNumber,
      sequence: session.latestIssuedSaveSequence, content: session.lastSavedContent };
  }
  function clean(session, source) {
    return session?.libraryFileId === source.libraryFileId && session.hydratedFromLibrary === true &&
      session.dirty === false && session.hasPendingEditorChanges === false && session.inFlightSaveSequence === null &&
      session.draftContent === source.content && session.lastSavedContent === source.content &&
      Number.isSafeInteger(session.latestIssuedSaveSequence) && session.latestIssuedSaveSequence >= 0 &&
      (session.baseVersionNumber === null || Number.isSafeInteger(session.baseVersionNumber) && session.baseVersionNumber >= 0);
  }
  function currentSession() {
    if (!captured || options.bindings.peek('conversation')?.writingLibrarySessions?.() !== captured.store ||
        options.shared.canvasQueryClient?.() !== captured.client) fail('runtime_unavailable');
    return captured.store.getSessionSnapshot(captured.id);
  }
  async function capture(source, deadline) {
    check(deadline);
    if (!/^libfile[_-][A-Za-z0-9_-]{1,152}$/.test(source.libraryFileId || '')) fail('selection_unavailable');
    if (captured) {
      if (captured.id !== source.libraryFileId || pending) fail('web_edit_pending');
      const session = currentSession();
      if (!clean(session, source) || !same(stamp(session), captured.stamp)) fail('version_conflict');
      return;
    }
    let timer;
    const conversation = await Promise.race([
      options.bindings.load('conversation'),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(Error('writing_runtime_unavailable')),
        Math.min(3000, deadline - now())); })
    ]).finally(() => page.clearTimeout(timer));
    check(deadline);
    const store = conversation?.writingLibrarySessions?.(), client = options.shared.canvasQueryClient?.();
    for (const name of ['getSessionSnapshot', 'retainSession', 'updateDraft', 'beginSave', 'completeSaveSuccess',
      'completeSaveError', 'acknowledgeLocalSave', 'enqueueSave'])
      if (typeof store?.[name] !== 'function') fail('runtime_unavailable');
    if (typeof client?.getQueryCache !== 'function' || typeof client.invalidateQueries !== 'function') fail('runtime_unavailable');
    let session = store.getSessionSnapshot(source.libraryFileId);
    if (!clean(session, source) && (session == null || session.hydratedFromLibrary === false) &&
        page.__elonChatGptWritingLibraryRead) {
      await page.__elonChatGptWritingLibraryRead.hydrate(page, options, source, store, deadline);
      check(deadline);
      session = store.getSessionSnapshot(source.libraryFileId);
    }
    // Reading must establish authority; never promote a conversation seed directly.
    if (!clean(session, source)) fail('web_edit_pending');
    captured = { id: source.libraryFileId, store, client, stamp: stamp(session) };
    currentSession();
  }
  function rollback() {
    if (!pending || !options.current()) return;
    const session = currentSession(), job = pending;
    if (session?.inFlightSaveSequence !== job.sequence || session.latestIssuedSaveSequence !== job.sequence) return;
    captured.store.completeSaveError({ libraryFileId: captured.id, saveSequence: job.sequence });
    if (!session.hasPendingEditorChanges && session.draftContent === job.content) {
      captured.store.updateDraft({ libraryFileId: captured.id, content: job.original.content,
        fileId: session.fileId, source: job.draftSource });
    }
    pending = null;
    const after = currentSession();
    if (clean(after, job.original)) captured.stamp = stamp(after);
  }
  async function write(source, expected, operation, deadline, wasDispatched) {
    await capture(source, deadline);
    const owner = captured, release = owner.store.retainSession(owner.id);
    let cancelled = false, timer;
    try {
      const queued = owner.store.enqueueSave({ libraryFileId: owner.id, save: async () => {
        if (cancelled) fail('timeout');
        check(deadline);
        const session = currentSession();
        if (!clean(session, source) || !same(stamp(session), owner.stamp)) fail('version_conflict');
        owner.store.updateDraft({ libraryFileId: owner.id, content: expected.content,
          fileId: session.fileId, source: 'chat' });
        const started = owner.store.beginSave({ libraryFileId: owner.id, content: expected.content });
        if (!started || !Number.isSafeInteger(started.saveSequence)) {
          const after = currentSession();
          if (options.current() && after?.inFlightSaveSequence === null && !after.hasPendingEditorChanges &&
              after.draftContent === expected.content && same(stamp(after), owner.stamp))
            owner.store.updateDraft({ libraryFileId: owner.id, content: source.content, fileId: session.fileId, source: session.draftSource });
          fail('web_edit_pending');
        }
        pending = { sequence: started.saveSequence, content: expected.content, original: source, draftSource: session.draftSource };
        try { return await operation(); }
        catch (error) {
          // Unknown POST outcomes keep their sequence reserved until a read confirms the same content.
          if (!wasDispatched() || /^http_(400|401|403|404|409|422|429)$/.test(error?.message || '')) rollback();
          throw error;
        }
      } });
      return await Promise.race([queued, new Promise((_, reject) => {
        timer = page.setTimeout(() => { cancelled = true; reject(Error('writing_timeout')); }, Math.max(1, deadline - now()));
      })]);
    } finally { page.clearTimeout(timer); release(); }
  }
  function invalidate(id) {
    const queries = captured.client.getQueryCache().findAll({ queryKey: ['file-preview', id] });
    for (const query of queries) {
      if (typeof query.state?.data === 'string')
        Promise.resolve(captured.client.invalidateQueries({ queryKey: ['file-preview-contents', query.state.data], refetchType: 'all' })).catch(() => {});
    }
    Promise.resolve(captured.client.invalidateQueries({ queryKey: ['file-preview', id], refetchType: 'all' })).catch(() => {});
  }
  function confirm(source, deadline) {
    check(deadline);
    const session = currentSession();
    if (!pending) return captured?.id === source.libraryFileId && clean(session, source) && same(stamp(session), captured.stamp);
    if (source.libraryFileId !== captured.id || source.content !== pending.content ||
        session?.inFlightSaveSequence !== pending.sequence || session.latestIssuedSaveSequence !== pending.sequence ||
        session.hasPendingEditorChanges || session.draftContent !== pending.content) return false;
    captured.store.completeSaveSuccess({ libraryFileId: captured.id, saveSequence: pending.sequence, submittedContent: source.content });
    captured.store.acknowledgeLocalSave({ libraryFileId: captured.id, content: source.content, fileId: session.fileId });
    const after = currentSession();
    if (!clean(after, source)) return false;
    captured.stamp = stamp(after);
    pending = null;
    // Match the official library-linked writing acknowledgement, without refreshing the conversation page.
    try {
      for (const id of new Set([captured.id, after.fileId].filter(Boolean))) invalidate(id);
    } catch (_) { /* Content is confirmed; a missing preview cache is not another write. */ }
    return true;
  }
  return Object.freeze({ capture, write, confirm });
});
