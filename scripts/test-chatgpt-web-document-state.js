'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const evidence = require('../android/app/src/main/assets/chatgpt_web_private_protocol_evidence');

function node(overrides = {}) {
  const value = { tagName: 'DIV', isConnected: true, isContentEditable: true,
    getBoundingClientRect: () => ({ width: 320, height: 48 }),
    style: { display: 'block', visibility: 'visible' }, ...overrides };
  for (const key of ['textContent', 'innerText', 'innerHTML', 'value']) {
    Object.defineProperty(value, key, { get() { throw new Error('private field read'); } });
  }
  return value;
}
function fixture(prompts = [node()], overrides = {}) {
  const doc = { readyState: 'complete', visibilityState: 'visible', hasFocus: () => true,
    body: node({ tagName: 'BODY', isContentEditable: false }),
    documentElement: { getAttribute: () => null },
    querySelectorAll: selector => selector === 'main' || selector === 'form' ? [{}] : prompts,
    ...overrides };
  const page = { location: { origin: 'https://chatgpt.com' }, document: doc,
    innerWidth: 360, innerHeight: 780, getComputedStyle: node => node.style };
  for (const key of ['cookie', 'title', 'URL']) {
    Object.defineProperty(doc, key, { get() { throw new Error('private field read'); } });
  }
  for (const key of ['fetch', 'localStorage', 'sessionStorage']) {
    Object.defineProperty(page, key, { get() { throw new Error('transport or storage read'); } });
  }
  return { page, read: () => JSON.parse(evidence.documentState(page)) };
}

test('on-demand shapes neither read personal data nor activate a request', () => {
  const f = fixture();
  const value = f.read();
  assert.equal(value.schema, 'elon.document_state.v1');
  assert.equal(value.prompt_count, 1);
  assert.equal(value.visible_prompt_count, 1);
  assert.equal(value.editable_count, 1);
  assert.equal(value.incomplete, false);
  assert.equal(value.prompts[0].editable, true);
  assert.equal(value.body.tag, 'body');
});
test('absent, hidden and zero-sized composers remain distinct', () => {
  assert.equal(fixture([]).read().prompt_count, 0);
  for (const candidate of [node({ style: { display: 'none', visibility: 'visible' } }),
    node({ style: { display: 'block', visibility: 'hidden' } }),
    node({ getBoundingClientRect: () => ({ width: 0, height: 48 }) }),
    node({ isConnected: false })]) {
    const result = fixture([candidate]).read();
    assert.equal(result.prompt_count, 1);
    assert.equal(result.visible_prompt_count, 0);
  }
});
test('loading and hidden documents are not silently labelled ready', () => {
  const value = fixture([], { readyState: 'loading', visibilityState: 'hidden', hasFocus: () => false }).read();
  assert.equal(value.ready, 'loading');
  assert.equal(value.visibility, 'hidden');
  assert.equal(value.focused, false);
});
test('selectors outside the prompt scope are counted without exposing values', () => {
  const f = fixture([], { querySelectorAll: selector => selector === '[contenteditable="true"], textarea' ? [node()] : [] });
  assert.equal(f.read().prompt_count, 0);
  assert.equal(f.read().editable_count, 1);
});
test('output limits mark partial observation rather than claim feature absence', () => {
  const f = fixture(Array.from({ length: 300 }, () => node()));
  const value = f.read();
  assert.equal(value.prompt_count, 16);
  assert.equal(value.prompts.length, 8);
  assert.equal(value.editable_count, 256);
  assert.equal(value.incomplete, true);
  f.page.getComputedStyle = () => { throw new Error('unavailable'); };
  assert.equal(f.read().prompts[0].visibility, 'unknown');
  assert.equal(f.read().incomplete, true);
});
test('hostile dimensions, tags and styles cannot enter a receipt as arbitrary strings', () => {
  const f = fixture([node({ tagName: 'private text', style: { display: 'secret', visibility: 'secret' },
    getBoundingClientRect: () => ({ width: Infinity, height: 50000 }) })]);
  f.page.innerWidth = -50;
  f.page.innerHeight = NaN;
  const value = f.read();
  assert.equal(value.prompts[0].tag, 'unknown');
  assert.equal(value.prompts[0].display, 'unknown');
  assert.equal(value.prompts[0].width, 0);
  assert.equal(value.prompts[0].height, 20000);
  assert.equal(value.viewport_width, 0);
  assert.equal(value.viewport_height, 0);
  assert.doesNotMatch(JSON.stringify(value), /secret|private text/);
});
test('nonofficial origin and missing document are refused', () => {
  const f = fixture();
  f.page.location.origin = 'https://example.test';
  assert.equal(evidence.documentState(f.page), null);
  f.page.location.origin = 'https://chatgpt.com';
  f.page.document = null;
  assert.equal(evidence.documentState(f.page), null);
});
