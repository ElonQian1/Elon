(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateTasks = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const policy = options.policy || page.__elonChatGptPrivateTasksPolicy;
  const identity = (options.contract || page.__elonChatGptPrivateConversationShareContract).create(page).identity;
  const now = options.now || Date.now;
  const cache = new Map(), pending = new Map(), failures = new Map();
  let owner = null, epoch = 0;
  function invalidate() { epoch++; owner = null; cache.clear(); pending.clear(); failures.clear(); }
  function current(binding) {
    try {
      return binding.epoch === epoch && page.location.origin === 'https://chatgpt.com' &&
        page.document === binding.document && page.__elonChatGptDocumentToken === binding.token &&
        identity() === binding.account;
    } catch (_) { return false; }
  }
  function bind() {
    const binding = { document: page.document, token: page.__elonChatGptDocumentToken, account: identity(), epoch };
    if (page.location.origin !== 'https://chatgpt.com' || !binding.account ||
        !/^doc_[a-z0-9_]{3,80}$/.test(binding.token || '')) {
      invalidate(); throw Error('tasks_auth_unavailable');
    }
    if (owner && !current(owner)) { invalidate(); binding.epoch = epoch; }
    owner = binding;
    return binding;
  }
  function selection(input) {
    if (!input || typeof input !== 'object' || Array.isArray(input) || typeof input.force !== 'boolean') throw Error('tasks_request_invalid');
    let path, key;
    if (input.operation === 'list') {
      if (Object.keys(input).some(k => !['operation', 'filter', 'cursor', 'force'].includes(k)) || !policy.filters.includes(input.filter)) throw Error('tasks_request_invalid');
      let cursor;
      try { cursor = policy.cursor(input.cursor); } catch (_) { throw Error('tasks_request_invalid'); }
      const query = new URLSearchParams({ filter: input.filter });
      if (cursor !== null) query.set('cursor', cursor);
      path = '/backend-api/automations?' + query;
      key = 'list:' + input.filter + ':' + (cursor || '');
    } else if (input.operation === 'latest') {
      if (Object.keys(input).some(k => !['operation', 'id', 'force'].includes(k)) || !policy.validId(input.id)) throw Error('tasks_request_invalid');
      path = '/backend-api/automation/' + encodeURIComponent(input.id) + '/latest_backing_run?include_snapshot=true';
      key = 'latest:' + input.id;
    } else throw Error('tasks_request_invalid');
    return { key, path, operation: input.operation };
  }
  function bounded(map, key, value) {
    map.delete(key); map.set(key, value);
    while (map.size > 12) map.delete(map.keys().next().value);
  }
  function result(entry, cached) {
    // Consumers cannot mutate the in-memory authoritative copy.
    return { ok: true, code: 'tasks_ready', cached, fetchedAt: entry.at, data: JSON.parse(JSON.stringify(entry.data)) };
  }
  async function fetchValue(binding, selected) {
    if (!current(binding)) throw Error('tasks_context_changed');
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders())) {
      if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
        'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
    }
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, selected.path, {
      method: 'GET', headers, credentials: 'include', cache: 'no-store', redirect: 'error',
      __elonPrivateTransport: 'scheduled_tasks_v1',
    }, { timeoutMs: 7000, maxBytes: 1024 * 1024, mode: 'json' });
    if (!current(binding)) throw Error('tasks_context_changed');
    const data = selected.operation === 'list' ? policy.page(response.payload) : policy.latest(response.payload);
    const entry = { binding, data, at: now() };
    bounded(cache, selected.key, entry); failures.delete(selected.key);
    return result(entry, false);
  }
  async function run(input) {
    let binding, selected;
    try {
      selected = selection(input); binding = bind();
      const existing = cache.get(selected.key), age = existing ? now() - existing.at : -1;
      if (!input.force && existing && current(existing.binding) && age >= 0 && age < 60000) return result(existing, true);
      const failure = failures.get(selected.key);
      if (failure && now() >= failure.at && (failure.code === 'tasks_rate_limited'
          ? now() - failure.at < 60000 : !input.force && now() - failure.at < 15000)) {
        return { ok: false, code: failure.code, ...(existing && current(existing.binding)
          ? { stale: JSON.parse(JSON.stringify(existing.data)), fetchedAt: existing.at } : {}) };
      }
      let flight = pending.get(selected.key);
      if (!flight || !current(flight.binding)) {
        if (pending.size >= 4) throw Error('tasks_busy');
        flight = { binding, promise: null };
        flight.promise = fetchValue(binding, selected).finally(() => {
          if (pending.get(selected.key) === flight) pending.delete(selected.key);
        });
        pending.set(selected.key, flight);
      }
      const outcome = await flight.promise;
      if (!current(binding)) throw Error('tasks_context_changed');
      return JSON.parse(JSON.stringify(outcome));
    } catch (error) {
      const raw = String(error?.message || ''), changed = binding && !current(binding);
      const code = changed ? 'tasks_context_changed' : /^http_(401|403)$/.test(raw) ? 'tasks_auth_required' :
        raw === 'http_404' ? 'tasks_not_found' : raw === 'http_429' ? 'tasks_rate_limited' :
        /^tasks_[a-z_]+$/.test(raw) ? raw : 'tasks_unavailable';
      if (changed) return { ok: false, code };
      if (code === 'tasks_auth_required') {
        invalidate(); page.__elonChatGptPrivateAuthContext?.invalidate?.('scheduled_tasks_rejected');
        return { ok: false, code };
      }
      if (binding && selected) bounded(failures, selected.key, { code, at: now() });
      const previous = selected && cache.get(selected.key);
      return { ok: false, code, ...(previous && current(previous.binding) ? {
        stale: JSON.parse(JSON.stringify(previous.data)), fetchedAt: previous.at,
      } : {}) };
    }
  }
  return Object.freeze({ version: 1, run, invalidate });
});
