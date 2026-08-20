'use strict';

const assert = require('node:assert/strict');
const forms = require('../android/app/src/main/assets/chatgpt_web_adapter_form_controls.js');

function control(options = {}) {
  const attributes = Object.assign({}, options.attributes);
  const events = [];
  return {
    tagName: options.tagName || 'INPUT',
    type: options.type || '',
    min: options.min || '',
    max: options.max || '',
    step: options.step || '',
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
  sliderSettable: false,
  sliderMin: null,
  sliderMax: null,
  sliderStep: null,
  sliderValue: null,
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
assert.equal(forms.semantic(forms.describe(switchControl)), 'toggle');

const selectedTab = control({
  tagName: 'BUTTON',
  attributes: { role: 'tab', 'aria-selected': 'true', 'aria-label': 'General' }
});
const tabDetails = forms.describe(selectedTab);
assert.equal(tabDetails.role, 'tab');
assert.equal(tabDetails.inputKind, 'tab');
assert.equal(tabDetails.selected, true);
assert.equal(tabDetails.stateSettable, true);
assert.equal(forms.semantic(tabDetails), 'selection');
assert.equal(forms.planSelectedState(selectedTab, true).needsActivation, false);
assert.equal(forms.planSelectedState(selectedTab, false).reason, 'tab_cannot_clear');

const menuRadio = control({
  tagName: 'DIV',
  attributes: { role: 'menuitemradio', 'aria-checked': 'true', 'aria-label': 'Fast' }
});
assert.equal(forms.describe(menuRadio).role, 'menuitemradio');
assert.equal(forms.describe(menuRadio).inputKind, 'radio');
assert.equal(forms.describe(menuRadio).stateSettable, true);
assert.equal(forms.planSelectedState(menuRadio, false).reason, 'radio_cannot_clear');

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
assert.equal(forms.semantic(forms.describe(modelSelect)), 'selection');
assert.equal(forms.describe(modelSelect).selectedChoiceIndex, 0);
assert.equal(forms.selectChoice(modelSelect, 1).ok, true);
assert.equal(modelSelect.selectedIndex, 1);
assert.deepEqual(modelSelect.events, ['input', 'change']);
assert.equal(forms.selectChoice(modelSelect, 3).reason, 'invalid_choice');

const range = control({
  type: 'range',
  min: '0',
  max: '2',
  step: '0.5',
  value: '1',
  attributes: { 'aria-label': 'Thinking effort' }
});
const rangeDetails = forms.describe(range);
assert.equal(rangeDetails.role, 'slider');
assert.equal(rangeDetails.sliderSettable, true);
assert.equal(rangeDetails.sliderMin, 0);
assert.equal(rangeDetails.sliderMax, 2);
assert.equal(rangeDetails.sliderStep, 0.5);
assert.equal(rangeDetails.sliderValue, 1);
assert.equal(forms.semantic(rangeDetails), 'slider');
assert.equal(forms.setSliderValue(range, 1.6).ok, true);
assert.equal(range.value, '1.5');
assert.deepEqual(range.events, ['input', 'change']);
assert.equal(forms.setSliderValue(range, Number.NaN).reason, 'invalid_value');

const ariaRange = control({
  tagName: 'DIV',
  attributes: {
    role: 'slider',
    'aria-label': 'Thinking effort',
    'aria-valuemin': '0',
    'aria-valuemax': '3',
    'aria-valuenow': '2'
  }
});
const ariaRangeDetails = forms.describe(ariaRange);
assert.equal(ariaRangeDetails.sliderSettable, true);
assert.equal(ariaRangeDetails.sliderMin, 0);
assert.equal(ariaRangeDetails.sliderMax, 3);
assert.equal(ariaRangeDetails.sliderStep, 1);
assert.equal(ariaRangeDetails.sliderValue, 2);
assert.equal(forms.planSliderValue(ariaRange, 3).pointer, true);
assert.equal(forms.setSliderValue(ariaRange, 3).reason, 'requires_pointer');

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
assert.match(forms.ACTIONABLE_SELECTOR, /role="tab"/);
assert.match(forms.ACTIONABLE_SELECTOR, /role="menuitemradio"/);
process.stdout.write('CHATGPT_FORM_CONTROLS=passed\n');
