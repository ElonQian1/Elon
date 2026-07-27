(function installUiTunerPwaBridge() {
  'use strict';
  const params = new URLSearchParams(window.location.search || '');
  if (params.get('ui_tuner_preview') !== '1' || window.parent === window) return;

  const SOURCE = 'elon-pwa-design-bridge';
  const PARENT_SOURCE = 'elon-pc-ui-tuner';
  const PROTOCOL_VERSION = 1;
  const DRAFT_RETRY_LIMIT = 8;
  const DRAFT_RETRY_DELAY_MS = 120;
  const originalInlineStyles = new Map();
  let selectedElement = null;
  let selection = null;
  let selecting = false;
  let acceptedSessionToken = '';
  let completedDraftRevision = '';
  let activeDraft = null;
  let lastDraftAck = null;
  let draftRetryTimer = 0;
  let routeDebounceTimer = 0;
  let lastRouteSignature = '';
  let selectingTimeoutTimer = 0;
  const editableProperties = [
    'width', 'height',
    'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
    'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
    'borderRadius', 'fontSize', 'fontWeight', 'lineHeight',
    'color', 'backgroundColor', 'opacity',
  ];

  function post(type, payload) {
    window.parent.postMessage({ source: SOURCE, protocolVersion: PROTOCOL_VERSION, type, payload }, window.location.origin);
  }

  function postHealth(reason) {
    const ack = activeDraft ? draftAck(activeDraft, false) : lastDraftAck;
    post('health', {
      reason: reason || 'manual',
      ready: true,
      mode: selecting ? 'select' : 'interact',
      selected: Boolean(selectedElement && selectedElement.isConnected),
      editablePropertyCount: editableProperties.length,
      canApplyDraft: true,
      canVerifySource: typeof window.__ELON_UI_TUNER_VERIFY_SOURCE__ === 'function',
      draft: ack ? {
        requestedCount: ack.requestedCount,
        appliedCount: ack.appliedCount,
        unresolvedCount: Array.isArray(ack.unresolved) ? ack.unresolved.length : 0,
        complete: Boolean(ack.complete),
        revision: ack.revision,
        retrying: Boolean(ack.retrying),
        exhausted: Boolean(ack.exhausted),
      } : null,
      route: routeState('health'),
    });
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

  function normalizedScreenText(value, maxLength) {
    return String(value || '')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, maxLength);
  }

  function screenKeyPart(value) {
    return normalizedScreenText(value, 160).toLowerCase();
  }

  function stableElement(elements, identity) {
    return Array.from(elements || [])
      .filter((element) => element && normalizedScreenText(identity(element), 160))
      .sort((left, right) => {
        const leftKey = normalizedScreenText(identity(left), 160);
        const rightKey = normalizedScreenText(identity(right), 160);
        return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
      })[0] || null;
  }

  function activePageElement() {
    return stableElement(document.querySelectorAll('.page.active[id]'), (element) => element.id);
  }

  function explicitScreenElement(activePage) {
    if (activePage && normalizedScreenText(activePage.getAttribute('data-ui-screen'), 160)) return activePage;
    const activeParent = activePage && typeof activePage.closest === 'function'
      ? activePage.closest('[data-ui-screen]')
      : null;
    if (activeParent && normalizedScreenText(activeParent.getAttribute('data-ui-screen'), 160)) return activeParent;
    const activeChildren = activePage && typeof activePage.querySelectorAll === 'function'
      ? activePage.querySelectorAll('[data-ui-screen]')
      : [];
    const scopedExplicit = stableElement(activeChildren, (element) => element.getAttribute('data-ui-screen'));
    if (scopedExplicit) return scopedExplicit;
    const statefulExplicit = stableElement(
      document.querySelectorAll('[data-ui-screen].active, [data-ui-screen][aria-hidden="false"]'),
      (element) => element.getAttribute('data-ui-screen'),
    );
    if (statefulExplicit) return statefulExplicit;
    return [document.body, document.documentElement]
      .find((element) => element && normalizedScreenText(element.getAttribute('data-ui-screen'), 160)) || null;
  }

  function screenIdentity() {
    const activePage = activePageElement();
    const topTitle = document.querySelector('#topTitle');
    const visibleTitle = normalizedScreenText(topTitle && compactText(topTitle), 160);
    const explicit = explicitScreenElement(activePage);
    if (explicit) {
      const explicitKey = screenKeyPart(explicit.getAttribute('data-ui-screen'));
      const explicitTitle = normalizedScreenText(
        explicit.getAttribute('data-ui-screen-title')
          || explicit.getAttribute('aria-label')
          || explicit.getAttribute('title')
          || visibleTitle
          || explicit.getAttribute('data-ui-screen'),
        160,
      );
      return { screenKey: 'data-ui-screen:' + explicitKey, screenTitle: explicitTitle };
    }
    if (activePage && activePage.id) {
      const title = visibleTitle
        || normalizedScreenText(activePage.getAttribute('data-title') || activePage.getAttribute('aria-label'), 160)
        || normalizedScreenText(document.title, 160)
        || activePage.id;
      return {
        screenKey: 'page:' + activePage.id + '|title:' + (screenKeyPart(title) || 'untitled'),
        screenTitle: title,
      };
    }
    return {
      screenKey: 'screen:unidentified',
      screenTitle: visibleTitle || normalizedScreenText(document.title, 160) || '未识别画面',
    };
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

  function explicitStyleBinding(element) {
    const serialized = element.getAttribute('data-ui-style-binding') || '';
    if (!serialized || serialized.length > 4000) return null;
    let input;
    try { input = JSON.parse(serialized); } catch (_) { return null; }
    const sourceFile = String(input && input.sourceFile || '').trim().replace(/\\/g, '/');
    const sourceRevision = String(input && input.sourceRevision || '').trim().toLowerCase();
    const kind = String(input && input.kind || '');
    const target = String(input && input.target || '').trim();
    const range = input && input.range;
    const unsafePath = !sourceFile || sourceFile.startsWith('/') || /^[a-z]:\//i.test(sourceFile)
      || sourceFile.split('/').some((segment) => !segment || segment === '.' || segment === '..');
    if (!input || input.version !== 1 || unsafePath || !/^[a-f0-9]{64}$/.test(sourceRevision)
      || !['css-rule', 'style-object', 'token-json'].includes(kind)
      || !target || target.length > 240 || !range
      || !Number.isSafeInteger(range.start) || !Number.isSafeInteger(range.end)
      || range.start < 0 || range.end <= range.start
      || !input.propertyMap || typeof input.propertyMap !== 'object') return null;
    const allowed = new Set(editableProperties);
    const propertyMap = Object.entries(input.propertyMap).reduce((result, entry) => {
      const property = entry[0];
      const sourceProperty = String(entry[1] || '').trim();
      if (allowed.has(property) && sourceProperty && sourceProperty.length <= 160
        && /^[a-zA-Z_$][\w$.-]*$/.test(sourceProperty)) result[property] = sourceProperty;
      return result;
    }, {});
    if (!Object.keys(propertyMap).length || Object.keys(propertyMap).length !== Object.keys(input.propertyMap).length) return null;
    return {
      version: 1,
      sourceFile,
      sourceRevision,
      kind,
      target,
      range: { start: range.start, end: range.end },
      propertyMap,
    };
  }

  function kebabCase(property) {
    return property.replace(/[A-Z]/g, (match) => '-' + match.toLowerCase());
  }

  function sourceSelectorPriority(selector) {
    const stableAttributes = (selector.match(/\[(?:data-ui-node|data-testid|data-test-id|data-resource-id|aria-label)\b/gi) || []).length;
    const ids = (selector.match(/#[a-zA-Z_][\w-]*/g) || []).length;
    const classesAndAttributes = (selector.match(/\.[a-zA-Z_][\w-]*|\[[^\]]+\]/g) || []).length;
    const transient = /:(?:hover|active|focus|focus-visible|focus-within|visited|target|checked|disabled|enabled)\b/i.test(selector);
    return stableAttributes * 1000000 + ids * 100000 + classesAndAttributes * 100 - (transient ? 10000000 : 0);
  }

  function inspectAuthoredStyles(element) {
    const values = {};
    const selectors = [];
    function visitRules(rules) {
      Array.from(rules || []).forEach((rule) => {
        if (rule.cssRules) {
          visitRules(rule.cssRules);
          return;
        }
        if (!rule.selectorText || !rule.style) return;
        try {
          if (!element.matches(rule.selectorText)) return;
          let editable = false;
          editableProperties.forEach((property) => {
            const value = rule.style.getPropertyValue(kebabCase(property));
            if (!value) return;
            values[property] = value;
            editable = true;
          });
          if (editable && selectors.length < 16 && !selectors.includes(rule.selectorText)) selectors.push(rule.selectorText);
        } catch (_) {
          // Ignore unsupported selectors in authored stylesheets.
        }
      });
    }
    Array.from(document.styleSheets || []).forEach((sheet) => {
      try { visitRules(sheet.cssRules); } catch (_) { /* Cross-origin sheets are not inspected. */ }
    });
    editableProperties.forEach((property) => {
      const inlineValue = element.style.getPropertyValue(kebabCase(property));
      if (inlineValue) values[property] = inlineValue;
    });
    selectors.sort((left, right) => sourceSelectorPriority(right) - sourceSelectorPriority(left));
    return { values, selectors };
  }

  function computedStyleValues(computed) {
    return editableProperties.reduce((result, property) => {
      const value = computed[property];
      if (value !== undefined && value !== '') result[property] = String(value);
      return result;
    }, {});
  }

  function snapshotOf(element, knownRect) {
    const rect = knownRect || element.getBoundingClientRect();
    const computed = window.getComputedStyle(element);
    const authored = inspectAuthoredStyles(element);
    return {
      identity: identityOf(element),
      rect: { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      originalStyle: {
        computed: computedStyleValues(computed),
        authored: authored.values,
        inlineStyle: element.getAttribute('style'),
      },
      domContext: localDomContext(element),
      sourceSelectors: authored.selectors,
      sourceBinding: explicitStyleBinding(element),
    };
  }

  function routeState(reason) {
    const screen = screenIdentity();
    return {
      reason,
      href: window.location.href,
      path: window.location.pathname,
      search: window.location.search,
      hash: window.location.hash,
      title: document.title,
      screenKey: screen.screenKey,
      screenTitle: screen.screenTitle,
      viewport: { width: window.innerWidth, height: window.innerHeight },
      scroll: { x: window.scrollX, y: window.scrollY },
    };
  }

  function postRoute(reason) {
    const state = routeState(reason);
    const signature = [
      state.path, state.search, state.hash, state.screenKey,
      state.viewport.width + 'x' + state.viewport.height,
    ].join('|');
    scheduleDraftRetry('route');
    if (signature === lastRouteSignature) return;
    lastRouteSignature = signature;
    post('route-changed', state);
  }

  function scheduleRoute(reason) {
    if (routeDebounceTimer) return;
    routeDebounceTimer = window.setTimeout(() => {
      routeDebounceTimer = 0;
      postRoute(reason);
    }, 80);
  }

  function findDesignTarget(target) {
    if (!(target instanceof Element) || target === selection) return null;
    return target.closest('[data-ui-node],[id],button,input,textarea,select,a,[role="button"],.tab,.conversation-item,.project-card')
      || target.closest('header,nav,main,section,article,div,span');
  }

  function drawSelection(element, knownRect) {
    if (!selection) return;
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

  function cancelDraftRetry() {
    if (!draftRetryTimer) return;
    window.clearTimeout(draftRetryTimer);
    draftRetryTimer = 0;
  }

  function clearDraftTracking() {
    cancelDraftRetry();
    completedDraftRevision = '';
    activeDraft = null;
    lastDraftAck = null;
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

  function applyStyleToElement(element, style, notify) {
    if (!element || element === selection || !style) return false;
    rememberInlineStyles(element);
    editableProperties.forEach((property) => {
      if (style[property] === undefined) return;
      const cssProperty = kebabCase(property);
      if (style[property] === null || style[property] === '') element.style.removeProperty(cssProperty);
      else element.style.setProperty(cssProperty, styleValue(property, style[property]));
    });
    if (element === selectedElement) drawSelection(element);
    if (notify !== false) post('style-applied', { node: snapshotOf(element) });
    return true;
  }

  function applyStyle(payload, notify) {
    if (!payload || !payload.style) return false;
    return applyStyleToElement(resolveTarget(payload), payload.style, notify);
  }

  function resetStyles(notify) {
    originalInlineStyles.forEach((original, element) => {
      if (!element.isConnected) return;
      if (original === null) element.removeAttribute('style');
      else element.setAttribute('style', original);
    });
    originalInlineStyles.clear();
    clearDraftTracking();
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

  function draftIdentityMatch(element, expected) {
    if (!expected || typeof expected !== 'object') return { matches: false, reason: 'identity-insufficient' };
    const actual = identityOf(element);
    const stableFields = ['stableId', 'testId', 'resourceId', 'sourceSymbol', 'componentPath', 'uiNode', 'id'];
    const expectedStable = stableFields.filter((field) => String(expected[field] || ''));
    for (const field of expectedStable) {
      if (String(actual[field] || '') !== String(expected[field] || '')) {
        return { matches: false, reason: 'identity-mismatch' };
      }
    }
    if (expectedStable.length) {
      if (expected.tag && actual.tag !== String(expected.tag).toLowerCase()) {
        return { matches: false, reason: 'identity-mismatch' };
      }
      return { matches: true, reason: '' };
    }
    const expectedTag = String(expected.tag || '').toLowerCase();
    if (!expectedTag) return { matches: false, reason: 'identity-insufficient' };
    if (actual.tag !== expectedTag) return { matches: false, reason: 'identity-mismatch' };
    const semanticFields = ['ariaLabel', 'role', 'text'];
    const expectedSemantic = semanticFields.filter((field) => String(expected[field] || ''));
    const expectedClasses = Array.isArray(expected.classNames)
      ? expected.classNames.map((value) => String(value || '')).filter(Boolean).slice(0, 12)
      : [];
    if (!expectedSemantic.length && !expectedClasses.length) {
      return { matches: false, reason: 'identity-insufficient' };
    }
    for (const field of expectedSemantic) {
      if (String(actual[field] || '') !== String(expected[field] || '')) {
        return { matches: false, reason: 'identity-mismatch' };
      }
    }
    if (expectedClasses.some((name) => !actual.classNames.includes(name))) {
      return { matches: false, reason: 'identity-mismatch' };
    }
    return { matches: true, reason: '' };
  }

  function draftAck(state, exhausted) {
    const unresolved = state.entries
      .filter((entry) => entry.status !== 'applied')
      .map((entry) => ({
        index: entry.index,
        selector: entry.selector,
        identityKey: String(entry.identity && entry.identity.key || ''),
        reason: entry.reason || 'target-missing',
      }));
    const appliedCount = state.entries.length - unresolved.length;
    return {
      requestedCount: state.entries.length,
      appliedCount,
      unresolved,
      complete: appliedCount === state.entries.length,
      draftKey: state.draftKey,
      revision: state.revision,
      attempt: state.attempt,
      maxAttempts: DRAFT_RETRY_LIMIT,
      retrying: appliedCount !== state.entries.length && !exhausted,
      exhausted: appliedCount !== state.entries.length && exhausted,
    };
  }

  function scheduleDraftRetry() {
    if (!activeDraft || draftRetryTimer || activeDraft.attempt >= DRAFT_RETRY_LIMIT) return;
    const retryable = activeDraft.entries.some((entry) => entry.status === 'pending');
    if (!retryable) return;
    draftRetryTimer = window.setTimeout(() => {
      draftRetryTimer = 0;
      attemptDraft();
    }, DRAFT_RETRY_DELAY_MS);
  }

  function attemptDraft() {
    const state = activeDraft;
    if (!state) return;
    state.attempt += 1;
    state.entries.forEach((entry) => {
      if (entry.status !== 'pending') return;
      let element = null;
      try { element = entry.selector ? document.querySelector(entry.selector) : null; } catch (_) { /* Invalid selectors stay unresolved. */ }
      if (!element || element === selection) {
        entry.reason = 'target-missing';
        return;
      }
      const identity = draftIdentityMatch(element, entry.identity);
      if (!identity.matches) {
        entry.status = 'failed';
        entry.reason = identity.reason;
        return;
      }
      if (applyStyleToElement(element, entry.styleDiff, false)) {
        entry.status = 'applied';
        entry.reason = '';
      }
    });
    drawSelection(selectedElement);
    const hasRetryable = state.entries.some((entry) => entry.status === 'pending');
    const exhausted = !hasRetryable || state.attempt >= DRAFT_RETRY_LIMIT;
    const acknowledgement = draftAck(state, exhausted);
    lastDraftAck = acknowledgement;
    post('draft-applied', acknowledgement);
    postHealth('draft-applied');
    if (acknowledgement.complete) {
      completedDraftRevision = state.revisionKey;
      activeDraft = null;
      cancelDraftRetry();
      return;
    }
    if (!exhausted) scheduleDraftRetry();
  }

  function applyDraft(payload) {
    const draftKey = String(payload && payload.draftKey || '');
    const revision = Number(payload && payload.revision);
    const revisionKey = draftKey && Number.isInteger(revision) && revision >= 0
      ? draftKey + '@' + revision
      : '';
    if (revisionKey && revisionKey === completedDraftRevision && lastDraftAck) {
      post('draft-applied', lastDraftAck);
      return;
    }
    if (revisionKey && activeDraft && revisionKey === activeDraft.revisionKey) {
      post('draft-applied', lastDraftAck || draftAck(activeDraft, false));
      scheduleDraftRetry();
      return;
    }
    resetStyles(false);
    const elements = payload && Array.isArray(payload.elements) ? payload.elements : [];
    activeDraft = {
      draftKey,
      revision,
      revisionKey,
      attempt: 0,
      entries: elements.map((entry, index) => ({
        index,
        selector: String(entry && entry.selector || ''),
        identity: entry && entry.identity,
        styleDiff: entry && entry.styleDiff && typeof entry.styleDiff === 'object' ? entry.styleDiff : {},
        status: 'pending',
        reason: 'target-missing',
      })),
    };
    attemptDraft();
  }

  function handleDesignClick(event) {
    const target = findDesignTarget(event.target);
    if (!target) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    selectElement(target, 'click');
    setSelecting(false);
  }

  function clearSelectingTimeout() {
    if (!selectingTimeoutTimer) return;
    window.clearTimeout(selectingTimeoutTimer);
    selectingTimeoutTimer = 0;
  }

  function setSelecting(enabled) {
    if (selecting === enabled) return;
    selecting = enabled;
    if (selecting) {
      clearSelectingTimeout();
      selection = document.createElement('div');
      selection.id = 'uiTunerPreviewSelection';
      selection.setAttribute('aria-hidden', 'true');
      document.body.appendChild(selection);
      document.body.classList.add('ui-tuner-preview-active', 'ui-tuner-preview-selecting');
      document.addEventListener('click', handleDesignClick, true);
      selectingTimeoutTimer = window.setTimeout(() => {
        if (!selecting) return;
        setSelecting(false);
        post('bridge-notice', { level: 'info', message: '选择组件已超时取消；页面已恢复正常操作' });
      }, 15000);
      drawSelection(selectedElement);
      return;
    }
    clearSelectingTimeout();
    document.removeEventListener('click', handleDesignClick, true);
    if (selection) selection.remove();
    selection = null;
    document.body.classList.remove('ui-tuner-preview-active', 'ui-tuner-preview-selecting');
  }

  window.addEventListener('scroll', () => drawSelection(selectedElement), true);
  window.addEventListener('resize', () => {
    drawSelection(selectedElement);
    scheduleRoute('resize');
  });
  window.addEventListener('keydown', (event) => {
    if (!selecting || event.key !== 'Escape') return;
    event.preventDefault();
    event.stopImmediatePropagation();
    setSelecting(false);
    post('bridge-notice', { level: 'info', message: '已退出选择组件模式；页面恢复正常操作' });
  }, true);
  window.addEventListener('hashchange', () => postRoute('hashchange'));
  window.addEventListener('popstate', () => postRoute('popstate'));
  window.addEventListener('load', () => scheduleDraftRetry('load'));
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => scheduleDraftRetry('dom-ready'), { once: true });
  }
  ['pushState', 'replaceState'].forEach((method) => {
    const original = history[method];
    history[method] = function uiTunerHistoryChange() {
      const result = original.apply(this, arguments);
      postRoute(method);
      return result;
    };
  });

  if (typeof MutationObserver === 'function') {
    const screenObserver = new MutationObserver(() => {
      scheduleRoute('screen-mutation');
      scheduleDraftRetry('mutation');
    });
    screenObserver.observe(document.body, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class', 'aria-hidden', 'data-ui-screen', 'data-ui-screen-title', 'id'],
    });
  }

  window.addEventListener('message', (event) => {
    if (event.origin !== window.location.origin || event.source !== window.parent) return;
    const message = event.data || {};
    if (message.source !== PARENT_SOURCE || message.protocolVersion !== PROTOCOL_VERSION) return;
    if (message.type === 'set-mode') {
      setSelecting(Boolean(message.payload && message.payload.mode !== 'interact'));
      post('mode-changed', { mode: selecting ? 'select' : 'interact' });
      postHealth('mode-changed');
    } else if (message.type === 'set-session-auth') {
      const sessionToken = String(message.payload && message.payload.token || '');
      if (!sessionToken || sessionToken.length > 8192) return;
      if (sessionToken === acceptedSessionToken) return;
      acceptedSessionToken = sessionToken;
      window.dispatchEvent(new CustomEvent('elon:ui-tuner-session-auth', {
        detail: { token: sessionToken },
      }));
      post('session-auth-accepted', {});
    } else if (message.type === 'apply-style') {
      applyStyle(message.payload, true);
    } else if (message.type === 'apply-draft') {
      applyDraft(message.payload);
    } else if (message.type === 'health-check') {
      postHealth('health-check');
    } else if (message.type === 'reset-element') {
      resetElement(message.payload);
    } else if (message.type === 'reset-styles') {
      resetStyles(true);
    } else if (message.type === 'verify-source') {
      resetStyles(false);
      if (typeof window.__ELON_UI_TUNER_VERIFY_SOURCE__ === 'function') {
        window.__ELON_UI_TUNER_VERIFY_SOURCE__(message.payload, routeState('source-verification'));
      }
    }
  });

  post('ready', {
    href: window.location.href,
    viewport: { width: window.innerWidth, height: window.innerHeight },
    mode: 'interact',
  });
  postRoute('ready');
  postHealth('ready');
})();
