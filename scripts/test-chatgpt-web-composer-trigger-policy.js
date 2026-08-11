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

const semanticEvents = [];
const semanticResults = [];
const semanticButton = {
  id: '',
  textContent: 'Sol',
  getAttribute: () => null
};
const semanticSandbox = {
  document: {
    querySelector: () => null,
    querySelectorAll: () => []
  },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    __elonChatGptActionTargetPolicy: {
      actionPoint: () => null,
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    },
    __elonChatGptLayout: {
      findSemanticNode(semantic, region) {
        assert.equal(semantic, 'model');
        assert.equal(region, 'composer');
        return semanticButton;
      },
      requestSemanticTouch(semantic, purpose, emitEvent, region) {
        assert.equal(semantic, 'model');
        assert.equal(region, 'composer');
        emitEvent({ type: 'web_touch_request', purpose, xRatio: 0.4, yRatio: 0.8 });
        return true;
      }
    }
  }
};
semanticButton.getBoundingClientRect = () => ({ width: 80, height: 40 });
semanticSandbox.window.window = semanticSandbox.window;
semanticSandbox.window.document = semanticSandbox.document;
semanticSandbox.window.location = semanticSandbox.location;

vm.runInNewContext(source, semanticSandbox, { filename: 'chatgpt_web_adapter_composer.js' });
semanticSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  composer,
  (event) => semanticEvents.push(event),
  (...args) => semanticResults.push(args)
);

assert.equal(semanticResults.length, 0);
assert.equal(semanticEvents.length, 1);
assert.equal(semanticEvents[0].type, 'web_touch_request');
assert.equal(semanticEvents[0].purpose, 'list_model_options');

process.stdout.write('CHATGPT_COMPOSER_TRIGGER_POLICY=passed\n');
