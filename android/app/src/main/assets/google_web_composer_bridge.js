(function (root, factory) {
  'use strict';

  const api = factory(root);
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebComposerBridge ||
      Number(root.__elonGoogleWebComposerBridge.version || 0) < api.version)) {
    root.__elonGoogleWebComposerBridge = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function (root) {
  'use strict';

  const ENTRY_SELECTOR = [
    'textarea',
    'input[type="text"]',
    'input[type="search"]',
    'input:not([type])',
    '[role="searchbox"]',
    '[role="textbox"]',
    '[role="combobox"]',
    '[contenteditable="true"]',
    '[contenteditable="plaintext-only"]'
  ].join(',');
  const POSITIVE_LABEL = /ask|anything|follow.?up|chat|prompt|question|提问|尽情|追问|输入|询问/i;

  function cleanText(value) {
    return String(value || '').replace(/\s+/g, ' ').trim();
  }

  function scoreMeta(meta) {
    if (!meta || !meta.visible || meta.disabled || meta.inNavigation) return -1000;
    let score = 0;
    if (meta.tag === 'textarea') score += 45;
    if (meta.contentEditable) score += 36;
    if (meta.role === 'textbox' || meta.role === 'combobox') score += 24;
    if (meta.role === 'searchbox' || meta.type === 'search') score += 8;
    if (meta.positiveLabel) score += 90;
    if (meta.bottomHalf) score += 30;
    if (meta.formOwned) score += 8;
    return score;
  }

  function visible(node) {
    if (!node || node.nodeType !== 1) return false;
    const view = node.ownerDocument && node.ownerDocument.defaultView;
    if (!view) return false;
    const rect = node.getBoundingClientRect();
    const style = view.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden';
  }

  function documents() {
    if (!root || !root.document) return [];
    const values = [root.document];
    for (let index = 0; index < values.length && values.length < 12; index += 1) {
      const doc = values[index];
      for (const frame of doc.querySelectorAll('iframe')) {
        try {
          if (frame.contentDocument && !values.includes(frame.contentDocument)) {
            values.push(frame.contentDocument);
          }
        } catch (_) {}
      }
    }
    return values;
  }

  function roots(doc) {
    const values = [doc];
    for (let index = 0; index < values.length && values.length < 240; index += 1) {
      const scope = values[index];
      for (const node of scope.querySelectorAll('*')) {
        if (node.shadowRoot && !values.includes(node.shadowRoot)) values.push(node.shadowRoot);
      }
    }
    return values;
  }

  function candidates() {
    const seen = new Set();
    const values = [];
    for (const doc of documents()) {
      for (const scope of roots(doc)) {
        for (const node of scope.querySelectorAll(ENTRY_SELECTOR)) {
          if (seen.has(node)) continue;
          seen.add(node);
          const view = node.ownerDocument.defaultView;
          const rect = node.getBoundingClientRect();
          const label = cleanText([
            node.getAttribute('aria-label'),
            node.getAttribute('placeholder'),
            node.getAttribute('title'),
            node.getAttribute('name')
          ].filter(Boolean).join(' '));
          const meta = {
            tag: node.tagName.toLowerCase(),
            type: cleanText(node.getAttribute('type')).toLowerCase(),
            role: cleanText(node.getAttribute('role')).toLowerCase(),
            contentEditable: node.isContentEditable || /^(true|plaintext-only)$/i.test(
              cleanText(node.getAttribute('contenteditable'))
            ),
            visible: visible(node),
            disabled: node.matches(':disabled') || node.getAttribute('aria-disabled') === 'true',
            inNavigation: !!node.closest('header, nav, [role="navigation"], [role="tablist"]'),
            positiveLabel: POSITIVE_LABEL.test(label),
            bottomHalf: rect.bottom >= Math.max(1, view.innerHeight) * 0.5,
            formOwned: !!node.closest('form')
          };
          values.push({ node, meta, score: scoreMeta(meta), bottom: rect.bottom });
        }
      }
    }
    return values;
  }

  function find() {
    return candidates()
      .filter((candidate) => candidate.score > 0)
      .sort((left, right) => right.score - left.score || right.bottom - left.bottom)[0]?.node || null;
  }

  function value(composer) {
    if (!composer) return '';
    const tag = composer.tagName.toLowerCase();
    if (tag === 'textarea' || tag === 'input') return String(composer.value || '');
    return String(composer.innerText || composer.textContent || '');
  }

  function setValue(composer, nextValue) {
    if (!composer) return false;
    composer.focus();
    const view = composer.ownerDocument.defaultView;
    const tag = composer.tagName.toLowerCase();
    if (tag === 'textarea' || tag === 'input') {
      const prototype = tag === 'textarea' ? view.HTMLTextAreaElement.prototype : view.HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(prototype, 'value');
      if (!setter || typeof setter.set !== 'function') return false;
      setter.set.call(composer, nextValue);
    } else {
      composer.textContent = nextValue;
    }
    const InputEventType = view.InputEvent || view.Event;
    composer.dispatchEvent(new InputEventType('input', {
      bubbles: true,
      composed: true,
      inputType: 'insertText',
      data: nextValue
    }));
    composer.dispatchEvent(new view.Event('change', { bubbles: true }));
    return cleanText(value(composer)) === cleanText(nextValue);
  }

  function form(composer) {
    return composer && composer.closest ? composer.closest('form') : null;
  }

  function findAction(composer, labels) {
    if (!composer) return null;
    const needles = labels.map((label) => label.toLowerCase());
    const scopes = [form(composer), composer.getRootNode(), composer.ownerDocument].filter(Boolean);
    const seen = new Set();
    const matches = [];
    for (const scope of scopes) {
      for (const node of scope.querySelectorAll('button, [role="button"]')) {
        if (seen.has(node) || !visible(node) || node.matches(':disabled') ||
            node.getAttribute('aria-disabled') === 'true') continue;
        seen.add(node);
        const label = cleanText([
          node.getAttribute('aria-label'), node.getAttribute('title'), node.textContent
        ].filter(Boolean).join(' ')).toLowerCase();
        if (needles.some((needle) => label.includes(needle))) matches.push(node);
      }
      if (matches.length) break;
    }
    return matches[0] || null;
  }

  function scoreSubmitAction(meta) {
    if (!meta || !meta.visible || meta.disabled || meta.negativeLabel) return -1000;
    let score = 0;
    if (meta.positiveLabel) score += 140;
    if (meta.submitType) score += 90;
    if (meta.sameForm) score += 45;
    if (meta.nearComposer) score += 35;
    return score;
  }

  function findSubmitAction(composer) {
    if (!composer) return null;
    const composerRect = composer.getBoundingClientRect();
    const composerForm = form(composer);
    const scopes = [composerForm, composer.getRootNode(), composer.ownerDocument].filter(Boolean);
    const seen = new Set();
    const matches = [];
    for (const scope of scopes) {
      for (const node of scope.querySelectorAll('button, [role="button"]')) {
        if (seen.has(node)) continue;
        seen.add(node);
        const label = cleanText([
          node.getAttribute('aria-label'), node.getAttribute('title'), node.textContent
        ].filter(Boolean).join(' '));
        const rect = node.getBoundingClientRect();
        const meta = {
          visible: visible(node),
          disabled: node.matches(':disabled') || node.getAttribute('aria-disabled') === 'true',
          positiveLabel: /send|submit|ask|发送|提交|询问/i.test(label),
          negativeLabel: /microphone|voice|attach|upload|add|clear|close|history|new topic|麦克风|语音|附件|上传|添加|清除|关闭|历史|新话题/i.test(label),
          submitType: String(node.getAttribute('type') || '').toLowerCase() === 'submit',
          sameForm: !!composerForm && node.closest('form') === composerForm,
          nearComposer: Math.abs(rect.bottom - composerRect.bottom) <= 120 &&
            rect.left >= composerRect.left - 120 && rect.right <= composerRect.right + 120
        };
        const score = scoreSubmitAction(meta);
        if (score > 0) matches.push({ node, score, right: rect.right });
      }
      if (matches.length) break;
    }
    return matches.sort((left, right) => right.score - left.score || right.right - left.right)[0]?.node || null;
  }

  function pressEnter(composer) {
    if (!composer) return false;
    const KeyboardEventType = composer.ownerDocument.defaultView.KeyboardEvent;
    for (const type of ['keydown', 'keypress', 'keyup']) {
      composer.dispatchEvent(new KeyboardEventType(type, {
        key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true, cancelable: true
      }));
    }
    return true;
  }

  function diagnostics() {
    const docs = documents();
    const allRoots = docs.flatMap(roots);
    const values = candidates();
    return 'docs=' + docs.length + '|shadows=' + Math.max(0, allRoots.length - docs.length) +
      '|entries=' + values.length + '|visible=' + values.filter((value) => value.meta.visible).length;
  }

  return Object.freeze({
    version: 2,
    scoreMeta,
    scoreSubmitAction,
    find,
    value,
    setValue,
    form,
    findAction,
    findSubmitAction,
    pressEnter,
    diagnostics
  });
});
