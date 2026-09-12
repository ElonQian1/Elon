(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateWritingBlocks) {
    root.__elonChatGptPrivateWritingBlocks = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const policy = page.__elonChatGptWritingBlockPolicy, parser = page.__elonChatGptTextBlocks;
  const context = options.context || page.__elonChatGptWritingBlockContext.create(page);
  const entries = new Map(), receipts = new Map(), now = options.now || Date.now;
  let active = false;
  const fail = code => { throw Error('writing_' + code); };
  const result = (entry, code) => ({ ok: true, code, ticket: entry.ticket, path: entry.binding.path,
    id: entry.source.id, messageId: entry.source.messageId, pending: !!entry.pending });
  function valid(entry) {
    if (!entry || !entry.binding.current() || now() < entry.at || now() - entry.at > 1800000) fail('selection_expired');
  }
  async function request(entry, method, body, deadline, dispatch) {
    valid(entry);
    const remaining = Math.min(8000, deadline - now());
    if (remaining <= 0) fail('timeout');
    const headers = { ...page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), Accept: 'application/json' };
    const init = { method, headers, credentials: 'same-origin', cache: 'no-store', redirect: 'error',
      __elonPrivateTransport: 'writing_blocks_v1' };
    if (body) { headers['Content-Type'] = 'application/json'; init.body = JSON.stringify(body); }
    const path = body ? '/backend-api/conversation/message/writing-blocks' : '/backend-api/conversation/' + entry.binding.id;
    dispatch?.();
    const value = await page.__elonChatGptPrivateJsonRequest.request(page, path, init,
      { timeoutMs: remaining, maxBytes: body ? 65536 : 4 * 1024 * 1024, mode: body ? 'none' : 'json' });
    valid(entry);
    return value.payload;
  }
  async function read(entry, deadline) {
    const payload = await request(entry, 'GET', null, deadline);
    return policy.source(payload, entry.binding.id, entry.source.messageId, entry.source.id, parser);
  }
  async function verify(entry, deadline) {
    const expected = entry.pending;
    if (!expected) return result(entry, 'writing_ready');
    const fresh = await read(entry, deadline);
    if (!policy.same(fresh, expected)) return result(entry, 'writing_write_unconfirmed');
    const synchronized = await entry.binding.reconcile(fresh, deadline);
    if (!synchronized) return result(entry, 'writing_saved_sync_pending');
    entry.source = fresh;
    entry.pending = null;
    entry.at = now();
    return result(entry, 'writing_saved');
  }
  async function run(input, confirmed, snapshot) {
    if (active) return { ok: false, code: 'writing_busy' };
    active = true;
    let entry;
    try {
      input = policy.parse(JSON.stringify(input));
      const deadline = now() + 24000;
      if (input.operation === 'prepare') {
        const binding = await context.capture(input, snapshot);
        for (const [key, value] of entries) if (!value.binding.current()) entries.delete(key);
        const existing = [...entries.values()].find(value => value.binding.id === binding.id &&
          value.source.messageId === input.messageId && value.source.id === input.id);
        if (existing?.pending) return result(existing, 'writing_write_unconfirmed');
        const bytes = new Uint8Array(16);
        page.crypto.getRandomValues(bytes);
        entry = { ticket: 'wb_' + Array.from(bytes, value => value.toString(16).padStart(2, '0')).join(''),
          binding, source: { id: input.id, messageId: input.messageId }, at: now(), pending: null };
        const fresh = await read(entry, deadline);
        if (fresh.content !== input.content) fail('version_conflict');
        binding.local(fresh);
        entry.source = fresh;
        if (existing) entries.delete(existing.ticket);
        if (entries.size >= 16) fail('selection_limit');
        entries.set(entry.ticket, entry);
        return result(entry, 'writing_ready');
      }
      entry = entries.get(input.ticket);
      valid(entry);
      if (entry.binding.path !== input.path) fail('context_changed');
      if (input.operation === 'verify') return await verify(entry, deadline);
      if (!confirmed) fail('confirmation_required');
      if (page.__elonChatGptPrivateConversationMutationsEnabled !== true) fail('disabled');
      if (entry.pending) return result(entry, 'writing_write_unconfirmed');
      const fresh = await read(entry, deadline);
      if (!policy.same(fresh, entry.source)) fail('version_conflict');
      entry.binding.local(fresh);
      if (fresh.content === input.content) return result(entry, 'writing_saved');
      const expected = { ...fresh, content: input.content };
      try {
        await request(entry, 'POST', policy.body(entry.binding.id, fresh, input.content, new Date(now()).toISOString()),
          deadline, () => { entry.pending = expected; });
      } catch (error) {
        // A rejected HTTP response is known; a timeout/transport error can still have committed.
        if (/^http_(400|401|403|404|409|422|429)$/.test(error?.message || '')) { entry.pending = null; throw error; }
        throw error;
      }
      return await verify(entry, deadline);
    } catch (error) {
      if (entry?.pending) return result(entry, 'writing_write_unconfirmed');
      const reason = String(error?.message || '');
      return { ok: false, code: /^writing_[a-z_]{1,64}$/.test(reason) ? reason :
        /^http_\d{3}$/.test(reason) ? 'writing_' + reason : 'writing_unavailable' };
    } finally { active = false; }
  }
  function handle(action, command, respond, snapshot, emit) {
    if (action !== 'writing_block') return false;
    let input;
    try {
      input = policy.parse(command.value);
      if (!/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '')) fail('request_invalid');
      if (input.operation === 'save' && command.selected !== true) fail('confirmation_required');
    } catch (_) { respond(action, false, 'writing_request_invalid'); return true; }
    const fingerprint = JSON.stringify([input, command.selected === true]);
    let job = receipts.get(command.requestId);
    if (job && job.fingerprint !== fingerprint) { respond(action, false, 'writing_request_conflict'); return true; }
    if (!job) {
      job = { fingerprint, promise: run(input, command.selected === true, snapshot) };
      receipts.set(command.requestId, job);
      while (receipts.size > 64) receipts.delete(receipts.keys().next().value);
    }
    job.promise.then(value => {
      if (value.ok) emit({ type: 'writing_block', version: 1, requestId: command.requestId,
        path: value.path, ticket: value.ticket, id: value.id, messageId: value.messageId, pending: value.pending });
      respond(action, value.ok, value.code);
    }).catch(() => respond(action, false, 'writing_unavailable'));
    return true;
  }
  return Object.freeze({ version: 1, handle, run });
});
