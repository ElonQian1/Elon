(function () {
  'use strict';
  const params = new URLSearchParams(window.location.search);
  if (params.get('ui_tuner_preview') !== '1' || window.parent === window) return;

  const SOURCE = 'elon-pwa-design-bridge';
  const PROTOCOL_VERSION = 1;
  const editableProperties = new Set([
    'width', 'height', 'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
    'marginTop', 'marginRight', 'marginBottom', 'marginLeft', 'borderRadius',
    'fontSize', 'fontWeight', 'lineHeight', 'color', 'backgroundColor', 'opacity',
  ]);
  let lastRequestId = '';

  function kebabCase(property) {
    return property.replace(/[A-Z]/g, (match) => '-' + match.toLowerCase());
  }

  function authoredValue(element, property) {
    const cssProperty = kebabCase(property);
    const inline = element.style.getPropertyValue(cssProperty);
    if (inline) return inline;
    let matched = '';
    function visit(rules) {
      Array.from(rules || []).forEach((rule) => {
        if (rule.cssRules) return visit(rule.cssRules);
        if (!rule.selectorText || !rule.style) return;
        try {
          if (element.matches(rule.selectorText) && rule.style.getPropertyValue(cssProperty)) {
            matched = rule.style.getPropertyValue(cssProperty);
          }
        } catch (_) { /* Ignore unsupported or inaccessible selectors. */ }
      });
    }
    Array.from(document.styleSheets || []).forEach((sheet) => {
      try { visit(sheet.cssRules); } catch (_) { /* Ignore cross-origin sheets. */ }
    });
    return matched;
  }

  function metadata(name, fallback) {
    const node = document.querySelector('meta[name="' + name + '"]');
    const value = String(node && node.getAttribute('content') || '').trim();
    if (!value) return fallback;
    try { return JSON.parse(value); } catch (_) { return value; }
  }

  function verify(request, route) {
    const requestId = String(request && request.requestId || '').trim();
    const checks = request && Array.isArray(request.checks) ? request.checks.slice(0, 64) : [];
    if (!requestId || requestId.length > 160 || requestId === lastRequestId) return;
    lastRequestId = requestId;
    const nodes = checks.map((check) => {
      const elementKey = String(check && check.elementKey || '').slice(0, 400);
      const selector = String(check && check.selector || '').slice(0, 1000);
      let element = null;
      try { element = selector ? document.querySelector(selector) : null; } catch (_) { element = null; }
      const result = { elementKey, selector, found: Boolean(element), computed: {}, authored: {} };
      if (!element) return result;
      const computed = window.getComputedStyle(element);
      const properties = Array.isArray(check.properties) ? check.properties.filter((value) => editableProperties.has(value)) : [];
      properties.forEach((property) => {
        const computedValue = computed[property];
        const authored = authoredValue(element, property);
        if (computedValue !== undefined && computedValue !== '') result.computed[property] = String(computedValue);
        if (authored) result.authored[property] = String(authored);
      });
      return result;
    });
    const changedFiles = metadata('elon-ui-changed-files', []);
    const sourceRevisions = metadata('elon-ui-source-revisions', {});
    window.parent.postMessage({
      source: SOURCE,
      protocolVersion: PROTOCOL_VERSION,
      type: 'source-verification',
      payload: {
        requestId,
        route,
        sourceRevision: String(metadata('elon-ui-source-revision', '')),
        sourceRevisions: sourceRevisions && typeof sourceRevisions === 'object' ? sourceRevisions : {},
        changedFiles: Array.isArray(changedFiles) ? changedFiles.map(String) : [],
        nodes,
      },
    }, window.location.origin);
  }

  window.__ELON_UI_TUNER_VERIFY_SOURCE__ = verify;
})();
