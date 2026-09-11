(function (root, factory) {
  'use strict';
  const policy = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_private_context_sources_policy.js') : root?.__elonChatGptPrivateContextSourcesPolicy;
  if (typeof module === 'object' && module.exports) module.exports = { version: 1, create: host => factory(host, policy) };
  if (root && (!root.__elonChatGptPrivateContextSources || root.__elonChatGptPrivateContextSources.version < 1)) {
    root.__elonChatGptPrivateContextSources = factory(root, policy);
  }
})(typeof window === 'object' ? window : null, function (root, policy) {
  'use strict';
  const ID = /^[A-Za-z0-9_-]{1,180}$/;
  const MAX_BATCH = 8, CAPACITY = 32, TTL_MS = 120000, BUDGET_MS = 4000;
  const cache = new Map(), flights = new Map();
  let scope = '', epoch = 0;
  const now = () => Date.now();

  function identity(headers) {
    const values = Object.fromEntries(Object.entries(headers || {}).map(([key, value]) => [key.toLowerCase(), value]));
    if (!/^Bearer\s+\S{8,65536}$/i.test(values.authorization || '')) return '';
    return JSON.stringify([values.authorization, values['chatgpt-account-id'] || '', values['oai-device-id'] || '']);
  }

  function owner(headers) {
    const account = identity(headers), href = root.location?.href, doc = root.__elonChatGptDocumentToken;
    if (!account || root.location?.origin !== 'https://chatgpt.com' ||
        !/^doc_[A-Za-z0-9_-]{1,160}$/.test(doc || '') || !href) return null;
    const key = JSON.stringify([account, href, doc]);
    if (scope !== key) { scope = key; epoch++; cache.clear(); }
    const generation = epoch;
    return { generation, current: () => {
      try { return generation === epoch && root.location?.href === href &&
        root.__elonChatGptDocumentToken === doc && identity(
          root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.()) === account; }
      catch (_) { return false; }
    } };
  }

  function candidates(payload) {
    const projection = root.__elonChatGptPrivateHistoryProjection?.create({});
    return (projection?.sourceMessages(payload) || []).flatMap(({ id, message }) => {
      const info = policy?.inspect(message.metadata);
      return info ? [{ id, metadata: message.metadata, info }] : [];
    });
  }

  function bind(entry, supplement, token, until) {
    policy.attach(entry.metadata, policy.resolve(entry.metadata, supplement), () => token.current() &&
      now() < until && policy.inspect(entry.metadata)?.fingerprint === entry.info.fingerprint);
  }

  function cached(entry, conversationId, token) {
    const key = conversationId + ':' + entry.id, record = cache.get(key);
    if (record && record.until > now() && record.fingerprint === entry.info.fingerprint && token.current()) {
      cache.delete(key); cache.set(key, record);
      bind(entry, record.state, token, record.until);
      return true;
    }
    if (record) cache.delete(key);
    return false;
  }

  function applyCached(payload, conversationId) {
    const token = owner(root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.());
    if (!token || !ID.test(conversationId)) return;
    for (const entry of candidates(payload)) {
      if (!cached(entry, conversationId, token)) bind(entry, null, token, now() + TTL_MS);
    }
  }

  async function load(entry, conversationId, token, headers, deadline) {
    const key = conversationId + ':' + entry.id;
    const flightKey = token.generation + ':' + key + ':' + entry.info.fingerprint;
    let request = flights.get(flightKey);
    if (!request) {
      if (flights.size >= MAX_BATCH || now() >= deadline) return false;
      request = (async () => {
        const response = await root.__elonChatGptPrivateJsonRequest.request(root,
          '/backend-api/sidebar/conversation_context_sources', {
            method: 'POST', credentials: 'include', cache: 'no-store',
            headers: { ...headers, 'Content-Type': 'application/json', Accept: 'text/event-stream' },
            body: JSON.stringify({ conversation_id: conversationId, message_id: entry.id,
              expand_partial_inline: entry.info.expand === true })
          }, { mode: 'text', timeoutMs: Math.max(1, deadline - now()), maxBytes: 1048576 });
        if (!token.current()) throw new Error('context_sources_stale');
        const state = policy.decode(response.text, root.__elonChatGptPrivateStreamPolicy?.createSseDecoder);
        const record = { state, fingerprint: entry.info.fingerprint, until: now() + TTL_MS };
        cache.delete(key); cache.set(key, record);
        while (cache.size > CAPACITY) cache.delete(cache.keys().next().value);
        return record;
      })().finally(() => { if (flights.get(flightKey) === request) flights.delete(flightKey); });
      flights.set(flightKey, request);
    }
    const record = await request;
    if (!token.current() || policy.inspect(entry.metadata)?.fingerprint !== entry.info.fingerprint) return false;
    bind(entry, record.state, token, record.until);
    return true;
  }

  async function enrich(payload, conversationId, expectedOwner) {
    const entries = candidates(payload);
    if (!entries.length) return { stale: false, partial: false };
    if (!ID.test(conversationId)) return { stale: false, partial: true };
    if (expectedOwner && !expectedOwner.current()) return { stale: true, partial: true };
    const deadline = now() + BUDGET_MS;
    const headers = root.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const token = owner(headers);
    if (!token) return { stale: false, partial: true };
    let partial = false;
    const queue = [];
    for (const entry of entries.slice().reverse()) {
      if (cached(entry, conversationId, token)) continue;
      bind(entry, null, token, now() + TTL_MS);
      if (entry.info.fetch && ID.test(entry.id)) queue.push(entry);
      else if (policy.resolve(entry.metadata).partial) partial = true;
    }
    if (queue.length > MAX_BATCH) partial = true;
    const selected = queue.slice(0, MAX_BATCH);
    async function worker() {
      while (selected.length && token.current()) {
        const entry = selected.shift();
        try { if (!await load(entry, conversationId, token, headers, deadline)) partial = true; }
        catch (_) { partial = true; }
      }
    }
    // Two bounded readers only on an explicit native inventory request. No idle
    // polling, persisted tokens, DOM fallback, or user-message write is involved.
    await Promise.all([worker(), worker()]);
    return { stale: !token.current(), partial };
  }

  return Object.freeze({ version: 1, capture: owner,
    hasSources: payload => candidates(payload).length > 0, applyCached, enrich });
});
