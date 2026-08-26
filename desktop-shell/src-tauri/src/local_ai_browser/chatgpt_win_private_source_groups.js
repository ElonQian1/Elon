(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (!root) return;
  const policy = root.__elonChatGptPrivateStreamPolicy;
  if (!policy || typeof policy.createSession !== 'function') return;
  if (policy.__elonWinPrivateSourceGroupsWrapped === true) return;
  const enhanced = Object.freeze(api.enhancePolicy(policy));
  root.__elonChatGptPrivateStreamPolicy = enhanced;
  root.__elonChatGptWinPrivateSourceGroupsInstalled = Object.freeze({
    version: 2,
    basePolicy: policy,
    policy: enhanced,
  });
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const MAX_SOURCES = 12;
  const MAX_TITLE_LENGTH = 180;
  const MAX_SNIPPET_LENGTH = 320;

  function cleanText(value, maximum) {
    return String(value || '')
      .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, maximum);
  }

  function publicHttpsUrl(value) {
    try {
      const url = new URL(String(value || ''));
      if (url.protocol !== 'https:' || url.username || url.password ||
          (url.port && url.port !== '443')) return '';
      url.search = '';
      url.hash = '';
      return url.toString();
    } catch (_) {
      return '';
    }
  }

  function sourceHost(value) {
    try { return new URL(value).hostname.toLowerCase().replace(/^www\./, ''); }
    catch (_) { return ''; }
  }

  function compactEnvelope(payload) {
    if (!payload || typeof payload !== 'object') return null;
    return Number.isFinite(payload.c) && payload.v && typeof payload.v === 'object'
      ? payload.v
      : payload;
  }

  function visibleMessage(payload) {
    const envelope = compactEnvelope(payload);
    if (!envelope || typeof envelope !== 'object') return null;
    const message = envelope.message || envelope.data && envelope.data.message;
    if (!message || typeof message !== 'object') return null;
    return { envelope, message };
  }

  function sourceCitations(payload) {
    const visible = visibleMessage(payload);
    const metadata = visible && visible.message && visible.message.metadata;
    const groups = metadata && Array.isArray(metadata.search_result_groups)
      ? metadata.search_result_groups
      : [];
    const icons = metadata && Array.isArray(metadata.tool_icons)
      ? metadata.tool_icons
      : [];
    const citations = [];
    const seen = new Set();
    groups.slice(0, MAX_SOURCES).forEach((group, groupIndex) => {
      const entries = group && Array.isArray(group.entries) ? group.entries : [];
      const iconUrl = publicHttpsUrl(icons[groupIndex]);
      entries.slice(0, MAX_SOURCES).forEach((entry) => {
        if (citations.length >= MAX_SOURCES || !entry || typeof entry !== 'object') return;
        const url = publicHttpsUrl(entry.url);
        if (!url || seen.has(url)) return;
        const title = cleanText(
          entry.title || entry.attribution || group.domain || sourceHost(url),
          MAX_TITLE_LENGTH
        );
        if (!title) return;
        seen.add(url);
        const citation = {
          type: 'citation',
          text: title,
          url,
          markerText: cleanText(entry.attribution || group.domain || sourceHost(url), 80) || title,
          citationId: 'private_source_' + (citations.length + 1),
          groupSize: 1,
          targetHost: sourceHost(url)
        };
        const snippet = cleanText(entry.snippet, MAX_SNIPPET_LENGTH);
        const thumbnailUrl = publicHttpsUrl(entry.thumbnail_url);
        if (iconUrl) citation.iconUrl = iconUrl;
        if (snippet) citation.snippet = snippet;
        if (thumbnailUrl) citation.thumbnailUrl = thumbnailUrl;
        citations.push(citation);
      });
    });
    return {
      conversationId: String(
        visible && (visible.envelope.conversation_id || visible.envelope.conversationId) || ''
      ).slice(0, 180),
      turnId: String(metadata && (
        metadata.turn_exchange_id || metadata.working_turn_id
      ) || '').slice(0, 180),
      citations
    };
  }

  function mergeCitations(primary, supporting) {
    const merged = [];
    const indexes = new Map();
    [supporting, primary].forEach((parts) => {
      (Array.isArray(parts) ? parts : []).forEach((part) => {
        if (!part || part.type !== 'citation' || !part.url || merged.length >= MAX_SOURCES) return;
        const url = publicHttpsUrl(part.url);
        if (!url) return;
        const known = indexes.get(url);
        if (known === undefined) {
          indexes.set(url, merged.length);
          merged.push(Object.assign({}, part, { url }));
          return;
        }
        const current = merged[known];
        merged[known] = Object.assign({}, part, current, {
          text: cleanText(current.text, MAX_TITLE_LENGTH).length >= cleanText(part.text, MAX_TITLE_LENGTH).length
            ? current.text
            : part.text,
          iconUrl: current.iconUrl || part.iconUrl,
          snippet: current.snippet || part.snippet,
          thumbnailUrl: current.thumbnailUrl || part.thumbnailUrl,
          groupSize: Math.max(Number(current.groupSize) || 1, Number(part.groupSize) || 1)
        });
      });
    });
    return merged;
  }

  function enhanceSession(policy, session) {
    let sources = [];
    let sourceConversationId = '';
    let sourceTurnId = '';

    function clearSources() {
      sources = [];
      sourceConversationId = '';
      sourceTurnId = '';
    }

    function begin() {
      clearSources();
      return session.begin();
    }

    function accept(payload) {
      const accepted = session.accept(payload);
      const extracted = sourceCitations(payload);
      if (extracted.citations.length) {
        sources = extracted.citations;
        sourceConversationId = extracted.conversationId;
        sourceTurnId = extracted.turnId;
      }
      return accepted || extracted.citations.length > 0;
    }

    function current(pathname) {
      const active = session.current(pathname);
      if (!active) return null;
      if (active.conversationId && sourceConversationId &&
          active.conversationId !== sourceConversationId) return active;
      if (active.turnId && sourceTurnId && active.turnId !== sourceTurnId) return active;
      return Object.assign({}, active, {
        citations: mergeCitations(active.citations, sources)
      });
    }

    function merge(values, pathname) {
      const active = current(pathname);
      return active ? policy.mergeMessages(values, active) : values;
    }

    function reset() {
      clearSources();
      return session.reset();
    }

    return Object.freeze(Object.assign({}, session, { begin, accept, current, merge, reset }));
  }

  function enhancePolicy(policy) {
    if (!policy || typeof policy.createSession !== 'function' ||
        typeof policy.mergeMessages !== 'function') return policy;
    if (policy.__elonWinPrivateSourceGroupsWrapped === true) return policy;
    return Object.assign({}, policy, {
      __elonWinPrivateSourceGroupsWrapped: true,
      createSession(options) {
        return enhanceSession(policy, policy.createSession(options || {}));
      }
    });
  }

  return Object.freeze({ enhancePolicy, mergeCitations, sourceCitations });
});
