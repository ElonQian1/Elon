'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname,
  '..',
  'android',
  'app',
  'src',
  'main',
  'assets',
  'chatgpt_web_adapter_composer.js'
), 'utf8');

const roleButton = {
  id: '',
  textContent: 'GPT-5.6 Sol',
  getAttribute(name) {
    return ({ role: 'button', 'aria-haspopup': 'menu' })[name] || null;
  }
};
const scope = {
  querySelector: () => null,
  querySelectorAll(selector) {
    return selector === 'button, [role="button"]' ? [roleButton] : [];
  }
};
const composer = {
  closest(selector) {
    return selector === 'form' ? scope : null;
  }
};
const events = [];
const results = [];
const sandbox = {
  document: {
    querySelector: () => null,
    querySelectorAll: () => []
  },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    __elonChatGptActionTargetPolicy: {
      actionPoint(node) {
        return node === roleButton ? { x: 200, y: 700 } : null;
      },
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    }
  }
};
sandbox.window.window = sandbox.window;
sandbox.window.document = sandbox.document;
sandbox.window.location = sandbox.location;

vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_adapter_composer.js' });
sandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  composer,
  (event) => events.push(event),
  (...args) => results.push(args)
);

assert.equal(results.length, 0, 'menu collection remains pending until the official menu settles');
assert.equal(events.length, 1);
assert.equal(events[0].type, 'web_touch_request');
assert.equal(events[0].purpose, 'list_model_options');
assert.equal(events[0].xRatio, 0.5);
assert.equal(events[0].yRatio, 0.875);

process.stdout.write('CHATGPT_COMPOSER_TRIGGER_POLICY=passed\n');
