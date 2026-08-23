(function () {
  'use strict';

  const existing = window.__elonGoogleWebPrivateReplyObserver;
  if (existing && Number(existing.version) >= 5) return;
  if (location.origin !== 'https://google.com' && location.origin !== 'https://www.google.com') return;

  const candidatePolicy = window.__elonGoogleWebAnswerCandidatePolicy;
  const originalFetch = typeof window.fetch === 'function' ? window.fetch : null;
  let prompt = '';
  let baseline = new WeakSet();
  let currentReply = null;
  let listener = null;
  let generation = 0;
  let baselineCount = 0;
  let responseCount = 0;
  let probeCount = 0;
  const MAX_REPLY_LENGTH = 40000;
  const PROBE_DELAYS_MS = [40, 120, 260, 520, 900, 1400, 2200];

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
    return rect.width > 0 && rect.height > 0 && node.getClientRects().length > 0;
  }

  function directText(node) {
    return cleanText(Array.from(node.childNodes)
      .filter((child) => child.nodeType === Node.TEXT_NODE)
      .map((child) => child.nodeValue || '')
      .join(' '));
  }

  function candidateNodes() {
    return Array.from(document.querySelectorAll(
      'main span, main p, [role="main"] span, [role="main"] p'
    )).filter((node) => {
      const text = directText(node);
      if (!text || text === prompt || text.length > MAX_REPLY_LENGTH || !isVisible(node)) return false;
      if (!node.closest('main, [role="main"]') || node.closest(
        'header, nav, footer, form, [role="navigation"], [role="dialog"], ' +
        '[role="tablist"], [role="toolbar"], a[href], button, [role="button"], ' +
        '[role="link"], li, [role="listitem"], [aria-live], [role="status"], [role="alert"]'
      )) return false;
      if (node.querySelector('a[href], button, [role="button"], input, textarea')) return false;
      if (!candidatePolicy) return true;
      const filters = [
        'navigationOnlyText',
        'transientStatusText',
        'shareSurfaceText',
        'disclosureOnlyText',
        'pageChromeText'
      ];
      return !filters.some((name) =>
        typeof candidatePolicy[name] === 'function' && candidatePolicy[name](text)
      );
    });
  }

  function collectReply() {
    const seenText = new Set();
    const parts = [];
    for (const node of candidateNodes()) {
      if (baseline.has(node)) continue;
      const text = directText(node);
      if (!text || seenText.has(text)) continue;
      seenText.add(text);
      parts.push(text);
      if (parts.join('\n\n').length >= MAX_REPLY_LENGTH) break;
    }
    return cleanText(parts.join('\n\n')).slice(0, MAX_REPLY_LENGTH);
  }

  function publishReply(activeGeneration, complete) {
    if (activeGeneration !== generation || !prompt) return;
    probeCount += 1;
    const text = collectReply();
    if (!text) return;
    const next = Object.freeze({ prompt, text, streaming: !complete });
    if (currentReply && currentReply.text === next.text && currentReply.streaming === next.streaming) return;
    currentReply = next;
    if (typeof listener === 'function') listener();
  }

  function scheduleReplyProbe() {
    const activeGeneration = generation;
    PROBE_DELAYS_MS.forEach((delay, index) => {
      window.setTimeout(() => publishReply(
        activeGeneration,
        index === PROBE_DELAYS_MS.length - 1
      ), delay);
    });
  }

  function observePrompt(value) {
    prompt = cleanText(value).slice(0, 20000);
    baseline = new WeakSet(candidateNodes());
    baselineCount = candidateNodes().length;
    currentReply = null;
    responseCount = 0;
    probeCount = 0;
    generation += 1;
  }

  if (originalFetch) {
    window.fetch = function () {
      const args = arguments;
      let url;
      try {
        const input = args[0];
        url = new URL(typeof input === 'string' ? input : input.url, location.href);
      } catch (_) {
        return originalFetch.apply(this, args);
      }
      return Promise.resolve(originalFetch.apply(this, args)).then((response) => {
        if (response && response.ok && url.origin === location.origin &&
            url.pathname === '/async/folif' && prompt) {
          responseCount += 1;
          scheduleReplyProbe();
        }
        return response;
      });
    };
  }

  window.__elonGoogleWebPrivateReplyObserver = Object.freeze({
    version: 5,
    observePrompt,
    snapshot: () => currentReply,
    diagnostics: () => {
      const candidates = candidateNodes();
      const added = candidates.filter((node) => !baseline.has(node)).length;
      return [
        'v5',
        'p' + (prompt ? 1 : 0),
        'b' + Math.min(baselineCount, 999),
        'r' + Math.min(responseCount, 9),
        'probe' + Math.min(probeCount, 9),
        'c' + Math.min(candidates.length, 999),
        'n' + Math.min(added, 999),
        'reply' + (currentReply ? 1 : 0),
        'done' + (currentReply && !currentReply.streaming ? 1 : 0)
      ].join('|');
    },
    setListener: (value) => { listener = typeof value === 'function' ? value : null; }
  });
})();
