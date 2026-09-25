'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const test = require('node:test');

const root = path.resolve(__dirname, '..');
const bootstrap = fs.readFileSync(path.join(root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs'), 'utf8');
const manifest = bootstrap.split('const ADAPTER_ASSETS:')[1].split('\n];')[0];
const names = [...manifest.matchAll(/^\s*"(chatgpt_[a-z0-9_]+\.js)",/gm)].map(match => match[1]);
const composer = 'chatgpt_web_adapter_composer.js';
const required = [
  'chatgpt_web_adapter_composer_dismiss_policy.js',
  'chatgpt_web_dictation_runtime.js',
  'chatgpt_web_adapter_dictation_actions.js',
];

function loadPrefix(omit) {
  const page = { location: { origin: 'https://chatgpt.com' } };
  const context = vm.createContext({ window: page, location: page.location });
  function load() {
    for (const name of names.slice(0, names.indexOf(composer) + 1)) {
      if (name === omit) continue;
      const source = fs.readFileSync(path.join(root, 'android/app/src/main/assets', name), 'utf8');
      vm.runInContext(source, context, { filename: name, timeout: 1000 });
    }
  }
  load();
  return { page, load };
}

test('Win ships required composer dependencies before their consumer', () => {
  assert.ok(names.indexOf(composer) > 0);
  for (const name of required) {
    assert.ok(names.includes(name), 'missing Win dependency: ' + name);
    assert.ok(names.indexOf(name) < names.indexOf(composer), 'late Win dependency: ' + name);
  }
  assert.ok(names.indexOf(required[1]) < names.indexOf(required[2]));
});

test('real Win manifest initializes composer without test-injected dependencies', () => {
  const { page } = loadPrefix();
  assert.equal(typeof page.__elonChatGptDictationActions.create, 'function');
  assert.equal(typeof page.__elonChatGptComposer.capabilities, 'function');
  assert.equal(typeof page.__elonChatGptComposer.requestAttachmentUpload, 'function');
  assert.equal(typeof page.__elonChatGptComposer.startDictation, 'function');
  assert.equal(typeof page.__elonChatGptComposer.dismissOpenMenu, 'function');
  assert.equal(typeof page.__elonChatGptComposerDismissPolicy.dismissMenu, 'function');
  assert.equal(typeof page.__elonChatGptDictationRuntime.createCaptureTracker, 'function');
});

test('Win composer survives reinjection without replacing the active instance', () => {
  const { page, load } = loadPrefix();
  const before = page.__elonChatGptComposer;
  load();
  assert.equal(page.__elonChatGptComposer, before);
});

test('missing dictation actions reproduces the observed production bootstrap failure', () => {
  assert.throws(() => loadPrefix('chatgpt_web_adapter_dictation_actions.js'),
    error => error.name === 'TypeError' && error.stack.includes('chatgpt_web_adapter_composer.js:24'));
});
