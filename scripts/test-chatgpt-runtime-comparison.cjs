'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { roleFiles, compareSymbol, fingerprint } = require('./analyze-chatgpt-runtime-contracts.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep12-full.cjs');

const imports = Object.values(profile.files).map(file => './' + file);
test('legacy three-role evidence remains compatible', () => {
  const { react, ...expected } = profile.files;
  assert.deepEqual(roleFiles(imports), expected);
});
test('full evidence includes the observed React import', () => {
  assert.deepEqual(roleFiles(imports, true), profile.files);
});
test('full evidence rejects missing and ambiguous React imports', () => {
  assert.throws(() => roleFiles(imports.filter(file => !file.startsWith('./2340486e-')), true), /ambiguous:react/);
  assert.throws(() => roleFiles([...imports, './2340486e-other.js'], true), /ambiguous:react/);
});
test('all currently consumed exports belong to the research baseline', async () => {
  const binding = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
  const requested = new Map();
  const modules = Object.fromEntries(Object.keys(profile.files).map(role => [role, new Proxy({}, {
    getOwnPropertyDescriptor(_, key) {
      const names = requested.get(role) || new Set(); names.add(key); requested.set(role, names);
      return { configurable: true, enumerable: true, value: () => {} };
    },
    get(_, key) { return key === 'then' ? undefined : () => {}; }
  })]));
  const base = 'https://chatgpt.com/cdn/assets/';
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'fixture', performance: { getEntriesByName: url =>
      url === base + profile.anchor ? [{}] : [] } };
  const runtime = binding.create(page, { loadRuntime: async url => {
    const role = Object.keys(profile.files).find(role => base + profile.files[role] === url);
    assert.ok(role); return modules[role];
  } });
  // The actual runtime's normalization reads every configured export. The test
  // tracks those accesses without importing any website source or credentials.
  for (const role of Object.keys(profile.files)) {
    await runtime.load(role);
    const expected = new Set(Object.values({ ...profile.expectedExports[role], ...profile.extraExports[role] }));
    assert.deepEqual(requested.get(role), expected, role);
  }
});
test('identical minified wrappers remain advisory ambiguous candidates', () => {
  const node = { type: 'Identifier', name: 'local' };
  const old = { exported: new Map([['old', 'getter']]), definitions: new Map([['getter', [node]]]) };
  const current = { index: new Map([[fingerprint(node), ['first', 'second']]]),
    exported: new Map([['a', 'first'], ['b', 'second']]) };
  assert.equal(compareSymbol(old, current, 'old').candidates.length, 2);
});
test('attachment acceptance admits transport before touching files or draft', () => {
  const source = fs.readFileSync(path.join(__dirname, 'smoke-chatgpt-fresh-attachment-dispatch.ps1'), 'utf8');
  const admission = source.indexOf('Stage transport_admission');
  assert.ok(admission > 0 && admission < source.indexOf('Stage stage_files'));
  assert.ok(source.indexOf("throw 'trial_not_armed'") < source.indexOf('Act set_input_text'));
  assert.ok(source.indexOf("throw 'trial_admission_changed'") < source.indexOf('Stage native_send'));
  assert.equal((source.match(/-Step send_fresh_attachment_fixture/g) || []).length, 1);
});
