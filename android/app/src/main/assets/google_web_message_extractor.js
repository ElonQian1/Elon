(function () {
  'use strict';

  const extractorVersion = 16;
  if (window.__elonGoogleWebMessageExtractor &&
      window.__elonGoogleWebMessageExtractor.version === extractorVersion) return;

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const candidatePolicy = window.__elonGoogleWebAnswerCandidatePolicy;
  const queryPolicy = window.__elonGoogleWebQueryPolicy;
  const TRUSTED_ANSWER_SELECTORS = [
    '[data-sfc-cp][data-hveid]',
    '[id^="aim-chrome-initial-inline-async-container"]',
    '[data-container-id]',
    '[data-snhf]',
    '[data-attrid*="ai" i]',
    'article',
    '[role="article"]'
  ];
  const TRUSTED_ANSWER_SELECTOR = TRUSTED_ANSWER_SELECTORS.join(',');
  let rememberedQueryValue = '';
  let rememberedQueryOwned = false;

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function isVisible(node) {
    if (!(node instanceof Element)) return false;
    const rect = node.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0 || node.getClientRects().length === 0) return false;
    if (typeof node.checkVisibility === 'function' && !node.checkVisibility({
      checkOpacity: true,
      checkVisibilityCSS: true
    })) return false;
    for (let current = node; current && current instanceof Element; current = current.parentElement) {
      if (current.hidden || current.hasAttribute('inert') ||
          current.getAttribute('aria-hidden') === 'true') return false;
      const style = window.getComputedStyle(current);
      if (style.display === 'none' || style.visibility === 'hidden' ||
          style.visibility === 'collapse' || Number(style.opacity) === 0) return false;
    }
    return true;
  }

  function rememberQuery(value) {
    const query = cleanText(value).slice(0, 40000);
    if (!query) return;
    rememberedQueryValue = query;
    rememberedQueryOwned = true;
  }

  function clearRememberedQuery() {
    rememberedQueryValue = '';
    rememberedQueryOwned = false;
  }

  function rememberedQuery() {
    return rememberedQueryValue;
  }

  function currentQuery() {
    const urlQuery = cleanText(new URLSearchParams(location.search).get('q')).slice(0, 40000);
    const selectors = [
      'main [data-user-query]',
      'main [data-query]',
      'main [aria-label*="your question" i]',
      'main [aria-label*="您的问题" i]'
    ];
    const headings = Array.from(document.querySelectorAll(selectors.join(',')))
      .filter(isVisible)
      .map((node) => cleanText(node.innerText || node.textContent))
      .filter((text) => text.length > 1 && text.length <= 40000);
    const explicitQuery = headings[headings.length - 1] || '';
    const selected = queryPolicy && typeof queryPolicy.select === 'function'
      ? queryPolicy.select({
          explicitQuery,
          rememberedQuery: rememberedQuery(),
          rememberedOwned: rememberedQueryOwned,
          urlQuery
        })
      : explicitQuery || rememberedQuery() || urlQuery;
    if (selected && selected !== rememberedQueryValue) {
      rememberedQueryValue = selected;
      rememberedQueryOwned = false;
    }
    return selected;
  }

  function hasCurrentQuery() {
    return !!currentQuery();
  }

  function currentQueryMatches(value) {
    return cleanText(currentQuery()) === cleanText(value);
  }

  function citationParts(container) {
    if (!container) return [];
    const seen = new Set();
    const parts = [];
    for (const link of container.querySelectorAll('a[href^="https://"]')) {
      if (!isVisible(link)) continue;
      try {
        const url = new URL(link.href);
        if (allowedOrigins.has(url.origin)) continue;
        const safeUrl = url.origin + url.pathname;
        if (seen.has(safeUrl)) continue;
        seen.add(safeUrl);
        const title = cleanText(link.textContent || link.getAttribute('aria-label')) || url.hostname;
        parts.push({
          type: 'citation',
          text: title.slice(0, 160),
          title: title.slice(0, 160),
          url: safeUrl.slice(0, 1200),
          targetKind: 'external',
          targetHost: url.hostname.slice(0, 253)
        });
        if (parts.length >= 12) break;
      } catch (_) {}
    }
    return parts;
  }

  function containsComposer(node, composer) {
    return !!composer && (node === composer || node.contains(composer));
  }

  function excludedContainer(node, composer) {
    if (!isVisible(node) || containsComposer(node, composer)) return true;
    if (node.closest(
      'header, nav, footer, form, [role="navigation"], [role="dialog"], ' +
      '[role="tablist"], [role="toolbar"]'
    )) return true;
    return false;
  }

  function followsQuery(node, queryAnchor) {
    if (!queryAnchor || node === queryAnchor || typeof Node === 'undefined') return false;
    return !!(queryAnchor.compareDocumentPosition(node) & Node.DOCUMENT_POSITION_FOLLOWING);
  }

  function findQueryAnchor(query) {
    if (!query) return null;
    const nodes = uniqueNodes([
      'main [data-user-query]',
      'main [data-query]',
      'main [aria-label*="your question" i]',
      'main [aria-label*="您的问题" i]',
      'body div',
      'body span',
      'body p'
    ]).slice(0, 2400).filter((node) =>
      isVisible(node) && cleanText(node.innerText || node.textContent) === query
    );
    return nodes[nodes.length - 1] || null;
  }

  function candidateFrom(node, composer, query, queryAnchor, explicit) {
    if (excludedContainer(node, composer)) return null;
    let text = cleanText(node.innerText || node.textContent).slice(0, 40000);
    if (query && text.startsWith(query)) text = cleanText(text.slice(query.length));
    if (!text || text === query) return null;
    const citations = citationParts(node);
    const semanticBlocks = node.querySelectorAll('p, li, blockquote, pre, table, h2, h3').length;
    const controls = node.querySelectorAll('button, [role="button"], input, textarea').length;
    const links = node.querySelectorAll('a[href]').length;
    const tabControls = node.querySelectorAll('[role="tab"], [role="tablist"], [role="toolbar"]').length +
      (node.matches('[role="tab"], [role="tablist"], [role="toolbar"]') ? 1 : 0);
    const liveRegion = node.matches('[aria-live], [role="status"], [role="alert"]');
    const metrics = {
      hasQuery: !!query,
      text,
      textLength: text.length,
      citations: citations.length,
      semanticBlocks,
      links,
      tabControls,
      liveRegion,
      controls,
      afterQuery: followsQuery(node, queryAnchor),
      trustedAnswerContainer: node.matches(TRUSTED_ANSWER_SELECTOR),
      interactive: !!node.closest(
        'a[href], button, input, textarea, select, [role="button"], [role="link"], ' +
        '[role="menuitem"], [role="tab"]'
      ),
      explicit
    };
    if (candidatePolicy && !candidatePolicy.accepts(metrics)) return null;
    const depth = Math.min(12, node.closest('main, [role="main"]') ? 1 : 0);
    const score = Math.min(text.length, 8000) + citations.length * 900 +
      Math.min(semanticBlocks, 20) * 90 + (explicit ? 1400 : 0) + depth * 30 -
      Math.min(controls, 20) * 120 -
      (candidatePolicy ? candidatePolicy.penalty(metrics) : 0);
    return {
      node,
      text,
      citations,
      score,
      explicit,
      afterQuery: metrics.afterQuery,
      trustedAnswerContainer: metrics.trustedAnswerContainer
    };
  }

  function uniqueNodes(selectors) {
    const seen = new Set();
    const nodes = [];
    for (const node of document.querySelectorAll(selectors.join(','))) {
      if (!seen.has(node)) {
        seen.add(node);
        nodes.push(node);
      }
    }
    return nodes;
  }

  function answerCandidate(composer, query) {
    const queryAnchor = findQueryAnchor(query);
    const explicitSelectors = TRUSTED_ANSWER_SELECTORS.concat('[role="region"]');
    const semanticSelectors = [
      'body section',
      'body [aria-live="polite"]',
      'body [aria-live="assertive"]'
    ];
    const genericSelectors = ['body div'];
    const explicit = uniqueNodes(explicitSelectors)
      .map((node) => candidateFrom(node, composer, query, queryAnchor, true))
      .filter(Boolean);
    const semantic = uniqueNodes(semanticSelectors)
      .map((node) => candidateFrom(node, composer, query, queryAnchor, true))
      .filter(Boolean);
    const generic = uniqueNodes(genericSelectors).slice(0, 1200)
      .map((node) => candidateFrom(node, composer, query, queryAnchor, false))
      .filter(Boolean)
      .filter((candidate) => {
        if (candidate.text.length < 8) return true;
        const child = Array.from(candidate.node.children)
          .map((node) => candidateFrom(node, composer, query, queryAnchor, false))
          .filter(Boolean)
          .sort((left, right) => right.text.length - left.text.length)[0];
        return !child || child.text.length < candidate.text.length * 0.92;
      });
    const candidatesByNode = new Map();
    explicit.concat(semantic, generic).forEach((candidate) => {
      const current = candidatesByNode.get(candidate.node);
      if (!current || candidate.score > current.score) candidatesByNode.set(candidate.node, candidate);
    });
    const candidates = Array.from(candidatesByNode.values()).sort((left, right) => {
      if (left.node !== right.node && typeof left.node.compareDocumentPosition === 'function') {
        const position = left.node.compareDocumentPosition(right.node);
        if (position & 4) return -1;
        if (position & 2) return 1;
      }
      return 0;
    }).map((candidate, index) => ({
      ...candidate,
      domOrder: index,
      textLength: candidate.text.length
    }));
    if (candidatePolicy && typeof candidatePolicy.select === 'function') {
      return candidatePolicy.select(candidates);
    }
    return candidates.sort((left, right) =>
      right.domOrder - left.domOrder || right.score - left.score
    )[0] || null;
  }

  function extract(composer, streaming) {
    const query = currentQuery();
    const answer = query ? answerCandidate(composer, query) : null;
    const messages = [];
    if (query) messages.push({
      id: 'google-query-current',
      role: 'user',
      state: 'completed',
      content: [{ type: 'text', text: query }]
    });
    if (answer) {
      const content = answer.text ? [{ type: 'text', text: answer.text }] : [];
      content.push(...answer.citations);
      if (content.length) messages.push({
        id: 'google-answer-current',
        role: 'assistant',
        state: streaming ? 'streaming' : 'completed',
        content
      });
    }
    return { messages, queryFound: !!query, answerFound: !!answer };
  }

  function diagnostics(composer, extraction) {
    const mainCount = document.querySelectorAll('main, [role="main"]').length;
    const explicitCount = document.querySelectorAll(
      '[data-container-id], [data-snhf], article, [role="article"], [role="region"]'
    ).length;
    const semanticCount = document.querySelectorAll(
      'body section, body [aria-live="polite"], body [aria-live="assertive"]'
    ).length;
    const trustedCount = Array.from(document.querySelectorAll(TRUSTED_ANSWER_SELECTOR))
      .filter(isVisible).length;
    const responseRootCount = Array.from(document.querySelectorAll('[data-sfc-cp][data-hveid]'))
      .filter(isVisible).length;
    const divCount = Math.min(document.querySelectorAll('body div').length, 9999);
    return [
      'v=' + extractorVersion,
      'main=' + mainCount,
      'explicit=' + explicitCount,
      'semantic=' + semanticCount,
      'trusted=' + trustedCount,
      'roots=' + responseRootCount,
      'div=' + divCount,
      'composer=' + (composer ? 1 : 0),
      'query=' + (extraction.queryFound ? 1 : 0),
      'owned=' + (rememberedQueryOwned ? 1 : 0),
      'answer=' + (extraction.answerFound ? 1 : 0)
    ].join('|').slice(0, 160);
  }

  window.__elonGoogleWebMessageExtractor = Object.freeze({
    version: extractorVersion,
    rememberQuery,
    clearRememberedQuery,
    hasCurrentQuery,
    currentQueryMatches,
    extract,
    diagnostics
  });
})();
