'use strict';
const fs = require('node:fs'), vm = require('node:vm');
const assert = require('node:assert/strict'), test = require('node:test');
const { fixture } = require('./fixtures/chatgpt-model-state');
const source = fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_adapter_composer.js'), 'utf8');
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

for (const kind of ['scoped selector', 'semantic composer', 'scoped label']) {
  test(kind + ' precedes the page header and admits the private picker', async () => {
    const f = fixture(), page = f.page, events = [], results = [];
    const header = { ...f.node, textContent: 'Header model', __reactFiber$fixture: null };
    f.node.textContent = 'High';
    const scope = {
      querySelector: selector => kind === 'scoped selector' && selector === '[aria-label*="选择模型"]' ? f.node : null,
      querySelectorAll: () => kind === 'scoped label' ? [f.node] : []
    };
    page.document.querySelector = selector => selector === '[data-testid="model-switcher"]' ? header : null;
    page.document.querySelectorAll = () => [];
    page.getComputedStyle = () => ({ display: 'block', visibility: 'visible' });
    page.__elonChatGptActionTargetPolicy = {
      actionPoint: node => node === f.node ? { x: 40, y: 40 } : null
    };
    page.__elonChatGptModelLabelPolicy = { isModelLabel: () => true };
    page.__elonChatGptLayout = { findSemanticNode: () => kind === 'semantic composer' ? f.node : null };
    page.__elonChatGptPrivateModelState = { create: () => f.runtime };
    page.__elonChatGptDictationActions = { create: () => ({}) };
    page.__elonChatGptComposerSubmenu = { createRecovery: () => ({}) };
    vm.runInNewContext(source, { window: page, document: page.document, location: { origin: 'https://chatgpt.com' } });
    const input = { closest: () => scope }, composer = page.__elonChatGptComposer;
    assert.equal(composer.modelTrigger(input), f.node);
    composer.requestOptions('model', input, e => events.push(e), (...r) => results.push(r)); await flush();
    assert.ok(events.at(-1).options.every(option => option.id.startsWith('private_model_')));
    assert.equal(results.at(-1)[1], true);
    assert.equal(events.some(e => e.type === 'web_touch_request'), false);
  });
}
