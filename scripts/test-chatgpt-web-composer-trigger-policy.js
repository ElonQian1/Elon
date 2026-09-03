'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const dictationActionsModule = require('../android/app/src/main/assets/chatgpt_web_adapter_dictation_actions.js');
const runDictationTriggerTest = require('./test-chatgpt-web-composer-dictation-trigger.js');

async function main() {

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
const dictationSessionPolicy = require('../android/app/src/main/assets/chatgpt_web_adapter_dictation_session_policy.js');
const composerSubmenu = require('../android/app/src/main/assets/chatgpt_web_adapter_composer_submenu.js');
const modelLabelPolicy = require('../android/app/src/main/assets/chatgpt_web_adapter_model_label_policy.js');

function runComposer(sandbox) {
  sandbox.window.__elonChatGptComposerSubmenu = composerSubmenu;
  sandbox.window.__elonChatGptDictationActions =
    dictationActionsModule.createForRoot(sandbox.window);
  vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_adapter_composer.js' });
}

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
    },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
sandbox.window.window = sandbox.window;
sandbox.window.document = sandbox.document;
sandbox.window.location = sandbox.location;

runComposer(sandbox);
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

const compactLevelButton = {
  id: '',
  textContent: '高',
  getAttribute: () => null
};
const compactLevelComposer = {
  closest(selector) {
    if (selector !== 'form') return null;
    return {
      querySelector: () => null,
      querySelectorAll: (candidate) => candidate === 'button, [role="button"]'
        ? [compactLevelButton]
        : []
    };
  }
};
const compactLevelEvents = [];
const compactLevelResults = [];
const compactLevelSandbox = {
  document: { querySelector: () => null, querySelectorAll: () => [] },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    __elonChatGptActionTargetPolicy: {
      actionPoint: (node) => node === compactLevelButton ? { x: 300, y: 700 } : null,
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: { filter: (_section, options) => options },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy,
    __elonChatGptModelLabelPolicy: modelLabelPolicy
  }
};
compactLevelSandbox.window.window = compactLevelSandbox.window;
compactLevelSandbox.window.document = compactLevelSandbox.document;
compactLevelSandbox.window.location = compactLevelSandbox.location;

runComposer(compactLevelSandbox);
compactLevelSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  compactLevelComposer,
  (event) => compactLevelEvents.push(event),
  (...args) => compactLevelResults.push(args)
);

assert.equal(compactLevelResults.length, 0, 'compact level opens the official model control');
assert.equal(compactLevelEvents.length, 1);
assert.equal(compactLevelEvents[0].purpose, 'list_model_options');

const mobileModelButton = {
  id: '',
  textContent: 'Auto',
  getBoundingClientRect() {
    return { left: 60, top: 640, right: 160, bottom: 680, width: 100, height: 40 };
  },
  getAttribute(name) {
    return ({
      'data-testid': 'model-switcher',
      'aria-label': 'Model selector, GPT-5.6 Instant'
    })[name] || null;
  }
};
const mobileScope = {
  querySelector(selector) {
    return selector.includes('model-switcher') ? mobileModelButton : null;
  },
  querySelectorAll: () => []
};
const mobileComposer = {
  closest(selector) {
    return selector === 'form' ? mobileScope : null;
  }
};
const mobileSandbox = {
  document: { querySelector: () => null, querySelectorAll: () => [] },
  location: { origin: 'https://chatgpt.com' },
  window: {
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    __elonChatGptModelLabelPolicy: {
      isModelLabel: (label) => /gpt|auto/i.test(label)
    },
    __elonChatGptComposerOptionPolicy: { filter: (_section, options) => options },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
mobileSandbox.window.window = mobileSandbox.window;
mobileSandbox.window.document = mobileSandbox.document;
mobileSandbox.window.location = mobileSandbox.location;
runComposer(mobileSandbox);
assert.equal(mobileSandbox.window.__elonChatGptComposer.currentModel(mobileComposer), 'Auto');

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
    },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
renamedSandbox.window.window = renamedSandbox.window;
renamedSandbox.window.document = renamedSandbox.document;
renamedSandbox.window.location = renamedSandbox.location;

runComposer(renamedSandbox);
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
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy,
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

runComposer(semanticSandbox);
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

const backgroundModelEvents = [];
const backgroundModelResults = [];
const backgroundModelButton = {
  id: '',
  textContent: '极高',
  getAttribute(name) {
    return name === 'data-testid' ? 'model-switcher' : null;
  }
};
const backgroundModelScope = {
  querySelector(selector) {
    return selector.includes('model-switcher') ? backgroundModelButton : null;
  },
  querySelectorAll: () => []
};
const backgroundModelComposer = {
  closest(selector) {
    return selector === 'form' ? backgroundModelScope : null;
  }
};
const backgroundModelSandbox = {
  document: { querySelector: () => null, querySelectorAll: () => [] },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    getComputedStyle: () => ({ display: 'block', visibility: 'hidden' }),
    __elonChatGptActionTargetPolicy: {
      actionPoint(node) {
        return node === backgroundModelButton ? { x: 200, y: 660 } : null;
      },
      signature: () => 'background-actionable'
    },
    __elonChatGptComposerOptionPolicy: { filter: (_section, options) => options },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
backgroundModelSandbox.window.window = backgroundModelSandbox.window;
backgroundModelSandbox.window.document = backgroundModelSandbox.document;
backgroundModelSandbox.window.location = backgroundModelSandbox.location;

runComposer(backgroundModelSandbox);
backgroundModelSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  backgroundModelComposer,
  (event) => backgroundModelEvents.push(event),
  (...args) => backgroundModelResults.push(args)
);

assert.equal(backgroundModelResults.length, 0, 'background model control remains actionable');
assert.equal(backgroundModelEvents.length, 1);
assert.equal(backgroundModelEvents[0].purpose, 'list_model_options');
assert.equal(backgroundModelEvents[0].xRatio, 0.5);
assert.equal(backgroundModelEvents[0].yRatio, 0.825);

const automaticModelEvents = [];
const automaticModelResults = [];
const automaticModelSandbox = {
  document: { querySelector: () => null, querySelectorAll: () => [] },
  location: { origin: 'https://chatgpt.com' },
  window: {
    __elonChatGptActionTargetPolicy: { actionPoint: () => null, signature: () => '' },
    __elonChatGptComposerOptionPolicy: { filter: (_section, options) => options },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
automaticModelSandbox.window.window = automaticModelSandbox.window;
automaticModelSandbox.window.document = automaticModelSandbox.document;
automaticModelSandbox.window.location = automaticModelSandbox.location;

runComposer(automaticModelSandbox);
automaticModelSandbox.window.__elonChatGptComposer.requestOptions(
  'model',
  { closest: () => null },
  (event) => automaticModelEvents.push(event),
  (...args) => automaticModelResults.push(args)
);

assert.equal(automaticModelEvents.length, 1);
assert.equal(automaticModelEvents[0].type, 'composer_controls_snapshot');
assert.equal(automaticModelEvents[0].options.length, 0);
assert.deepEqual(
  Array.from(automaticModelResults[0]),
  ['list_model_options', true, '官网当前由系统自动选择模型。']
);

await runDictationTriggerTest();

const optionEvents = [];
const optionResults = [];
const optionTrigger = {
  id: 'model-trigger',
  textContent: '极高',
  getAttribute(name) {
    return name === 'aria-expanded' ? 'true' : null;
  },
  getBoundingClientRect: () => ({
    left: 120,
    top: 680,
    right: 220,
    bottom: 730,
    width: 100,
    height: 50
  })
};
const optionNode = {
  id: '',
  textContent: '思考强度 极高',
  getAttribute(name) {
    return name === 'role' ? 'menuitemradio' : null;
  },
  querySelector: () => ({}),
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
    },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy,
    __elonChatGptLayout: {
      findSemanticNode(semantic, region) {
        return semantic === 'model' && region === 'composer' ? optionTrigger : null;
      }
    }
  }
};
optionSandbox.window.window = optionSandbox.window;
optionSandbox.window.document = optionSandbox.document;
optionSandbox.window.location = optionSandbox.location;

runComposer(optionSandbox);
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
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy,
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

runComposer(toolsSandbox);
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

const sidebarButton = {
  id: '',
  textContent: '',
  getAttribute(name) {
    return name === 'aria-label' ? 'Open tools sidebar' : null;
  }
};
const currentComposerPlus = {
  id: 'composer-plus-btn',
  textContent: '',
  getAttribute(name) {
    return name === 'aria-label' ? 'Add photos and files' : null;
  }
};
const unrelatedFileOption = {
  id: '',
  textContent: '文件',
  getAttribute(name) {
    return name === 'role' ? 'menuitem' : null;
  },
  hasAttribute: () => false,
  getBoundingClientRect: () => ({
    left: 0,
    top: 120,
    right: 120,
    bottom: 170,
    width: 120,
    height: 50
  })
};
const currentComposerScope = {
  querySelector(selector) {
    return selector === '#composer-plus-btn' ? currentComposerPlus : null;
  },
  querySelectorAll: () => []
};
const currentComposer = {
  closest(selector) {
    return selector === '#thread-bottom-container' ? currentComposerScope : null;
  }
};
const currentEvents = [];
const currentResults = [];
const currentSandbox = {
  document: {
    querySelector(selector) {
      return selector === 'button[aria-label*="tool" i]' ? sidebarButton : null;
    },
    querySelectorAll(selector) {
      return selector.includes('[role="menuitem"]') ? [unrelatedFileOption] : [];
    }
  },
  location: { origin: 'https://chatgpt.com' },
  window: {
    innerWidth: 400,
    innerHeight: 800,
    __elonChatGptActionTargetPolicy: {
      actionPoint(node) {
        if (node === currentComposerPlus) return { x: 40, y: 700 };
        if (node === sidebarButton) return { x: 200, y: 60 };
        if (node === unrelatedFileOption) return { x: 60, y: 145 };
        return null;
      },
      signature: () => 'visible'
    },
    __elonChatGptComposerOptionPolicy: {
      filter: (_section, options) => options
    },
    __elonChatGptDictationSessionPolicy: dictationSessionPolicy
  }
};
currentSandbox.window.window = currentSandbox.window;
currentSandbox.window.document = currentSandbox.document;
currentSandbox.window.location = currentSandbox.location;

runComposer(currentSandbox);
currentSandbox.window.__elonChatGptComposer.requestOptions(
  'tools',
  currentComposer,
  (event) => currentEvents.push(event),
  (...args) => currentResults.push(args)
);

assert.equal(currentResults.length, 0);
assert.equal(currentEvents.length, 1);
assert.equal(currentEvents[0].purpose, 'list_composer_tools');
assert.equal(currentEvents[0].xRatio, 0.1);
assert.equal(currentEvents[0].yRatio, 0.875);

process.stdout.write('CHATGPT_COMPOSER_TRIGGER_POLICY=passed\n');

}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
