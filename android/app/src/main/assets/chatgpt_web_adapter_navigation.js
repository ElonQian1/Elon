(function () {
  'use strict';

  if (window.__elonChatGptNavigation || location.origin !== 'https://chatgpt.com') return;

  const MAX_FEATURES = 60;
  let lastFeatures = [];

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function nodeLabel(node) {
    return cleanText([
      node && node.getAttribute('aria-label'),
      node && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' '));
  }

  function sameOriginPath(node) {
    if (!node || !node.getAttribute('href')) return '';
    try {
      const url = new URL(node.getAttribute('href'), location.origin);
      return url.origin === location.origin ? url.pathname : '';
    } catch {
      return '';
    }
  }

  function classify(label, path) {
    const value = (label + ' ' + path).toLowerCase();
    if (/library|文件库|资料库/.test(value)) return 'library';
    if (/scheduled|schedule|task|已安排|任务/.test(value)) return 'tasks';
    if (/project|项目/.test(value)) return 'projects';
    if (/\bgpt(s)?\b|探索.?gpt|发现.?gpt/.test(value)) return 'gpts';
    if (/memory|记忆/.test(value)) return 'memory';
    if (/plugin|connector|app(s)?\b|插件|应用/.test(value)) return 'apps';
    if (/setting|account|profile|设置|账号|账户/.test(value)) return 'settings';
    if (/more|更多/.test(value)) return 'more';
    return 'navigation';
  }

  function hash(value) {
    let result = 2166136261;
    for (let index = 0; index < value.length; index += 1) {
      result ^= value.charCodeAt(index);
      result = Math.imul(result, 16777619);
    }
    return (result >>> 0).toString(36);
  }

  function featureId(label, kind, path, occurrence) {
    return 'feature_' + hash(kind + '|' + label + '|' + path + '|' + occurrence);
  }

  function isConversationNode(node, path) {
    return /^\/c\/[A-Za-z0-9_-]+$/.test(path) || !!node.closest('a[href*="/c/"]');
  }

  function isExcludedLabel(label) {
    const value = label.toLowerCase();
    return /new chat|create chat|新聊天|新建聊天|open sidebar|close sidebar|打开侧边栏|关闭侧边栏|打开边栏|关闭边栏/.test(value);
  }

  function featureNodes() {
    const scopes = Array.from(document.querySelectorAll(
      'aside, nav, [data-testid*="sidebar" i], [role="navigation"], [role="dialog"]'
    )).filter(isVisible);
    const roots = scopes.length ? scopes : [document.body];
    const seen = new Set();
    const values = [];
    roots.forEach((root) => {
      Array.from(root.querySelectorAll('a[href], button, [role="button"], [role="menuitem"]'))
        .filter(isVisible)
        .forEach((node) => {
          if (seen.has(node)) return;
          seen.add(node);
          const label = nodeLabel(node).slice(0, 120);
          const path = sameOriginPath(node);
          if (!label || isExcludedLabel(label) || isConversationNode(node, path)) return;
          const kind = classify(label, path);
          const routeCandidate = path && path !== '/' && !path.startsWith('/auth');
          if (kind === 'navigation' && !routeCandidate) return;
          values.push({ node, label, path, kind });
        });
    });
    return values;
  }

  function readFeatures() {
    const occurrences = new Map();
    lastFeatures = featureNodes().map((item) => {
      const key = item.kind + '|' + item.label + '|' + item.path;
      const occurrence = occurrences.get(key) || 0;
      occurrences.set(key, occurrence + 1);
      return {
        id: featureId(item.label, item.kind, item.path, occurrence),
        label: item.label,
        kind: item.kind,
        selected: item.node.getAttribute('aria-current') === 'page' ||
          item.node.getAttribute('aria-selected') === 'true',
        node: item.node
      };
    }).slice(0, MAX_FEATURES);
    return lastFeatures.map(({ id, label, kind, selected }) => ({ id, label, kind, selected }));
  }

  function sidebarButton(open) {
    const needles = open
      ? ['open sidebar', '打开边栏', '打开侧边栏']
      : ['close sidebar', '关闭边栏', '关闭侧边栏'];
    return Array.from(document.querySelectorAll('button')).find((button) => {
      if (!isVisible(button)) return false;
      const label = nodeLabel(button).toLowerCase();
      return needles.some((needle) => label.includes(needle));
    }) || null;
  }

  function emitTouchRequest(purpose, node, emitEvent) {
    if (!isVisible(node)) return false;
    const rect = node.getBoundingClientRect();
    emitEvent({
      type: 'web_touch_request',
      purpose,
      xRatio: Math.max(0, Math.min(1, (rect.left + rect.width / 2) / window.innerWidth)),
      yRatio: Math.max(0, Math.min(1, (rect.top + rect.height / 2) / window.innerHeight))
    });
    return true;
  }

  function emitSnapshot(emitEvent) {
    const features = readFeatures();
    emitEvent({ type: 'navigation_snapshot', features });
    return features;
  }

  function requestList(emitEvent, result) {
    const existing = emitSnapshot(emitEvent);
    if (existing.length) return result('list_navigation', true, '');
    const open = sidebarButton(true);
    if (!open || !emitTouchRequest('list_navigation', open, emitEvent)) {
      return result('list_navigation', false, '官网功能侧栏入口当前不可见。');
    }
    result('list_navigation', true, '');
  }

  function collectList(emitEvent, result) {
    const features = emitSnapshot(emitEvent);
    result(
      'collect_navigation',
      features.length > 0,
      features.length ? '' : '官网功能侧栏尚未加载完成。'
    );
  }

  function selectFeature(id, emitEvent, result) {
    const feature = lastFeatures.find((item) => item.id === id);
    if (!feature || !isVisible(feature.node)) {
      return result('select_navigation', false, '官网功能入口已变化，请重新打开功能面板。');
    }
    if (!emitTouchRequest('select_navigation', feature.node, emitEvent)) {
      return result('select_navigation', false, '官网功能入口当前不可见。');
    }
    result('select_navigation', true, '');
  }

  function dismiss(emitEvent, result) {
    const close = sidebarButton(false);
    if (close) emitTouchRequest('dismiss_navigation', close, emitEvent);
    result('dismiss_navigation', true, '');
  }

  function capabilities() {
    return sidebarButton(true) || sidebarButton(false) || featureNodes().length
      ? ['feature_navigation']
      : [];
  }

  window.__elonChatGptNavigation = Object.freeze({
    capabilities,
    collectList,
    dismiss,
    requestList,
    selectFeature
  });
})();
