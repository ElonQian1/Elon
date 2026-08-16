'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const source = fs.readFileSync(
  path.resolve(
    __dirname,
    '../android/app/src/main/assets/chatgpt_web_adapter_composer.js'
  ),
  'utf8'
);
const submenuSource = fs.readFileSync(
  path.resolve(
    __dirname,
    '../android/app/src/main/assets/chatgpt_web_adapter_composer_submenu.js'
  ),
  'utf8'
);

let stage = 0;
const rect = {
  left: 24,
  top: 180,
  right: 360,
  bottom: 236,
  width: 336,
  height: 56
};

function hiddenRect() {
  return { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
}

function option(label, attributes, visibleWhen) {
  return {
    id: '',
    isConnected: true,
    textContent: label,
    getAttribute(name) {
      if (name === 'role') return 'menuitem';
      return Object.prototype.hasOwnProperty.call(attributes, name)
        ? attributes[name]
        : null;
    },
    hasAttribute(name) {
      return Object.prototype.hasOwnProperty.call(attributes, name);
    },
    getBoundingClientRect: () => visibleWhen() ? rect : hiddenRect()
  };
}

const submenu = option('更多模型', {
  'aria-haspopup': 'menu',
  'aria-expanded': 'false'
}, () => stage !== 2);
const leaf = option(
  '测试模型',
  { 'aria-checked': 'false' },
  () => stage === 1 || stage === 5
);
const modelTrigger = {
  id: 'model-trigger',
  isConnected: true,
  textContent: '模型',
  getAttribute: () => null,
  hasAttribute: () => false,
  getBoundingClientRect: () => rect
};
const events = [];
const results = [];
const document = {
  querySelector: () => null,
  querySelectorAll(selector) {
    if (!selector.includes('[role="menuitem"]')) return [];
    if (stage === 0 || stage === 3) return [submenu];
    if (stage === 1 || stage === 5) return [submenu, leaf];
    if (stage === 4) {
      stage = 5;
      return [submenu];
    }
    return [];
  }
};
const composer = {
  closest: () => document,
  getAttribute: () => null,
  textContent: ''
};
const sandbox = {
  Date,
  document,
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    setTimeout: (callback) => callback(),
    __elonChatGptActionTargetPolicy: {
      actionPoint(node) {
        return node.getBoundingClientRect().width > 0 ? { x: 192, y: 208 } : null;
      },
      signature(node) {
        return node === submenu ? `submenu-${stage}` : 'leaf';
      }
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    },
    __elonChatGptComposerToolStatePolicy: {
      directSelection(input) {
        return {
          known: input.ariaChecked === 'true' || input.ariaChecked === 'false',
          selected: input.ariaChecked === 'true'
        };
      },
      optionSelected(input) {
        return input.directSelected;
      }
    },
    __elonChatGptLayout: {
      findSemanticNode(semantic, region) {
        return semantic === 'model' && region === 'composer' ? modelTrigger : null;
      },
      setNodeExpanded() {
        throw new Error('submenu navigation must not depend on aria-expanded');
      }
    }
  }
};
sandbox.window.window = sandbox.window;
sandbox.window.document = document;
sandbox.window.location = sandbox.location;

vm.runInNewContext(submenuSource, sandbox, {
  filename: 'chatgpt_web_adapter_composer_submenu.js'
});
vm.runInNewContext(source, sandbox, {
  filename: 'chatgpt_web_adapter_composer.js'
});
const adapter = sandbox.window.__elonChatGptComposer;
adapter.requestOptions(
  'model',
  composer,
  (event) => events.push(event),
  (...args) => results.push(args)
);

assert.equal(results.length, 1);
assert.deepEqual(Array.from(results[0]), ['list_model_options', true, '']);
assert.equal(events[0].options[0].opensSubmenu, true);

adapter.selectOption(
  'model',
  events[0].options[0].id,
  composer,
  (event) => {
    events.push(event);
    if (event.type === 'web_touch_request' && event.purpose === 'open_model_submenu') {
      stage = 1;
    }
  },
  (...args) => results.push(args),
  () => {}
);

assert.equal(events[1].type, 'web_touch_request');
assert.equal(events[1].purpose, 'open_model_submenu');
assert.equal(results.length, 2);
assert.deepEqual(Array.from(results[1]), ['select_model_option', true, '']);
assert.equal(events[2].type, 'composer_controls_snapshot');
assert.equal(events[2].options.length, 1);
assert.equal(events[2].options[0].opensSubmenu, false);

const leafId = events[2].options[0].id;
stage = 2;
adapter.selectOption(
  'model',
  leafId,
  composer,
  (event) => {
    events.push(event);
    if (stage === 2 && event.purpose === 'select_model_option') stage = 3;
    else if (stage === 3 && event.purpose === 'open_model_submenu') stage = 4;
    else if (stage === 5 && event.purpose === 'select_model_option') stage = 2;
  },
  (...args) => results.push(args),
  () => {}
);

assert.equal(events[3].purpose, 'select_model_option');
assert.equal(events[4].purpose, 'open_model_submenu');
assert.equal(events[5].type, 'composer_controls_snapshot');
assert.equal(events[6].purpose, 'select_model_option');
assert.equal(results.length, 3);
assert.deepEqual(Array.from(results[2]), ['select_model_option', true, '']);

console.log('CHATGPT_COMPOSER_SUBMENU_SELECTION_POLICY=passed');
