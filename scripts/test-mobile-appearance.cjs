const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(__dirname, '../server/src/assets/mobile_startup.js'), 'utf8');
function boot(mode, blocked = false) {
  const values = new Map([['elon.mobile.appearance.v2', mode], ['account', 'fixture-account']]);
  const attributes = new Map();
  const listeners = [];
  const select = { value: '', addEventListener: (_, callback) => { select.change = callback; } };
  const document = {
    body: null,
    documentElement: {
      setAttribute: (key, value) => attributes.set(key, value),
      removeAttribute: key => attributes.delete(key),
    },
    addEventListener: (_, callback) => listeners.push(callback),
    getElementById: id => id === 'mobileAppearance' ? select : null,
  };
  const localStorage = {
    getItem: key => { if (blocked) throw new Error('unavailable'); return values.get(key); },
    setItem: (key, value) => { if (blocked) throw new Error('unavailable'); values.set(key, value); },
  };
  vm.runInNewContext(source, { document, localStorage, navigator: {}, setTimeout: () => 1, clearTimeout() {}, addEventListener() {} });
  listeners.forEach(callback => callback());
  return { values, attributes, select };
}

test('saved appearance is applied before DOM content and can return to the system theme', () => {
  const page = boot('dark');
  assert.equal(page.attributes.get('data-theme'), 'dark');
  assert.equal(page.select.value, 'dark');
  page.select.value = 'system'; page.select.change();
  assert.equal(page.attributes.has('data-theme'), false);
  assert.equal(page.values.get('elon.mobile.appearance.v2'), 'system');
  assert.equal(page.values.get('account'), 'fixture-account');
});

test('invalid preferences and unavailable storage do not break startup or theme selection', () => {
  for (const blocked of [false, true]) {
    const page = boot('unrecognized', blocked);
    assert.equal(page.select.value, 'system');
    page.select.value = 'light'; page.select.change();
    assert.equal(page.attributes.get('data-theme'), 'light');
    assert.equal(page.values.get('account'), 'fixture-account');
  }
});
