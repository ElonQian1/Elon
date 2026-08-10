'use strict';

const assert = require('node:assert/strict');
const forms = require('../android/app/src/main/assets/chatgpt_web_adapter_form_controls.js');

function control(options = {}) {
  const attributes = Object.assign({}, options.attributes);
  const events = [];
  return {
    tagName: options.tagName || 'INPUT',
    type: options.type || '',
    disabled: !!options.disabled,
    readOnly: !!options.readOnly,
    checked: !!options.checked,
    isContentEditable: !!options.isContentEditable,
    labels: options.labels || [],
    value: options.value || '',
    textContent: options.textContent || '',
    focused: false,
    events,
    getAttribute(name) { return attributes[name] || ''; },
    focus() { this.focused = true; },
    dispatchEvent(event) { events.push(event.type); return true; }
  };
}

const search = control({
  type: 'search',
  attributes: { placeholder: 'Search chats', name: 'query' }
});
assert.deepEqual(forms.describe(search), {
  role: 'textbox',
  inputKind: 'search',
  writable: true,
  sensitive: false,
  selected: false,
  label: 'Search chats'
});
assert.equal(forms.setText(search, 'release notes').ok, true);
assert.equal(search.value, 'release notes');
assert.equal(search.focused, true);
assert.deepEqual(search.events, ['input', 'change']);

const password = control({ type: 'password', attributes: { 'aria-label': 'Password' } });
assert.equal(forms.describe(password).sensitive, true);
assert.equal(forms.describe(password).writable, false);
assert.equal(forms.setText(password, 'secret').reason, 'sensitive');
assert.equal(password.value, '');

const readonly = control({ type: 'text', readOnly: true });
assert.equal(forms.describe(readonly).writable, false);
assert.equal(forms.setText(readonly, 'blocked').reason, 'not_writable');

const checkbox = control({ type: 'checkbox', checked: true, attributes: { 'aria-label': 'Enabled' } });
assert.equal(forms.describe(checkbox).role, 'checkbox');
assert.equal(forms.describe(checkbox).selected, true);
assert.equal(forms.describe(checkbox).writable, false);

const richText = control({
  tagName: 'DIV',
  isContentEditable: true,
  attributes: { role: 'textbox', 'aria-label': 'Description' }
});
assert.equal(forms.setText(richText, 'Updated description').ok, true);
assert.equal(richText.textContent, 'Updated description');

assert.match(forms.ACTIONABLE_SELECTOR, /role="textbox"/);
assert.match(forms.ACTIONABLE_SELECTOR, /role="slider"/);
process.stdout.write('CHATGPT_FORM_CONTROLS=passed\n');
