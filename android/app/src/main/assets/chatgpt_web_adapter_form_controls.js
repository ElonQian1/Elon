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
    '[role="menuitemcheckbox"]',
    '[role="menuitemradio"]',
    '[role="switch"]',
    '[role="tab"]',
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
    if (explicitRole === 'switch') return 'switch';
    if (explicitRole === 'tab') return 'tab';
    if (explicitRole === 'checkbox' || explicitRole === 'menuitemcheckbox') return 'checkbox';
    if (explicitRole === 'radio' || explicitRole === 'menuitemradio') return 'radio';
    if (explicitRole === 'combobox' || tag === 'select') return 'select';
    if (node && node.isContentEditable) return 'contenteditable';
    if (tag === 'textarea') return 'textarea';
    if (tag === 'input') return String(node.type || attribute(node, 'type') || 'text').toLowerCase();
    return explicitRole === 'textbox' ? 'text' : '';
  }

  function role(node) {
    const explicit = attribute(node, 'role').toLowerCase();
    if ([
      'textbox', 'combobox', 'checkbox', 'radio', 'menuitemcheckbox',
      'menuitemradio', 'switch', 'tab', 'slider'
    ].includes(explicit)) return explicit;
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

  function selectChoices(node) {
    if (tagName(node) !== 'select' || !node || !node.options) return [];
    return Array.from(node.options).slice(0, 50).map((option) =>
      clean(option && (option.label || option.textContent)) || '选项'
    );
  }

  function nativeSliderDetails(node, kind) {
    if (tagName(node) !== 'input' || kind !== 'range') return null;
    const min = Number(node.min || attribute(node, 'min') || 0);
    const max = Number(node.max || attribute(node, 'max') || 100);
    const rawStep = String(node.step || attribute(node, 'step') || '1').toLowerCase();
    const step = rawStep === 'any' ? NaN : Number(rawStep);
    const value = Number(node.value);
    const rawStepCount = (max - min) / step;
    const stepCount = Math.round(rawStepCount);
    if (![min, max, step, value].every(Number.isFinite) || max <= min || step <= 0) return null;
    if (Math.abs(rawStepCount - stepCount) > 1e-7 || stepCount < 1 || stepCount > 10000) return null;
    return { min, max, step, value: Math.min(max, Math.max(min, value)) };
  }

  function describe(node) {
    const resolvedRole = role(node);
    if (!resolvedRole) return null;
    const kind = inputKind(node);
    const sensitive = kind === 'password';
    const writable = resolvedRole === 'textbox' && TEXT_INPUT_KINDS.has(kind) &&
      !sensitive && !isDisabled(node) && !isReadOnly(node);
    const choiceLabels = selectChoices(node);
    const selectedChoiceIndex = choiceLabels.length && Number.isInteger(node.selectedIndex)
      ? node.selectedIndex
      : -1;
    const slider = nativeSliderDetails(node, kind);
    return {
      role: resolvedRole,
      inputKind: kind,
      writable,
      sensitive,
      selected: !!(node && node.checked) ||
        attribute(node, 'aria-checked') === 'true' ||
        (resolvedRole === 'tab' && attribute(node, 'aria-selected') === 'true'),
      stateSettable: [
        'checkbox', 'radio', 'menuitemcheckbox', 'menuitemradio', 'switch', 'tab'
      ].includes(resolvedRole) && !isDisabled(node),
      choiceLabels,
      selectedChoiceIndex,
      sliderSettable: !!slider && !isDisabled(node) && !isReadOnly(node),
      sliderMin: slider && slider.min,
      sliderMax: slider && slider.max,
      sliderStep: slider && slider.step,
      sliderValue: slider && slider.value,
      label: label(node)
    };
  }

  function semantic(details) {
    if (!details) return '';
    if (details.role === 'textbox') return 'text_input';
    if (['combobox', 'tab'].includes(details.role)) return 'selection';
    if ([
      'checkbox', 'radio', 'menuitemcheckbox', 'menuitemradio', 'switch'
    ].includes(details.role)) return 'toggle';
    if (details.role === 'slider') return 'slider';
    return '';
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

  function planSelectedState(node, rawSelected) {
    const details = describe(node);
    if (!details || !details.stateSettable) return { ok: false, reason: 'not_settable' };
    const selected = rawSelected === true;
    if (['radio', 'menuitemradio'].includes(details.role) && !selected) {
      return { ok: false, reason: 'radio_cannot_clear' };
    }
    if (details.role === 'tab' && !selected) {
      return { ok: false, reason: 'tab_cannot_clear' };
    }
    return {
      ok: true,
      reason: '',
      selected,
      needsActivation: details.selected !== selected
    };
  }

  function selectChoice(node, rawIndex) {
    const details = describe(node);
    if (!details || details.inputKind !== 'select' || tagName(node) !== 'select') {
      return { ok: false, reason: 'not_native_select' };
    }
    if (isDisabled(node)) return { ok: false, reason: 'disabled' };
    const index = Number(rawIndex);
    if (!Number.isInteger(index) || index < 0 || index >= details.choiceLabels.length) {
      return { ok: false, reason: 'invalid_choice' };
    }
    const option = node.options && node.options[index];
    if (!option || option.disabled) return { ok: false, reason: 'disabled_choice' };
    if (node.selectedIndex === index) return { ok: true, reason: '', changed: false };
    const constructor = typeof HTMLSelectElement === 'function' ? HTMLSelectElement : null;
    const descriptor = constructor && constructor.prototype
      ? Object.getOwnPropertyDescriptor(constructor.prototype, 'selectedIndex')
      : null;
    if (descriptor && descriptor.set) descriptor.set.call(node, index);
    else node.selectedIndex = index;
    if (typeof node.focus === 'function') node.focus();
    dispatchValueEvents(node);
    return { ok: true, reason: '', changed: true };
  }

  function setSliderValue(node, rawValue) {
    const details = describe(node);
    if (!details || !details.sliderSettable) return { ok: false, reason: 'not_settable' };
    const requested = Number(rawValue);
    if (!Number.isFinite(requested)) return { ok: false, reason: 'invalid_value' };
    const stepIndex = Math.round((requested - details.sliderMin) / details.sliderStep);
    const normalized = Math.min(
      details.sliderMax,
      Math.max(details.sliderMin, details.sliderMin + stepIndex * details.sliderStep)
    );
    const tolerance = Math.max(1e-9, Math.abs(details.sliderStep) * 1e-7);
    if (Math.abs(details.sliderValue - normalized) <= tolerance) {
      return { ok: true, reason: '', changed: false };
    }
    const setter = nativeValueSetter(node);
    if (setter) setter.call(node, String(normalized));
    else node.value = String(normalized);
    if (typeof node.focus === 'function') node.focus();
    dispatchValueEvents(node);
    return { ok: true, reason: '', changed: true };
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

  return Object.freeze({
    ACTIONABLE_SELECTOR,
    describe,
    planSelectedState,
    semantic,
    selectChoice,
    setSliderValue,
    setText
  });
});
