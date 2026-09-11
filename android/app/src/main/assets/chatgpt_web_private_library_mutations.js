(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateLibraryMutations) {
    root.__elonChatGptPrivateLibraryMutations = factory(root);
  }
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const ACTION = 'mutate_library_file', receipts = new Map();
  const raster = root.__elonChatGptPrivateLibraryRasterPolicy ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_private_library_raster_policy') : null);
  let active = null, retryAt = 0, retryContext = null;
  const outcome = (ok, code) => ({ ok, code });
  const validName = name => typeof name === 'string' && name === name.trim() && name.length > 0 &&
    name.length <= 180 && !/[\x00-\x1f\x7f/\\]/.test(name) && !['.', '..'].includes(name);

  function capabilities(file) {
    const ordinary = file?.kind === 'file' && /^libfile[_-][A-Za-z0-9_-]{1,200}$/.test(file.id || '') &&
      file.external_account == null && file.cloud_doc_url == null &&
      (file.library_artifact_type == null || raster?.matches(file) === true) &&
      file.saved_entity == null && file.trashed_at == null && file.is_project !== true;
    return { canRename: ordinary, canTrash: ordinary && /^file[_-][A-Za-z0-9_-]{1,200}$/.test(file.file_id || '') };
  }

  function deletionCompleted(text) {
    // Official QSr consumes newline-delimited JSON (dqt), not the chat SSE protocol.
    let buffer = '', completed = false, count = 0;
    for (const line of String(text || '').split('\n')) {
      buffer += line + '\n';
      if (!buffer.trim()) { buffer = ''; continue; }
      let event;
      try { event = JSON.parse(buffer); } catch (_) { continue; }
      buffer = '';
      if (++count > 512 || !event || typeof event !== 'object' || Array.isArray(event)) return false;
      if (event.event === 'file.deletion.error') return false;
      if (event.event === 'file.deletion.completed') completed = true;
    }
    return completed && !buffer.trim();
  }

  async function execute(input, selection) {
    const file = selection.source;
    const url = new URL('/backend-api/files/library/files/' + encodeURIComponent(file.id) +
      (input.operation === 'trash' ? '/delete_stream' : ''), root.location.origin);
    if (input.operation === 'trash') {
      url.searchParams.set('file_id', file.file_id);
      if (file.parent_directory_id != null) url.searchParams.set('parent_directory_id', file.parent_directory_id);
      url.searchParams.set('file_name', file.name);
      url.searchParams.set('soft_delete', 'true');
    }
    const headers = { ...root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), 'Content-Type': 'application/json' };
    if (!selection.current()) return outcome(false, 'library_selection_expired');
    let confirmed = false;
    try {
      const result = await root.__elonChatGptPrivateJsonRequest.request(root, url.href, {
        method: input.operation === 'rename' ? 'PATCH' : 'POST',
        credentials: 'same-origin', cache: 'no-store', redirect: 'error', headers,
        ...(input.operation === 'rename' ? { body: JSON.stringify({ file_name: input.name }) } : {}),
        __elonPrivateTransport: 'library_mutation_v1',
      }, { timeoutMs: 20000, maxBytes: 65536, mode: input.operation === 'rename' ? 'none' : 'text' });
      confirmed = input.operation === 'rename' || deletionCompleted(result.text);
      if (!selection.current()) return outcome(false, 'library_result_unconfirmed');
      return confirmed ? outcome(true, 'library_mutation_acknowledged') : outcome(false, 'library_result_unconfirmed');
    } catch (error) {
      const code = String(error?.message || '');
      return outcome(false, selection.current() && /^http_\d{3}$/.test(code) ? 'library_mutation_' + code : 'library_result_unconfirmed');
    } finally {
      selection.settle(confirmed, input.operation, input.name);
    }
  }

  function start(command) {
    let input;
    try { input = JSON.parse(command.value || '{}'); } catch (_) { return Promise.resolve(outcome(false, 'invalid_library_mutation')); }
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '') || !/^library_[a-f0-9]{32}$/.test(input?.fileHandle || '') ||
        !['rename', 'trash'].includes(input.operation) || input.operation === 'rename' && !validName(input.name)) {
      return Promise.resolve(outcome(false, 'invalid_library_mutation'));
    }
    if (command.selected !== true) return Promise.resolve(outcome(false, 'user_confirmation_required'));
    const fingerprint = JSON.stringify([input.fileHandle, input.operation, input.name || '']);
    const previous = receipts.get(command.requestId);
    if (previous) {
      if (!previous.current()) return Promise.resolve(outcome(false, 'library_request_context_changed'));
      return previous.fingerprint === fingerprint ? previous.promise : Promise.resolve(outcome(false, 'library_request_conflict'));
    }
    if (active) return Promise.resolve(outcome(false, 'library_mutation_busy'));
    if (Date.now() < retryAt && retryContext?.()) return Promise.resolve(outcome(false, 'library_mutation_cooldown'));
    const catalog = root.__elonChatGptPrivateLibraryCatalog;
    const selection = catalog?.selectMutation?.(input.fileHandle);
    if (!selection) return Promise.resolve(outcome(false, 'library_selection_expired'));
    const allowed = capabilities(selection.source);
    if (!(input.operation === 'rename' ? allowed.canRename : allowed.canTrash)) return Promise.resolve(outcome(false, 'library_mutation_unsupported'));
    if (!root.__elonChatGptPrivateJsonRequest) return Promise.resolve(outcome(false, 'library_mutation_unavailable'));
    const job = { fingerprint, current: selection.current };
    active = job;
    catalog.cancelActiveRead?.();
    job.promise = Promise.resolve().then(() => execute(input, selection))
      .catch(() => outcome(false, 'library_result_unconfirmed')).then(result => {
      if (!result.ok && job.current()) { retryAt = Date.now() + 45000; retryContext = job.current; }
      return result;
    }).finally(() => { if (active === job) active = null; });
    receipts.set(command.requestId, job);
    while (receipts.size > 128) receipts.delete(receipts.keys().next().value);
    return job.promise;
  }

  function handle(command, respond) {
    start(command).then(result => respond(ACTION, result.ok, result.code))
      .catch(() => respond(ACTION, false, 'library_result_unconfirmed'));
    return true;
  }
  return Object.freeze({ version: 2, capabilities, start, handle, busy: () => Boolean(active) });
});
