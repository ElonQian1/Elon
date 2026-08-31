(function () {
  'use strict';

  if (window.__elonChatGptNavigation || location.origin !== 'https://chatgpt.com') return;

  const navigationPolicy = window.__elonChatGptNavigationPolicy;
  if (!navigationPolicy) return;

  const MAX_FEATURES = 60;
  let lastFeatures = [];

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 &&
      rect.bottom > 0 && rect.right > 0 && rect.top < window.innerHeight && rect.left < window.innerWidth &&
      style.display !== 'none' && style.visibility !== 'hidden';
  }

  function isSidebarScope(node) {
    if (!isVisible(node)) return false;
    const rect = node.getBoundingClientRect();
    return rect.left <= window.innerWidth * 0.1 &&
      rect.top <= window.innerHeight * 0.25 &&
      rect.width >= window.innerWidth * 0.35 &&
      rect.height >= window.innerHeight * 0.6;
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
    return navigationPolicy.isConversationPath(path) || !!node.closest('a[href*="/c/"]');
  }

  function isExcludedLabel(label) {
    const value = label.toLowerCase();
    return /new chat|create chat|新聊天|新建聊天|open sidebar|close sidebar|打开侧边栏|关闭侧边栏|打开边栏|关闭边栏/.test(value);
  }

  function featureNodes() {
    const scopes = Array.from(document.querySelectorAll(
      'aside, nav, [data-testid*="sidebar" i], [role="navigation"], [role="dialog"]'
    )).filter(isSidebarScope);
    if (!scopes.length) return [];
    const roots = scopes;
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
          const kind = navigationPolicy.classify(label, path);
          const routeCandidate = path && path !== '/' && !path.startsWith('/auth');
          if (kind === 'navigation' && !routeCandidate) return;
          values.push({ node, label, path, kind });
        });
    });
    return values;
  }

  function readFeatures() {
    const observed = featureNodes();
    const observedKinds = new Set(observed.map((item) => item.kind));
    const builtIn = navigationPolicy.builtInFeatures()
      .filter((item) => !observedKinds.has(item.kind))
      .map((item) => Object.assign({ node: null }, item));
    const occurrences = new Map();
    lastFeatures = observed.concat(builtIn).map((item) => {
      const key = item.kind + '|' + item.label + '|' + item.path;
      const occurrence = occurrences.get(key) || 0;
      occurrences.set(key, occurrence + 1);
      return {
        id: featureId(item.label, item.kind, item.path, occurrence),
        label: item.label,
        kind: item.kind,
        selected: item.node ? item.node.getAttribute('aria-current') === 'page' ||
          item.node.getAttribute('aria-selected') === 'true' : location.pathname === item.path,
        node: item.node,
        path: item.path
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
    const hasVisibleOfficialFeature = lastFeatures.some((feature) =>
      feature.node && isVisible(feature.node)
    );
    if (existing.length && hasVisibleOfficialFeature) {
      return result('list_navigation', true, '');
    }
    const open = sidebarButton(true);
    if (open) {
      if (!emitTouchRequest('list_navigation', open, emitEvent)) {
        return result('list_navigation', false, '官网功能侧栏入口当前不可见。');
      }
      return result('list_navigation', true, '');
    }
    const layout = window.__elonChatGptLayout;
    if (
      layout && typeof layout.requestSemanticTouch === 'function' &&
      layout.requestSemanticTouch('navigation', 'list_navigation', emitEvent)
    ) {
      return result('list_navigation', true, '');
    }
    result('list_navigation', false, '官网功能侧栏入口当前不可见。');
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
    if (feature && !feature.node && feature.path) {
      location.assign(feature.path);
      return result('select_navigation', true, '');
    }
    if (!feature || !isVisible(feature.node)) {
      return result('select_navigation', false, '官网功能入口已变化，请重新打开功能面板。');
    }
    if (!emitTouchRequest('select_navigation', feature.node, emitEvent)) {
      return result('select_navigation', false, '官网功能入口当前不可见。');
    }
    result('select_navigation', true, '');
  }

  function dismiss(emitEvent, result) {
    const layout = window.__elonChatGptLayout;
    if (
      layout && typeof layout.requestSemanticTouch === 'function' &&
      layout.requestSemanticTouch('close', 'dismiss_navigation', emitEvent, 'overlay')
    ) {
      return result('dismiss_navigation', true, '');
    }
    const close = sidebarButton(false);
    if (close && emitTouchRequest('dismiss_navigation', close, emitEvent)) {
      return result('dismiss_navigation', true, '');
    }
    result('dismiss_navigation', false, '官网功能侧栏关闭入口当前不可见。');
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
