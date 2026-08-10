(function (root, factory) {
  'use strict';

  const adapter = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptFormControls = Object.freeze(adapter);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const ACTIONABLE_SELECTOR = [
    'input:not([type="hidden"]):not([type="file"])',
    'textarea',
    'select',
    '[contenteditable="true"]',
    '[role="textbox"]',
    '[role="combobox"]',
    '[role="checkbox"]',
    '[role="radio"]',
    '[role="slider"]'
  ].join(', ');
  const TEXT_INPUT_KINDS = new Set([
    'text', 'search', 'email', 'url', 'tel', 'number', 'date', 'time',
    'datetime-local', 'month', 'week', 'textarea', 'contenteditable'
  ]);

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim();
  }

  function attribute(node, name) {
    return node && typeof node.getAttribute === 'function'
      ? String(node.getAttribute(name) || '')
      : '';
  }

  function tagName(node) {
    return String(node && node.tagName || '').toLowerCase();
  }

  function inputKind(node) {
    const explicitRole = attribute(node, 'role').toLowerCase();
    const tag = tagName(node);
    if (explicitRole === 'slider') return 'range';
    if (explicitRole === 'checkbox') return 'checkbox';
    if (explicitRole === 'radio') return 'radio';
    if (explicitRole === 'combobox' || tag === 'select') return 'select';
    if (node && node.isContentEditable) return 'contenteditable';
    if (tag === 'textarea') return 'textarea';
    if (tag === 'input') return String(node.type || attribute(node, 'type') || 'text').toLowerCase();
    return explicitRole === 'textbox' ? 'text' : '';
  }

  function role(node) {
    const explicit = attribute(node, 'role').toLowerCase();
    if (['textbox', 'combobox', 'checkbox', 'radio', 'slider'].includes(explicit)) return explicit;
    const kind = inputKind(node);
    if (kind === 'select') return 'combobox';
    if (kind === 'checkbox') return 'checkbox';
    if (kind === 'radio') return 'radio';
    if (kind === 'range') return 'slider';
    return kind ? 'textbox' : '';
  }

  function label(node) {
    const labels = node && node.labels ? Array.from(node.labels) : [];
    return clean([
      attribute(node, 'aria-label'),
      labels.map((item) => item && item.textContent).filter(Boolean).join(' '),
      attribute(node, 'placeholder'),
      attribute(node, 'title'),
      attribute(node, 'name'),
      attribute(node, 'data-testid')
    ].find((value) => clean(value)) || '输入');
  }

  function isDisabled(node) {
    return !!(node && node.disabled) || attribute(node, 'aria-disabled') === 'true';
  }

  function isReadOnly(node) {
    return !!(node && node.readOnly) || attribute(node, 'aria-readonly') === 'true';
  }

  function describe(node) {
    const resolvedRole = role(node);
    if (!resolvedRole) return null;
    const kind = inputKind(node);
    const sensitive = kind === 'password';
    const writable = resolvedRole === 'textbox' && TEXT_INPUT_KINDS.has(kind) &&
      !sensitive && !isDisabled(node) && !isReadOnly(node);
    return {
      role: resolvedRole,
      inputKind: kind,
      writable,
      sensitive,
      selected: !!(node && node.checked) || attribute(node, 'aria-checked') === 'true',
      label: label(node)
    };
  }

  function nativeValueSetter(node) {
    const tag = tagName(node);
    const constructorName = tag === 'textarea' ? 'HTMLTextAreaElement' : 'HTMLInputElement';
    const constructor = typeof globalThis !== 'undefined' ? globalThis[constructorName] : null;
    const descriptor = constructor && constructor.prototype
      ? Object.getOwnPropertyDescriptor(constructor.prototype, 'value')
      : null;
    return descriptor && descriptor.set;
  }

  function dispatchValueEvents(node, value) {
    if (!node || typeof node.dispatchEvent !== 'function') return;
    const inputEvent = typeof InputEvent === 'function'
      ? new InputEvent('input', { bubbles: true, inputType: 'insertText', data: null })
      : new Event('input', { bubbles: true });
    node.dispatchEvent(inputEvent);
    node.dispatchEvent(new Event('change', { bubbles: true }));
  }

  function setText(node, rawValue) {
    const details = describe(node);
    if (!details || details.role !== 'textbox') return { ok: false, reason: 'not_textbox' };
    if (details.sensitive) return { ok: false, reason: 'sensitive' };
    if (!details.writable) return { ok: false, reason: 'not_writable' };

    const value = String(rawValue == null ? '' : rawValue).slice(0, 20000);
    if (node && typeof node.focus === 'function') node.focus();
    if (details.inputKind === 'contenteditable') {
      node.textContent = value;
    } else {
      const setter = nativeValueSetter(node);
      if (setter) setter.call(node, value);
      else node.value = value;
    }
    dispatchValueEvents(node, value);
    return { ok: true, reason: '' };
  }

  return Object.freeze({ ACTIONABLE_SELECTOR, describe, setText });
});
