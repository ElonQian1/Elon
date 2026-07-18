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
  const editableProperties = [
    'width', 'height',
    'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
    'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
    'borderRadius', 'fontSize', 'fontWeight', 'lineHeight',
    'color', 'backgroundColor', 'opacity',
  ];

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

  function attributeSelector(name, value) {
    return '[' + name + '=' + JSON.stringify(String(value)) + ']';
  }

  function uniqueSelector(selector) {
    try {
      return document.querySelectorAll(selector).length === 1;
    } catch (_) {
      return false;
    }
  }

  function domPath(element) {
    const parts = [];
    let current = element;
    while (current && current !== document.body && parts.length < 12) {
      const tag = current.tagName.toLowerCase();
      const siblings = current.parentElement
        ? Array.from(current.parentElement.children).filter((candidate) => candidate.tagName === current.tagName)
        : [];
      const position = siblings.length > 1 ? ':nth-of-type(' + (siblings.indexOf(current) + 1) + ')' : '';
      parts.unshift(tag + position);
      current = current.parentElement;
    }
    return 'body' + (parts.length ? ' > ' + parts.join(' > ') : '');
  }

  function selectorIdentity(element) {
    const uiNode = element.getAttribute('data-ui-node') || '';
    if (uiNode) {
      const selector = attributeSelector('data-ui-node', uiNode);
      if (uniqueSelector(selector)) return { selector, strategy: 'data-ui-node', confidence: 'high', confidenceScore: 1, needsBinding: false };
    }
    if (element.id) {
      const selector = '#' + CSS.escape(element.id);
      if (uniqueSelector(selector)) return { selector, strategy: 'id', confidence: 'high', confidenceScore: .95, needsBinding: false };
    }
    const stableAttributes = ['data-testid', 'data-component', 'data-source-node'];
    for (const name of stableAttributes) {
      const value = element.getAttribute(name);
      if (!value) continue;
      const selector = attributeSelector(name, value);
      if (uniqueSelector(selector)) return { selector, strategy: 'data-attribute', confidence: 'medium', confidenceScore: .78, needsBinding: true };
    }
    const ariaLabel = element.getAttribute('aria-label') || '';
    if (ariaLabel) {
      const selector = element.tagName.toLowerCase() + attributeSelector('aria-label', ariaLabel);
      if (uniqueSelector(selector)) return { selector, strategy: 'aria-label', confidence: 'medium', confidenceScore: .7, needsBinding: true };
    }
    return { selector: domPath(element), strategy: 'dom-path', confidence: 'low', confidenceScore: .4, needsBinding: true };
  }

  function identityOf(element) {
    const uiNode = element.getAttribute('data-ui-node') || '';
    const stableId = element.getAttribute('data-stable-id') || uiNode || '';
    const testId = element.getAttribute('data-testid') || '';
    const resourceId = element.getAttribute('data-resource-id') || '';
    const sourceSymbol = element.getAttribute('data-source-symbol') || element.getAttribute('data-source-node') || '';
    const component = element.closest('[data-component]');
    const componentPath = element.getAttribute('data-component-path') || (component && component.getAttribute('data-component')) || '';
    const id = element.id || '';
    const ariaLabel = element.getAttribute('aria-label') || '';
    const role = element.getAttribute('role') || '';
    const text = compactText(element);
    const selector = selectorIdentity(element);
    const key = stableId ? 'stable:' + stableId
      : testId ? 'test:' + testId
        : resourceId ? 'resource:' + resourceId
          : sourceSymbol ? 'symbol:' + sourceSymbol
            : componentPath ? 'component:' + componentPath
              : uiNode ? 'ui-node:' + uiNode
                : id ? 'id:' + id
                  : 'selector-evidence:' + selector.selector;
    return {
      key,
      selector: selector.selector,
      strategy: selector.strategy,
      confidence: selector.confidence,
      confidenceScore: selector.confidenceScore,
      needsBinding: selector.needsBinding,
      stableId,
      testId,
      resourceId,
      sourceSymbol,
      componentPath,
      uiNode,
      id,
      ariaLabel,
      role,
      text,
      tag: element.tagName.toLowerCase(),
      classNames: Array.from(element.classList).filter((name) => !name.startsWith('ui-tuner-')).slice(0, 12),
    };
  }

  function contextNode(element, relation) {
    if (!element) return null;
    const identity = identityOf(element);
    return {
      stableKey: identity.key,
      relation,
      tag: identity.tag,
      text: identity.text,
      role: identity.role,
    };
  }

  function localDomContext(element) {
    const parent = element.parentElement;
    const siblings = parent ? Array.from(parent.children).filter((candidate) => candidate !== element).slice(0, 4) : [];
    return [contextNode(parent, 'parent'), contextNode(element, 'self')]
      .concat(siblings.map((candidate) => contextNode(candidate, 'sibling')))
      .filter(Boolean);
  }

  function kebabCase(property) {
    return property.replace(/[A-Z]/g, (match) => '-' + match.toLowerCase());
  }

  function authoredValue(element, property) {
    const cssProperty = kebabCase(property);
    const inlineValue = element.style.getPropertyValue(cssProperty);
    if (inlineValue) return inlineValue;
    let matchedValue = '';
    function visitRules(rules) {
      Array.from(rules || []).forEach((rule) => {
        if (rule.cssRules) {
          visitRules(rule.cssRules);
          return;
        }
        if (!rule.selectorText || !rule.style) return;
        try {
          if (element.matches(rule.selectorText) && rule.style.getPropertyValue(cssProperty)) {
            matchedValue = rule.style.getPropertyValue(cssProperty);
          }
        } catch (_) {
          // Ignore unsupported selectors in authored stylesheets.
        }
      });
    }
    Array.from(document.styleSheets || []).forEach((sheet) => {
      try { visitRules(sheet.cssRules); } catch (_) { /* Cross-origin sheets are not inspected. */ }
    });
    return matchedValue;
  }

  function styleValues(element, computed, authored) {
    return editableProperties.reduce((result, property) => {
      const value = authored ? authoredValue(element, property) : computed[property];
      if (value !== undefined && value !== '') result[property] = String(value);
      return result;
    }, {});
  }

  function snapshotOf(element, knownRect) {
    const rect = knownRect || element.getBoundingClientRect();
    const computed = window.getComputedStyle(element);
    return {
      identity: identityOf(element),
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      originalStyle: {
        computed: styleValues(element, computed, false),
        authored: styleValues(element, computed, true),
        inlineStyle: element.getAttribute('style'),
      },
      domContext: localDomContext(element),
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

  function resolveTarget(payload) {
    if (payload && payload.selector) {
      try { return document.querySelector(payload.selector); } catch (_) { return null; }
    }
    return selectedElement && selectedElement.isConnected ? selectedElement : null;
  }

  function styleValue(property, value) {
    if (['width', 'height', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
      'marginTop', 'marginRight', 'marginBottom', 'marginLeft', 'borderRadius', 'fontSize'].includes(property)) {
      return cssDimension(value);
    }
    if (property === 'lineHeight' && /^-?\d+(\.\d+)?(dp|sp)$/.test(String(value).trim())) return cssDimension(value);
    return String(value).trim();
  }

  function applyStyle(payload, notify) {
    if (!payload || !payload.style) return;
    const element = resolveTarget(payload);
    if (!element || element === selection) return;
    const style = payload.style;
    rememberInlineStyles(element);
    editableProperties.forEach((property) => {
      if (style[property] === undefined) return;
      const cssProperty = kebabCase(property);
      if (style[property] === null || style[property] === '') element.style.removeProperty(cssProperty);
      else element.style.setProperty(cssProperty, styleValue(property, style[property]));
    });
    if (element === selectedElement) drawSelection(element);
    if (notify !== false) post('style-applied', { node: snapshotOf(element) });
  }

  function resetStyles(notify) {
    originalInlineStyles.forEach((original, element) => {
      if (!element.isConnected) return;
      if (original === null) element.removeAttribute('style');
      else element.setAttribute('style', original);
    });
    originalInlineStyles.clear();
    drawSelection(selectedElement);
    if (notify !== false) post('styles-reset', {});
  }

  function resetElement(payload) {
    const element = resolveTarget(payload);
    if (!element) return;
    if (originalInlineStyles.has(element)) {
      const original = originalInlineStyles.get(element);
      if (original === null) element.removeAttribute('style');
      else element.setAttribute('style', original);
      originalInlineStyles.delete(element);
    }
    drawSelection(selectedElement);
    post('element-reset', { node: snapshotOf(element) });
  }

  function applyDraft(payload) {
    resetStyles(false);
    const elements = payload && Array.isArray(payload.elements) ? payload.elements : [];
    elements.forEach((entry) => applyStyle({ selector: entry.selector, style: entry.styleDiff || {} }, false));
    drawSelection(selectedElement);
    post('draft-applied', { appliedCount: elements.length });
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
      applyStyle(message.payload, true);
    } else if (message.type === 'apply-draft') {
      applyDraft(message.payload);
    } else if (message.type === 'reset-element') {
      resetElement(message.payload);
    } else if (message.type === 'reset-styles') {
      resetStyles(true);
    }
  });

  post('ready', {
    href: window.location.href,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    mode: 'interact',
  });
  postRoute('ready');
})();
