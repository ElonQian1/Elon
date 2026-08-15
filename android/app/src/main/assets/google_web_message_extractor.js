(function () {
  'use strict';

  const extractorVersion = 6;
  if (window.__elonGoogleWebMessageExtractor &&
      window.__elonGoogleWebMessageExtractor.version === extractorVersion) return;

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const candidatePolicy = window.__elonGoogleWebAnswerCandidatePolicy;
  const queryPolicy = window.__elonGoogleWebQueryPolicy;
  let rememberedQueryValue = '';

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
  }

  function clearRememberedQuery() {
    rememberedQueryValue = '';
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
      ? queryPolicy.select({ explicitQuery, rememberedQuery: rememberedQuery(), urlQuery })
      : explicitQuery || rememberedQuery() || urlQuery;
    if (selected) rememberQuery(selected);
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

  function candidateFrom(node, composer, query, explicit) {
    if (excludedContainer(node, composer)) return null;
    let text = cleanText(node.innerText || node.textContent).slice(0, 40000);
    if (query && text.startsWith(query)) text = cleanText(text.slice(query.length));
    if (text.length < 8 || text === query) return null;
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
      explicit
    };
    if (candidatePolicy && !candidatePolicy.accepts(metrics)) return null;
    const depth = Math.min(12, node.closest('main, [role="main"]') ? 1 : 0);
    const score = Math.min(text.length, 8000) + citations.length * 900 +
      Math.min(semanticBlocks, 20) * 90 + (explicit ? 1400 : 0) + depth * 30 -
      Math.min(controls, 20) * 120 -
      (candidatePolicy ? candidatePolicy.penalty(metrics) : 0);
    return { node, text, citations, score, explicit };
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
    const explicitSelectors = [
      '[data-container-id]',
      '[data-snhf]',
      '[data-attrid*="ai" i]',
      'article',
      '[role="article"]',
      '[role="region"]'
    ];
    const semanticSelectors = [
      'body section',
      'body [aria-live="polite"]',
      'body [aria-live="assertive"]'
    ];
    const genericSelectors = ['body div'];
    const explicit = uniqueNodes(explicitSelectors)
      .map((node) => candidateFrom(node, composer, query, true))
      .filter(Boolean);
    const semantic = uniqueNodes(semanticSelectors)
      .map((node) => candidateFrom(node, composer, query, true))
      .filter(Boolean);
    const generic = uniqueNodes(genericSelectors).slice(0, 1200)
      .map((node) => candidateFrom(node, composer, query, false))
      .filter(Boolean)
      .filter((candidate) => {
        const child = Array.from(candidate.node.children)
          .map((node) => candidateFrom(node, composer, query, false))
          .filter(Boolean)
          .sort((left, right) => right.text.length - left.text.length)[0];
        return !child || child.text.length < candidate.text.length * 0.92;
      });
    return explicit.concat(semantic, generic)
      .sort((left, right) => right.score - left.score || left.text.length - right.text.length)[0] || null;
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
    const divCount = Math.min(document.querySelectorAll('body div').length, 9999);
    return [
      'v=' + extractorVersion,
      'main=' + mainCount,
      'explicit=' + explicitCount,
      'semantic=' + semanticCount,
      'div=' + divCount,
      'composer=' + (composer ? 1 : 0),
      'query=' + (extraction.queryFound ? 1 : 0),
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
