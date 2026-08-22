(function () {
  'use strict';

  if (window.__elonChatGptCitationAdapter || location.origin !== 'https://chatgpt.com') return;
  const base = window.__elonChatGptMessages;
  if (!base || typeof base.readMessageWindow !== 'function') return;

  const MAX_CITATIONS = 16;

  function cleanText(value, max) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[\u200b-\u200f\u2060\ufeff]/g, '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, max || 160);
  }

  function safePublicUrl(value) {
    try {
      const url = new URL(String(value || ''), location.href);
      if (url.protocol !== 'https:' || url.username || url.password ||
          (url.port && url.port !== '443') || url.origin === location.origin) return '';
      return (url.origin + url.pathname).slice(0, 1200);
    } catch (_) {
      return '';
    }
  }

  function safeIconUrl(value) {
    try {
      const url = new URL(String(value || ''), location.href);
      if (url.protocol !== 'https:' || url.username || url.password ||
          (url.port && url.port !== '443') || url.search || url.hash) return '';
      return (url.origin + url.pathname).slice(0, 1200);
    } catch (_) {
      return '';
    }
  }

  function citationGroupSize(value) {
    const match = cleanText(value, 80).match(/\+(\d+)\s*$/);
    return Math.min(32, Math.max(1, match ? Number(match[1]) + 1 : 1));
  }

  function normalizeCitationRecord(value, index) {
    const markerText = cleanText(value && (value.markerText || value.text), 80);
    const url = safePublicUrl(value && value.url);
    if (!markerText || !url) return null;
    const iconUrl = safeIconUrl(value && value.iconUrl);
    const host = new URL(url).hostname.toLowerCase().slice(0, 253);
    return {
      type: 'citation',
      text: markerText,
      title: markerText,
      url,
      targetKind: 'external',
      targetHost: host,
      markerText,
      citationId: 'citation_control_' + Math.max(1, Number(index) + 1),
      groupSize: citationGroupSize(markerText),
      ...(iconUrl ? { iconUrl } : {})
    };
  }

  function visible(node) {
    if (!(node instanceof Element) || !node.isConnected) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden' && Number(style.opacity) !== 0;
  }

  function referencedTargets(node) {
    const ids = cleanText([
      node.getAttribute('aria-controls'),
      node.getAttribute('aria-describedby')
    ].filter(Boolean).join(' '), 300).split(/\s+/).filter(Boolean).slice(0, 8);
    return ids.map((id) => document.getElementById(id)).filter(Boolean);
  }

  function linkedRecord(node, index) {
    const markerText = cleanText(
      node.innerText || node.textContent || node.getAttribute('aria-label'),
      80
    );
    if (!markerText) return null;
    const directUrl = safePublicUrl(
      node.getAttribute('data-citation-url') || node.getAttribute('data-source-url')
    );
    const targets = referencedTargets(node);
    const anchor = targets.flatMap((target) => Array.from(target.querySelectorAll('a[href]')))
      .find((candidate) => safePublicUrl(candidate.href || candidate.getAttribute('href')));
    const icon = anchor && (anchor.querySelector('img') || targets
      .flatMap((target) => Array.from(target.querySelectorAll('img')))[0]);
    return normalizeCitationRecord({
      markerText,
      url: directUrl || (anchor && (anchor.href || anchor.getAttribute('href'))),
      iconUrl: icon && (icon.currentSrc || icon.src || icon.getAttribute('src'))
    }, index);
  }

  function citationRecords(root) {
    if (!(root instanceof Element)) return [];
    const records = [];
    const seen = new Set();
    const controls = root.querySelectorAll(
      'button[aria-controls], button[aria-describedby], [role="button"][aria-controls], '
      + '[role="button"][aria-describedby], [data-citation-url], [data-source-url]'
    );
    Array.from(controls).slice(0, 64).forEach((node) => {
      if (records.length >= MAX_CITATIONS || !visible(node)) return;
      const record = linkedRecord(node, records.length);
      if (!record || seen.has(record.url)) return;
      seen.add(record.url);
      records.push(record);
    });
    return records;
  }

  function roleOf(node) {
    const owner = node.matches('[data-message-author-role]')
      ? node
      : node.querySelector('[data-message-author-role]');
    return owner && owner.getAttribute('data-message-author-role');
  }

  function messageNodes() {
    const main = document.querySelector('main');
    if (!main) return [];
    const turns = Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]'));
    return turns.length ? turns : Array.from(main.querySelectorAll('[data-message-author-role]'));
  }

  function literalCount(value, needle) {
    if (!needle) return 0;
    let count = 0;
    let offset = 0;
    while ((offset = value.indexOf(needle, offset)) >= 0) {
      count += 1;
      offset += needle.length;
    }
    return count;
  }

  function linkMarker(markdown, record) {
    const marker = record.markerText;
    if (!marker || literalCount(markdown, marker) !== 1 ||
        markdown.includes('[' + marker + '](')) return markdown;
    return markdown.replace(marker, '[' + marker.replace(/[\[\]]/g, '\\$&') + '](' + record.url + ')');
  }

  function augmentMessage(message, root) {
    if (!message || message.role !== 'assistant' || !(root instanceof Element)) return message;
    const records = citationRecords(root);
    if (!records.length) return message;
    const content = Array.isArray(message.content) ? message.content.slice() : [];
    const knownUrls = new Set(content.filter((part) => part && part.type === 'citation')
      .map((part) => safePublicUrl(part.url)).filter(Boolean));
    const markdownIndex = content.findIndex((part) => part && part.type === 'markdown');
    if (markdownIndex >= 0) {
      const part = Object.assign({}, content[markdownIndex]);
      part.text = records.reduce((text, record) => linkMarker(text, record), String(part.text || ''));
      content[markdownIndex] = part;
    }
    records.forEach((record) => {
      if (!knownUrls.has(record.url)) content.push(record);
    });
    return Object.assign({}, message, { content: content.slice(0, 24) });
  }

  function readMessageWindow(streaming, streamingAssistantKey) {
    const snapshot = base.readMessageWindow(streaming, streamingAssistantKey);
    const nodes = messageNodes().filter((node) => roleOf(node));
    const start = Math.max(0, nodes.length - (snapshot.messages || []).length);
    let cursor = start;
    const messages = (snapshot.messages || []).map((message) => {
      while (cursor < nodes.length && roleOf(nodes[cursor]) !== message.role) cursor += 1;
      const root = nodes[cursor] || null;
      cursor += 1;
      return augmentMessage(message, root);
    });
    return Object.assign({}, snapshot, { messages });
  }

  function readMessages(streaming) {
    return readMessageWindow(streaming).messages;
  }

  const api = Object.freeze({
    version: 1,
    citationGroupSize,
    normalizeCitationRecord,
    citationRecords
  });
  window.__elonChatGptCitationAdapter = api;
  window.__elonChatGptMessages = Object.freeze(Object.assign({}, base, {
    readMessages,
    readMessageWindow
  }));
})();
