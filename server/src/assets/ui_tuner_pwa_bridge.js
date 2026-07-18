(function installUiTunerPwaBridge() {
  'use strict';
  const params = new URLSearchParams(window.location.search || '');
  if (params.get('ui_tuner_preview') !== '1' || window.parent === window) return;

  const SOURCE = 'elon-pwa-design-bridge';
  const PARENT_SOURCE = 'elon-pc-ui-tuner';
  const PROTOCOL_VERSION = 1;
  const originalInlineStyles = new Map();
  let selectedElement = null;
  let selecting = false;

  document.body.classList.add('ui-tuner-preview-active');
  const selection = document.createElement('div');
  selection.id = 'uiTunerPreviewSelection';
  selection.setAttribute('aria-hidden', 'true');
  document.body.appendChild(selection);

  function post(type, payload) {
    window.parent.postMessage({ source: SOURCE, protocolVersion: PROTOCOL_VERSION, type, payload }, window.location.origin);
  }

  function compactText(element) {
    const directText = Array.from(element.childNodes || [])
      .filter((node) => node.nodeType === Node.TEXT_NODE)
      .map((node) => node.textContent || '')
      .join(' ');
    const accessibleText = element.getAttribute('aria-label')
      || element.getAttribute('title')
      || element.getAttribute('placeholder')
      || directText;
    return String(accessibleText || '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 160);
  }

  function identityOf(element) {
    const uiNode = element.getAttribute('data-ui-node') || '';
    const id = element.id || '';
    const ariaLabel = element.getAttribute('aria-label') || '';
    const role = element.getAttribute('role') || '';
    const text = compactText(element);
    return {
      key: uiNode || id || ariaLabel || [element.tagName.toLowerCase(), role, text.slice(0, 48)].filter(Boolean).join(':'),
      uiNode,
      id,
      ariaLabel,
      role,
      text,
      tag: element.tagName.toLowerCase(),
      classNames: Array.from(element.classList).filter((name) => !name.startsWith('ui-tuner-')).slice(0, 12),
    };
  }

  function snapshotOf(element, knownRect) {
    const rect = knownRect || element.getBoundingClientRect();
    const computed = window.getComputedStyle(element);
    return {
      identity: identityOf(element),
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      baseStyle: {
        width: computed.width,
        height: computed.height,
        borderRadius: computed.borderRadius,
        fontSize: computed.fontSize,
        fontWeight: computed.fontWeight,
        lineHeight: computed.lineHeight,
        color: computed.color,
        backgroundColor: computed.backgroundColor,
        padding: {
          start: computed.paddingInlineStart,
          top: computed.paddingTop,
          end: computed.paddingInlineEnd,
          bottom: computed.paddingBottom,
        },
        margin: {
          start: computed.marginInlineStart,
          top: computed.marginTop,
          end: computed.marginInlineEnd,
          bottom: computed.marginBottom,
        },
        opacity: computed.opacity,
      },
    };
  }

  function routeState(reason) {
    return {
      reason,
      href: window.location.href,
      path: window.location.pathname,
      search: window.location.search,
      hash: window.location.hash,
      title: document.title,
      viewport: { width: window.innerWidth, height: window.innerHeight },
      scroll: { x: window.scrollX, y: window.scrollY },
    };
  }

  function postRoute(reason) {
    post('route-changed', routeState(reason));
  }

  function findDesignTarget(target) {
    if (!(target instanceof Element) || target === selection) return null;
    return target.closest('[data-ui-node],[id],button,input,textarea,select,a,[role="button"],.tab,.conversation-item,.project-card')
      || target.closest('header,nav,main,section,article,div,span');
  }

  function drawSelection(element, knownRect) {
    if (!element || !element.isConnected) {
      selection.style.display = 'none';
      return;
    }
    const rect = knownRect || element.getBoundingClientRect();
    selection.style.display = rect.width > 0 && rect.height > 0 ? 'block' : 'none';
    selection.style.left = rect.left + 'px';
    selection.style.top = rect.top + 'px';
    selection.style.width = rect.width + 'px';
    selection.style.height = rect.height + 'px';
  }

  function selectElement(element, reason) {
    const rect = element.getBoundingClientRect();
    selectedElement = element;
    drawSelection(element, rect);
    post('selection', { reason: reason || 'pointer', node: snapshotOf(element, rect) });
  }

  function cssDimension(value) {
    if (value === null || value === undefined || value === '') return '';
    if (typeof value === 'number') return value + 'px';
    const text = String(value).trim();
    if (/^-?\d+(\.\d+)?(dp|sp)$/.test(text)) return parseFloat(text) + 'px';
    if (/^-?\d+(\.\d+)?$/.test(text)) return text + 'px';
    if (text === 'match_parent' || text === 'fill_parent') return '100%';
    if (text === 'wrap_content') return 'auto';
    return text;
  }

  function rememberInlineStyles(element) {
    if (!originalInlineStyles.has(element)) originalInlineStyles.set(element, element.getAttribute('style'));
  }

  function applyStyle(payload) {
    if (!selectedElement || !selectedElement.isConnected || !payload || !payload.style) return;
    const element = selectedElement;
    const style = payload.style;
    rememberInlineStyles(element);
    const direct = ['color', 'backgroundColor', 'borderRadius', 'fontSize', 'fontWeight', 'opacity'];
    direct.forEach((property) => {
      if (style[property] !== undefined) element.style[property] = property === 'fontSize' || property === 'borderRadius'
        ? cssDimension(style[property])
        : String(style[property]);
    });
    ['width', 'height'].forEach((property) => {
      if (style[property] !== undefined) element.style[property] = cssDimension(style[property]);
    });
    ['padding', 'margin'].forEach((group) => {
      const edges = style[group];
      if (!edges) return;
      element.style[group] = [edges.top, edges.end, edges.bottom, edges.start].map(cssDimension).join(' ');
    });
    if (payload.text !== undefined && element.children.length === 0) element.textContent = String(payload.text);
    drawSelection(element);
    post('style-applied', { node: snapshotOf(element) });
  }

  function resetStyles() {
    originalInlineStyles.forEach((original, element) => {
      if (!element.isConnected) return;
      if (original === null) element.removeAttribute('style');
      else element.setAttribute('style', original);
    });
    originalInlineStyles.clear();
    drawSelection(selectedElement);
    post('styles-reset', {});
  }

  document.addEventListener('click', (event) => {
    if (!selecting) return;
    const target = findDesignTarget(event.target);
    if (!target) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    selectElement(target, 'click');
  }, true);
  window.addEventListener('scroll', () => drawSelection(selectedElement), true);
  window.addEventListener('resize', () => drawSelection(selectedElement));
  window.addEventListener('hashchange', () => postRoute('hashchange'));
  window.addEventListener('popstate', () => postRoute('popstate'));
  ['pushState', 'replaceState'].forEach((method) => {
    const original = history[method];
    history[method] = function uiTunerHistoryChange() {
      const result = original.apply(this, arguments);
      postRoute(method);
      return result;
    };
  });

  window.addEventListener('message', (event) => {
    if (event.origin !== window.location.origin || event.source !== window.parent) return;
    const message = event.data || {};
    if (message.source !== PARENT_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return;
    if (message.type === 'set-mode') {
      selecting = message.payload && message.payload.mode !== 'interact';
      document.body.classList.toggle('ui-tuner-preview-selecting', selecting);
      drawSelection(selecting ? selectedElement : null);
      post('mode-changed', { mode: selecting ? 'select' : 'interact' });
    } else if (message.type === 'set-session-auth') {
      const sessionToken = String(message.payload && message.payload.token || '');
      if (!sessionToken || sessionToken.length > 8192) return;
      window.dispatchEvent(new CustomEvent('elon:ui-tuner-session-auth', {
        detail: { token: sessionToken },
      }));
      post('session-auth-accepted', {});
    } else if (message.type === 'apply-style') {
      applyStyle(message.payload);
    } else if (message.type === 'reset-styles') {
      resetStyles();
    }
  });

  post('ready', {
    href: window.location.href,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    mode: 'interact',
  });
  postRoute('ready');
})();
