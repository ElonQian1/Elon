(function () {
  'use strict';

  const extractorVersion = 25;
  if (window.__elonGoogleWebMessageExtractor &&
      window.__elonGoogleWebMessageExtractor.version === extractorVersion) return;

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const candidatePolicy = window.__elonGoogleWebAnswerCandidatePolicy;
  const queryPolicy = window.__elonGoogleWebQueryPolicy;
  const richContent = window.__elonGoogleWebRichContent;
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
  const QUERY_SELECTORS = [
    'main [data-user-query]',
    'main [data-query]',
    'main [aria-label*="your question" i]',
    'main [aria-label*="您的问题" i]'
  ];
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
    const headings = Array.from(document.querySelectorAll(QUERY_SELECTORS.join(',')))
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
        const iconUrl = citationIconUrl(link);
        parts.push({
          type: 'citation',
          text: title.slice(0, 160),
          title: title.slice(0, 160),
          url: safeUrl.slice(0, 1200),
          targetKind: 'external',
          targetHost: url.hostname.slice(0, 253),
          ...(iconUrl ? { iconUrl } : {})
        });
        if (parts.length >= 12) break;
      } catch (_) {}
    }
    return parts;
  }

  function citationIconUrl(link) {
    const icon = link && link.querySelector && link.querySelector('img');
    if (!icon) return '';
    try {
      const url = new URL(icon.currentSrc || icon.getAttribute('src') || '', location.href);
      if (url.protocol !== 'https:' || url.username || url.password ||
          (url.port && url.port !== '443')) return '';
      return (url.origin + url.pathname).slice(0, 1200);
    } catch (_) {
      return '';
    }
  }

  function externalLinkTextLength(container) {
    if (!container) return 0;
    let length = 0;
    for (const link of container.querySelectorAll('a[href^="https://"]')) {
      if (!isVisible(link)) continue;
      try {
        const url = new URL(link.href);
        if (allowedOrigins.has(url.origin)) continue;
        length += cleanText(link.innerText || link.textContent || link.getAttribute('aria-label')).length;
      } catch (_) {}
    }
    return Math.min(length, 40000);
  }

  function narrativeBlockCount(container) {
    if (!container) return 0;
    let count = 0;
    for (const block of container.querySelectorAll('p, li, blockquote')) {
      if (!isVisible(block)) continue;
      const text = cleanText(block.innerText || block.textContent);
      if (text.length < 80) continue;
      const linkedLength = externalLinkTextLength(block);
      if (linkedLength / text.length > 0.45) continue;
      count += 1;
      if (count >= 12) break;
    }
    return count;
  }

  function sourceResultItemCount(container) {
    if (!container) return 0;
    let count = 0;
    for (const item of container.querySelectorAll('li, [role="listitem"]')) {
      if (!isVisible(item)) continue;
      const text = cleanText(item.innerText || item.textContent);
      if (text.length < 60 || text.length > 400) continue;
      const linkedLength = externalLinkTextLength(item);
      if (linkedLength < 12 || linkedLength / text.length < 0.08) continue;
      count += 1;
      if (count >= 12) break;
    }
    return count;
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

  function alignedWithQuery(node, queryAnchor) {
    if (!queryAnchor || !isVisible(queryAnchor)) return false;
    const nodeRect = node.getBoundingClientRect();
    const queryRect = queryAnchor.getBoundingClientRect();
    const queryCenter = queryRect.left + queryRect.width / 2;
    const maximumAnswerWidth = Math.max(queryRect.width * 4, 960);
    return nodeRect.width <= maximumAnswerWidth &&
      queryCenter >= nodeRect.left - 24 && queryCenter <= nodeRect.right + 24;
  }

  function findQueryAnchor(query) {
    if (!query) return null;
    const nodes = uniqueNodes([
      ...QUERY_SELECTORS,
      'body div',
      'body span',
      'body p'
    ]).slice(0, 2400).filter((node) =>
      isVisible(node) && cleanText(node.innerText || node.textContent) === query
    );
    return nodes[nodes.length - 1] || null;
  }

  function answerActionLabel(value) {
    const label = cleanText(value).replace(/\s+/g, ' ').replace(/[.!。！]+$/, '');
    return /^(?:copy (?:text|response|answer)|复制(?:文字|回答|回复))$/i.test(label);
  }

  function findAnswerActionAnchor(queryAnchor, nextQueryAnchor, composer) {
    if (!queryAnchor) return null;
    return uniqueNodes(['main button', 'main [role="button"]', 'body button', 'body [role="button"]'])
      .filter((node) => {
        if (!isVisible(node) || !followsQuery(node, queryAnchor)) return false;
        if (nextQueryAnchor && !precedesBoundary(node, nextQueryAnchor)) return false;
        if (composer && !precedesBoundary(node, composer)) return false;
        return answerActionLabel(
          node.getAttribute('aria-label') || node.getAttribute('title') ||
          node.innerText || node.textContent
        );
      })[0] || null;
  }

  function precedesBoundary(node, boundary) {
    if (!boundary || node === boundary || node.contains(boundary) || typeof Node === 'undefined') {
      return !boundary;
    }
    return !!(node.compareDocumentPosition(boundary) & Node.DOCUMENT_POSITION_FOLLOWING);
  }

  function boundaryRelation(node, boundary) {
    if (!boundary || typeof Node === 'undefined') return 'unknown';
    if (node === boundary || node.contains(boundary)) return 'contains';
    const position = node.compareDocumentPosition(boundary);
    if (position & Node.DOCUMENT_POSITION_FOLLOWING) return 'before';
    if (position & Node.DOCUMENT_POSITION_PRECEDING) return 'after';
    return 'unknown';
  }

  function candidateFrom(
    node, composer, query, queryAnchor, nextQueryAnchor, answerActionAnchor, explicit
  ) {
    if (excludedContainer(node, composer)) return null;
    if (nextQueryAnchor && !precedesBoundary(node, nextQueryAnchor)) return null;
    let text = cleanText(node.innerText || node.textContent).slice(0, 40000);
    if (query && text.startsWith(query)) text = cleanText(text.slice(query.length));
    if (!text || text === query) return null;
    const citations = citationParts(node);
    const semanticBlocks = node.querySelectorAll('p, li, blockquote, pre, table, h2, h3').length;
    const controls = node.querySelectorAll('button, [role="button"], input, textarea').length;
    const links = node.querySelectorAll('a[href]').length;
    const linkedTextLength = externalLinkTextLength(node);
    const narrativeBlocks = narrativeBlockCount(node);
    const sourceResultItems = sourceResultItemCount(node);
    const queryAligned = alignedWithQuery(node, queryAnchor);
    const answerActionRelation = boundaryRelation(node, answerActionAnchor);
    const citationTextRatio = text.length ? Math.min(linkedTextLength, text.length) / text.length : 0;
    const sourceCollection = candidatePolicy && typeof candidatePolicy.sourceCollection === 'function'
      ? candidatePolicy.sourceCollection({
          citations: citations.length,
          links,
          semanticBlocks,
          narrativeBlocks,
          sourceResultItems,
          queryAligned,
          answerActionRelation,
          textLength: text.length,
          citationTextRatio
        })
      : citations.length >= 2 && links >= 2 && citationTextRatio >= 0.45;
    const tabControls = node.querySelectorAll('[role="tab"], [role="tablist"], [role="toolbar"]').length +
      (node.matches('[role="tab"], [role="tablist"], [role="toolbar"]') ? 1 : 0);
    const liveRegion = node.matches('[aria-live], [role="status"], [role="alert"]');
    const metrics = {
      hasQuery: !!query,
      text,
      textLength: text.length,
      citations: citations.length,
      semanticBlocks,
      narrativeBlocks,
      sourceResultItems,
      links,
      tabControls,
      liveRegion,
      controls,
      afterQuery: followsQuery(node, queryAnchor),
      queryAligned,
      answerActionRelation,
      trustedAnswerContainer: node.matches(TRUSTED_ANSWER_SELECTOR),
      resultListItem: !!node.closest('li, [role="listitem"]'),
      sourceCollection: sourceCollection,
      externalLinkTextLength: linkedTextLength,
      citationTextRatio,
      interactive: !!node.closest(
        'a[href], button, input, textarea, select, [role="button"], [role="link"], ' +
        '[role="menuitem"], [role="tab"]'
      ),
      explicit
    };
    if (candidatePolicy && !candidatePolicy.accepts(metrics)) return null;
    const depth = Math.min(12, node.closest('main, [role="main"]') ? 1 : 0);
    const score = Math.min(text.length, 8000) + Math.min(citations.length, 3) * 180 +
      Math.min(semanticBlocks, 20) * 90 + Math.min(narrativeBlocks, 8) * 520 +
      (explicit ? 1400 : 0) + (queryAligned ? 2200 : 0) +
      (answerActionRelation === 'before' ? 2600 : 0) + depth * 30 -
      Math.min(controls, 20) * 120 -
      (candidatePolicy ? candidatePolicy.penalty(metrics) : 0);
    return {
      node,
      text,
      citations,
      score,
      explicit,
      narrativeBlocks,
      sourceCollection,
      sourceResultItems,
      afterQuery: metrics.afterQuery,
      queryAligned,
      answerActionRelation,
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

  function answerCandidate(composer, query, queryAnchor, nextQueryAnchor) {
    queryAnchor = queryAnchor || findQueryAnchor(query);
    const answerActionAnchor = findAnswerActionAnchor(queryAnchor, nextQueryAnchor, composer);
    const explicitSelectors = TRUSTED_ANSWER_SELECTORS.concat('[role="region"]');
    const semanticSelectors = [
      'body section',
      'body [aria-live="polite"]',
      'body [aria-live="assertive"]'
    ];
    const genericSelectors = ['body div'];
    const explicit = uniqueNodes(explicitSelectors)
      .map((node) => candidateFrom(
        node, composer, query, queryAnchor, nextQueryAnchor, answerActionAnchor, true
      ))
      .filter(Boolean);
    const semantic = uniqueNodes(semanticSelectors)
      .map((node) => candidateFrom(
        node, composer, query, queryAnchor, nextQueryAnchor, answerActionAnchor, true
      ))
      .filter(Boolean);
    const generic = uniqueNodes(genericSelectors).slice(0, 1200)
      .map((node) => candidateFrom(
        node, composer, query, queryAnchor, nextQueryAnchor, answerActionAnchor, false
      ))
      .filter(Boolean)
      .filter((candidate) => {
        if (candidate.text.length < 8) return true;
        const child = Array.from(candidate.node.children)
          .map((node) => candidateFrom(
            node, composer, query, queryAnchor, nextQueryAnchor, answerActionAnchor, false
          ))
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

  function explicitQueries() {
    const entries = uniqueNodes(QUERY_SELECTORS)
      .filter(isVisible)
      .map((node) => ({ node, text: cleanText(node.innerText || node.textContent).slice(0, 40000) }))
      .filter((entry) => entry.text.length > 1);
    return entries.filter((entry, index) => !entries.some((other, otherIndex) =>
      otherIndex !== index && entry.node.contains(other.node) && entry.text === other.text
    )).filter((entry, index, values) =>
      index === 0 || entry.text !== values[index - 1].text
    );
  }

  function queryEntries() {
    const entries = explicitQueries();
    const current = currentQuery();
    if (current && !entries.some((entry) => entry.text === current)) {
      entries.push({ node: findQueryAnchor(current), text: current });
    }
    return entries.length ? entries : (current ? [{ node: findQueryAnchor(current), text: current }] : []);
  }

  function extract(composer, streaming) {
    const queries = queryEntries();
    const messages = [];
    let answerCount = 0;
    queries.forEach((entry, index) => {
      const next = queries[index + 1];
      const answer = answerCandidate(composer, entry.text, entry.node, next && next.node);
      messages.push({
        id: 'google-query-' + index,
        role: 'user',
        state: 'completed',
        content: [{ type: 'text', text: entry.text }]
      });
      if (!answer) return;
      const content = richContent && typeof richContent.parts === 'function'
        ? richContent.parts(answer.node, answer.text, entry.text)
        : (answer.text ? [{ type: 'text', text: answer.text }] : []);
      content.push(...answer.citations);
      if (content.length) messages.push({
        id: 'google-answer-' + index,
        role: 'assistant',
        state: streaming && index === queries.length - 1 ? 'streaming' : 'completed',
        content
      });
      answerCount += 1;
    });
    return {
      messages,
      queryFound: queries.length > 0,
      answerFound: answerCount > 0,
      observedMessageCount: messages.length,
      messageWindowStart: 0,
      turnCount: queries.length
    };
  }

  function lastAnswerNode(composer) {
    const queries = queryEntries();
    for (let index = queries.length - 1; index >= 0; index -= 1) {
      const entry = queries[index];
      const next = queries[index + 1];
      const answer = answerCandidate(composer || null, entry.text, entry.node, next && next.node);
      if (answer && answer.node && answer.node.isConnected) return answer.node;
    }
    return null;
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
      'answer=' + (extraction.answerFound ? 1 : 0),
      'turns=' + Math.min(99, extraction.turnCount || 0)
    ].join('|').slice(0, 160);
  }

  window.__elonGoogleWebMessageExtractor = Object.freeze({
    version: extractorVersion,
    rememberQuery,
    clearRememberedQuery,
    hasCurrentQuery,
    currentQueryMatches,
    lastAnswerNode,
    extract,
    diagnostics
  });
})();
