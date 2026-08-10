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
    selectedIndex: Number.isInteger(options.selectedIndex) ? options.selectedIndex : -1,
    options: options.options || [],
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
  stateSettable: false,
  choiceLabels: [],
  selectedChoiceIndex: -1,
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
assert.equal(forms.describe(checkbox).stateSettable, true);
assert.equal(forms.planSelectedState(checkbox, true).needsActivation, false);
assert.equal(forms.planSelectedState(checkbox, false).needsActivation, true);

const radio = control({ type: 'radio', checked: true });
assert.equal(forms.planSelectedState(radio, false).reason, 'radio_cannot_clear');

const switchControl = control({
  tagName: 'DIV',
  attributes: { role: 'switch', 'aria-checked': 'false', 'aria-label': 'Memory' }
});
assert.equal(forms.describe(switchControl).role, 'switch');
assert.equal(forms.describe(switchControl).stateSettable, true);

const modelSelect = control({
  tagName: 'SELECT',
  selectedIndex: 0,
  options: [
    { label: 'Fast', textContent: 'Fast', disabled: false },
    { label: 'Thinking', textContent: 'Thinking', disabled: false }
  ],
  attributes: { 'aria-label': 'Model' }
});
assert.deepEqual(forms.describe(modelSelect).choiceLabels, ['Fast', 'Thinking']);
assert.equal(forms.describe(modelSelect).selectedChoiceIndex, 0);
assert.equal(forms.selectChoice(modelSelect, 1).ok, true);
assert.equal(modelSelect.selectedIndex, 1);
assert.deepEqual(modelSelect.events, ['input', 'change']);
assert.equal(forms.selectChoice(modelSelect, 3).reason, 'invalid_choice');

const richText = control({
  tagName: 'DIV',
  isContentEditable: true,
  attributes: { role: 'textbox', 'aria-label': 'Description' }
});
assert.equal(forms.setText(richText, 'Updated description').ok, true);
assert.equal(richText.textContent, 'Updated description');

assert.match(forms.ACTIONABLE_SELECTOR, /role="textbox"/);
assert.match(forms.ACTIONABLE_SELECTOR, /role="slider"/);
assert.match(forms.ACTIONABLE_SELECTOR, /role="switch"/);
process.stdout.write('CHATGPT_FORM_CONTROLS=passed\n');
