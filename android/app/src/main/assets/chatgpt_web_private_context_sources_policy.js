(function (root, factory) {
  'use strict';
  const existing = root?.__elonChatGptPrivateContextSourcesPolicy;
  const exported = existing?.version >= 2 ? existing : factory();
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root) root.__elonChatGptPrivateContextSourcesPolicy = exported;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const overlays = new WeakMap(), approvals = new WeakMap();
  const MAX_ITEMS = 256, MAX_SEED = 262144;
  const object = value => value && typeof value === 'object' && !Array.isArray(value);
  const text = value => typeof value === 'string' ? value.trim().toLowerCase() : '';
  const hasUrl = item => typeof item.url === 'string';

  function isFile(item) {
    if (!object(item)) return false;
    return item.category != null ? text(item.category) === 'files' : item.type === 'file' ||
      ['files', 'library'].includes(text(item.attribution)) || text(item.url).startsWith('file://');
  }

  function key(item) {
    const context = item.type === 'conversation_context_citation';
    const past = context ? item.conversation_context_type === 'past_conversation' :
      typeof item.attribution === 'string' && item.attribution.length ? item.attribution === 'Past chat' :
        hasUrl(item) && /(?:^|\/)c\/([^/?#]+)/.test(item.url);
    if (context && item.conversation_context_type !== 'general_memory') {
      if (Number.isFinite(item.index)) return 'context:' + item.conversation_context_type + ':' + item.index;
      const match = typeof item.matched_text === 'string' && /^\ue200memcite\ue202(UM|GM|PC)\ue202([^\ue201]+)\ue201$/.exec(item.matched_text);
      const index = match && Number(match[2]);
      if (Number.isInteger(index) && index >= 1) return 'context:' +
        ({ UM: 'user_memory', GM: 'general_memory', PC: 'past_conversation' })[match[1]] + ':' + (index - 1);
    } else if (!context && past && Number.isFinite(item.index)) {
      return 'context:past_conversation:' + item.index;
    }
    if (typeof item.citation_uuid === 'string' && item.citation_uuid) return 'uuid:' + item.citation_uuid;
    if (past && hasUrl(item)) {
      try { const url = new URL(item.url, 'https://chatgpt.com');
        return 'past:' + url.pathname + url.search + url.hash; } catch (_) {}
    }
    return null;
  }

  function merge(items, item) {
    const identity = key(item), at = identity == null ? -1 : items.findIndex(value => key(value) === identity);
    const next = items.slice();
    if (at >= 0) next[at] = item;
    else next.push(item);
    if (next.length > MAX_ITEMS) throw new Error('context_sources_too_large');
    return next;
  }

  function seed(metadata) {
    const graph = metadata?.conversation_context_citation_metadata;
    if (graph == null) return null;
    if (!Array.isArray(graph) || graph.length > MAX_ITEMS) throw new Error('context_sources_seed');
    let items = [];
    for (const entry of graph) {
      if (!object(entry?.citation)) continue;
      const item = typeof entry.retrieval_origin === 'string'
        ? { ...entry.citation, retrieval_origin: entry.retrieval_origin } : entry.citation;
      items = merge(items, item);
    }
    return graph.length && !items.length ? null : items;
  }

  function inspect(metadata) {
    if (!object(metadata)) return null;
    const graph = metadata.conversation_context_citation_metadata;
    const status = metadata.conversation_context_citation_metadata_status;
    if (status == null && (graph == null || Array.isArray(graph) && !graph.length)) return null;
    try {
      if (graph != null && (!Array.isArray(graph) || graph.length > MAX_ITEMS)) throw new Error('context_sources_seed');
      const fingerprint = JSON.stringify([status ?? null, graph ?? null]);
      if (fingerprint.length > MAX_SEED) throw new Error('context_sources_seed');
      const items = seed(metadata);
      const supported = status == null || ['complete', 'complete_inline_only', 'seeded', 'marker',
        'pending_inline_finalize'].includes(status);
      return { fingerprint, items, status, fetch: supported && status !== 'complete' &&
        status !== 'pending_inline_finalize', expand: status === 'complete_inline_only', supported };
    } catch (_) { return { fingerprint: '', items: null, status, fetch: false, supported: false }; }
  }

  function withoutDeleted(metadata, items) {
    const graph = metadata?.conversation_context_citation_metadata;
    if (!Array.isArray(graph)) return items;
    // Official AKn removes by the outer record's exact URL, even when a later
    // supplemental item has a different UUID or an allowed PCA mask.
    const deleted = new Set(graph.flatMap(entry => entry?.deleted === true &&
      object(entry.citation) && hasUrl(entry.citation) ? [entry.citation.url] : []));
    return items.filter(item => !hasUrl(item) || !deleted.has(item.url));
  }

  function validateMask(mask) {
    if (!object(mask) || !['pending', 'complete', 'failed'].includes(mask.status) ||
        !Array.isArray(mask.urls) || mask.urls.length > MAX_ITEMS ||
        mask.urls.some(url => typeof url !== 'string' || url.length > 8192) ||
        mask.filtered_source_types != null && (!Array.isArray(mask.filtered_source_types) ||
          mask.filtered_source_types.length > 32 || mask.filtered_source_types.some(type => typeof type !== 'string'))) {
      throw new Error('context_sources_mask');
    }
    return { status: mask.status, urls: mask.urls.slice(),
      ...(mask.filtered_source_types ? { filtered_source_types: mask.filtered_source_types.slice() } : {}) };
  }

  function masked(item, mask) {
    if (!mask || item.retrieval_origin !== 'pca' || text(item.source_type) === 'gmail_dream') return false;
    const types = new Set(mask.filtered_source_types ?? ['convos', 'files', 'gmail']);
    // Only file rows can receive download approval. Other source families remain
    // in the selection counts, but never become file or scope identities.
    const memory = item.category != null ? text(item.category) === 'memory' :
      item.type === 'conversation_context_citation' && item.conversation_context_type
        ? ['user_instructions', 'past_conversation', 'user_memory', 'general_memory'].includes(item.conversation_context_type)
        : text(item.attribution) === 'past chat';
    const filtered = types.has('files') && isFile(item) || types.has('convos') && memory ||
      types.has('gmail') && text(item.attribution) === 'gmail';
    return filtered && !mask.urls.some(url => url.trim() === String(item.url || '').trim());
  }

  function withMask(state, mask) {
    if (!mask || mask === state.mask) return state;
    let items = state.items;
    if (!state.mask || state.mask.status === 'pending') {
      const pending = { ...mask, urls: [] }, before = [], revealed = [];
      for (const item of items) {
        (hasUrl(item) && masked(item, pending) && !masked(item, mask) ? revealed : before).push(item);
      }
      items = before.concat(revealed);
    }
    return { ...state, items, mask };
  }

  function decode(body, createDecoder) {
    if (typeof body !== 'string' || body.length > 1048576 || typeof createDecoder !== 'function') {
      throw new Error('context_sources_stream');
    }
    let state = { status: 'loading', items: [], mask: null, error: false }, count = 0, failure;
    const decoder = createDecoder(event => {
      if (!object(event)) return;
      if (event.type === 'conversation_context_source') {
        if (!object(event.item)) throw new Error('context_sources_item');
        state = { ...state, items: merge(state.items, event.item) };
      } else if (event.type === 'pca_source_filter_mask') {
        state = withMask(state, validateMask(event.mask));
      } else if (event.type === 'status') {
        if (!['idle', 'loading', 'done'].includes(event.status)) throw new Error('context_sources_status');
        state = { ...state, status: event.status };
      } else return;
      if (++count > 4096) throw new Error('context_sources_too_large');
    }, error => { if (error) failure = error; }, { strict: true });
    decoder.push(body);
    decoder.finish();
    if (failure || !count) throw new Error('context_sources_stream');
    // The official reader also completes at clean EOF, not only a done event.
    return { ...state, status: 'done' };
  }

  const populated = state => state.items.some(item => !hasUrl(item) || !masked(item, state.mask));
  function select(metadata, supplement) {
    const info = inspect(metadata);
    if (!info?.supported) return { items: [], partial: true };
    const fetched = supplement?.status === 'done' && !supplement.error;
    let selected = supplement || { status: 'loading', items: [], mask: null };
    const inline = info.items == null ? null : { items: info.items, mask: null,
      status: info.status === 'complete' ? 'done' : 'loading' };
    if (inline) {
      const maskedInline = withMask(inline, selected.mask);
      if (metadata.conversation_context_citation_metadata?.length && fetched && !populated(selected)) {
        selected = ['complete', 'failed'].includes(selected.mask?.status) && !populated(maskedInline)
          ? selected : maskedInline;
      } else if (info.status === 'seeded' && selected.items.length) {
        selected = { ...selected, items: selected.items.reduce(merge, inline.items) };
      } else if (inline.status === 'loading' && fetched && inline.items.length <= selected.items.length) {
        // Completed supplemental graph supersedes the partial inline graph.
      } else if (info.status === 'seeded' ? selected.items.length === 0 :
          info.status === 'complete_inline_only' ? inline.items.length > selected.items.length :
            inline.items.length >= selected.items.length) selected = maskedInline;
    }
    const complete = selected.status === 'done' && selected.mask?.status !== 'pending';
    const items = selected.items.filter(item => !masked(item, selected.mask) &&
      (complete || item.retrieval_origin !== 'pca'));
    return { items: withoutDeleted(metadata, items), partial: !complete };
  }

  function resolve(metadata, supplement) {
    try { return select(metadata, supplement); }
    catch (_) { return { items: [], partial: true }; }
  }

  function attach(metadata, result, current) {
    const record = { items: result.items.map(item => Object.freeze({ ...item })),
      partial: result.partial, current };
    overlays.set(metadata, record);
    for (const item of record.items) approvals.set(item, () => overlays.get(metadata) === record && current());
  }
  function lookup(metadata) {
    const record = object(metadata) && overlays.get(metadata);
    return record?.current() ? record : null;
  }
  function allows(item) { try { return approvals.get(item)?.() === true; } catch (_) { return false; } }

  return Object.freeze({ version: 2, inspect, resolve, decode, attach, lookup, allows, isFile, withoutDeleted });
});
