'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/chatgpt_web_adapter_skin.js'),
  'utf8'
);

function createDocument() {
  const nodes = new Map();
  const attributes = new Map();
  const head = {
    appendChild(node) {
      node.parentNode = head;
      nodes.set(node.id, node);
    },
    removeChild(node) {
      nodes.delete(node.id);
      node.parentNode = null;
    }
  };
  return {
    head,
    documentElement: {
      setAttribute(name, value) { attributes.set(name, value); },
      removeAttribute(name) { attributes.delete(name); },
      getAttribute(name) { return attributes.get(name) || null; }
    },
    createElement(tagName) { return { tagName, id: '', textContent: '', parentNode: null }; },
    getElementById(id) { return nodes.get(id) || null; }
  };
}

const document = createDocument();
const window = { document, location: { origin: 'https://chatgpt.com' } };
vm.runInNewContext(source, { window });

const skin = window.__elonChatGptSkin;
assert.ok(skin, 'skin module must install');
assert.deepStrictEqual(
  JSON.parse(JSON.stringify(skin.setEnabled(true))),
  { ok: true, enabled: true }
);
assert.strictEqual(document.documentElement.getAttribute(skin.rootAttribute), 'true');
const style = document.getElementById(skin.styleId);
assert.ok(style, 'skin mode must add one versioned style node');
assert.match(style.textContent, /data-testid="sidebar"/);
assert.match(style.textContent, /main \{ width: 100%/);

skin.setEnabled(true);
assert.strictEqual(document.getElementById(skin.styleId), style, 'enable must be idempotent');
assert.deepStrictEqual(
  JSON.parse(JSON.stringify(skin.setEnabled(false))),
  { ok: true, enabled: false }
);
assert.strictEqual(document.documentElement.getAttribute(skin.rootAttribute), null);
assert.strictEqual(document.getElementById(skin.styleId), null);

const rejected = skin.setEnabled(true, { document, origin: 'https://example.com' });
assert.strictEqual(rejected.ok, false);
assert.strictEqual(rejected.reason, 'unsupported_origin');

['document.cookie', 'fetch(', 'XMLHttpRequest', 'WebSocket', 'Authorization'].forEach((forbidden) => {
  assert.ok(!source.includes(forbidden), `skin policy must not contain ${forbidden}`);
});

console.log('ChatGPT single-WebView skin policy passed.');
