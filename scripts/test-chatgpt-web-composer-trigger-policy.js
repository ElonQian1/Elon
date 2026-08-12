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

const renamedModelButton = {
  id: '',
  textContent: '5.6 Terra 中',
  getAttribute(name) {
    return name === 'role' ? 'button' : null;
  }
};
const unrelatedComposerButton = {
  id: '',
  textContent: 'Workspace',
  getAttribute(name) {
    return name === 'role' ? 'button' : null;
  }
};
const renamedEvents = [];
const renamedResults = [];
const renamedScope = {
  querySelector: () => null,
  querySelectorAll(selector) {
    return selector === 'button, [role="button"]'
      ? [unrelatedComposerButton, renamedModelButton]
      : [];
  }
};
const renamedComposer = {
  closest(selector) {
    return selector === 'form' ? renamedScope : null;
  }
};
const renamedSandbox = {
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
        if (node === renamedModelButton) return { x: 210, y: 700 };
        if (node === unrelatedComposerButton) return { x: 80, y: 700 };
        return null;
      },
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    }
  }
};
renamedSandbox.window.window = renamedSandbox.window;
renamedSandbox.window.document = renamedSandbox.document;
renamedSandbox.window.location = renamedSandbox.location;

vm.runInNewContext(source, renamedSandbox, { filename: 'chatgpt_web_adapter_composer.js' });
renamedSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  renamedComposer,
  (event) => renamedEvents.push(event),
  (...args) => renamedResults.push(args)
);

assert.equal(renamedResults.length, 0, 'a uniquely isolated renamed model control remains usable');
assert.equal(renamedEvents.length, 1);
assert.equal(renamedEvents[0].purpose, 'list_model_options');
assert.equal(renamedEvents[0].xRatio, 0.525);

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
      }
    }
  }
};
semanticButton.getBoundingClientRect = () => ({
  left: 160,
  top: 640,
  right: 240,
  bottom: 680,
  width: 80,
  height: 40
});
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
assert.equal(semanticEvents[0].xRatio, 0.5);
assert.equal(semanticEvents[0].yRatio, 0.825);

const dictationEvents = [];
const dictationResults = [];
const dictationButton = {
  id: '',
  textContent: '',
  getAttribute(name) {
    return name === 'aria-label' ? '开始听写' : null;
  },
  getBoundingClientRect() {
    return { left: 300, top: 640, right: 360, bottom: 700, width: 60, height: 60 };
  }
};
const dictationScope = {
  querySelector: () => null,
  querySelectorAll: () => []
};
const dictationComposer = {
  closest(selector) {
    return selector === 'form' ? dictationScope : null;
  }
};
const dictationSandbox = {
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
      actionPoint(node) {
        return node === dictationButton ? { x: 330, y: 670 } : null;
      },
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    },
    __elonChatGptLayout: {
      findSemanticNode(semantic, region) {
        return semantic === 'dictation' && region === 'composer' ? dictationButton : null;
      }
    }
  }
};
dictationSandbox.window.window = dictationSandbox.window;
dictationSandbox.window.document = dictationSandbox.document;
dictationSandbox.window.location = dictationSandbox.location;

vm.runInNewContext(source, dictationSandbox, { filename: 'chatgpt_web_adapter_composer.js' });
assert.ok(
  Array.from(dictationSandbox.window.__elonChatGptComposer.capabilities(dictationComposer))
    .includes('dictation'),
  'semantic manifest dictation controls must be advertised as a composer capability'
);
dictationSandbox.window.__elonChatGptComposer.startDictation(
  dictationComposer,
  (event) => dictationEvents.push(event),
  (...args) => dictationResults.push(args)
);
assert.deepEqual(Array.from(dictationResults[0]), ['start_dictation', true, '']);
assert.equal(dictationEvents[0].type, 'web_touch_request');
assert.equal(dictationEvents[0].purpose, 'start_dictation');
assert.equal(dictationEvents[0].xRatio, 0.825);
assert.equal(dictationEvents[0].yRatio, 0.8375);

const optionEvents = [];
const optionResults = [];
const optionNode = {
  id: '',
  textContent: '思考强度 极高',
  getAttribute(name) {
    return name === 'role' ? 'menuitem' : null;
  },
  hasAttribute: () => false,
  getBoundingClientRect: () => ({
    left: 40,
    top: 400,
    right: 360,
    bottom: 460,
    width: 320,
    height: 60
  })
};
const optionSandbox = {
  document: {
    querySelector: () => null,
    querySelectorAll(selector) {
      return selector.includes('[role="menuitem"]') ? [optionNode] : [];
    }
  },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    setTimeout: (callback) => callback(),
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    __elonChatGptActionTargetPolicy: {
      actionPoint: () => null,
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    }
  }
};
optionSandbox.window.window = optionSandbox.window;
optionSandbox.window.document = optionSandbox.document;
optionSandbox.window.location = optionSandbox.location;

vm.runInNewContext(source, optionSandbox, { filename: 'chatgpt_web_adapter_composer.js' });
optionSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  composer,
  (event) => optionEvents.push(event),
  (...args) => optionResults.push(args)
);

assert.equal(optionResults.length, 1);
assert.deepEqual(Array.from(optionResults[0]), ['list_model_options', true, '']);
assert.equal(optionEvents.length, 1);
assert.equal(optionEvents[0].type, 'composer_controls_snapshot');
assert.equal(optionEvents[0].options.length, 1);

optionSandbox.window.__elonChatGptComposer.selectOption(
  'model',
  optionEvents[0].options[0].id,
  composer,
  (event) => optionEvents.push(event),
  (...args) => optionResults.push(args),
  () => {}
);

assert.equal(optionEvents.length, 2);
assert.equal(optionEvents[1].type, 'web_touch_request');
assert.equal(optionEvents[1].purpose, 'select_model_option');

const toolsEvents = [];
const toolsResults = [];
const toolsButton = {
  id: '',
  textContent: '',
  getAttribute: () => null,
  getBoundingClientRect: () => ({
    left: 20,
    top: 640,
    right: 60,
    bottom: 680,
    width: 40,
    height: 40
  })
};
const toolsSandbox = {
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
        assert.equal(semantic, 'attachment');
        assert.equal(region, 'composer');
        return toolsButton;
      }
    }
  }
};
toolsSandbox.window.window = toolsSandbox.window;
toolsSandbox.window.document = toolsSandbox.document;
toolsSandbox.window.location = toolsSandbox.location;

vm.runInNewContext(source, toolsSandbox, { filename: 'chatgpt_web_adapter_composer.js' });
toolsSandbox.window.__elonChatGptComposer.requestOptions(
  'tools',
  composer,
  (event) => toolsEvents.push(event),
  (...args) => toolsResults.push(args)
);

assert.equal(toolsResults.length, 0);
assert.equal(toolsEvents.length, 1);
assert.equal(toolsEvents[0].type, 'web_touch_request');
assert.equal(toolsEvents[0].purpose, 'list_composer_tools');
assert.equal(toolsEvents[0].xRatio, 0.1);
assert.equal(toolsEvents[0].yRatio, 0.825);

process.stdout.write('CHATGPT_COMPOSER_TRIGGER_POLICY=passed\n');
