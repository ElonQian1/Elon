'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const dictationActionsModule = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_dictation_actions.js'
);
const dictationRuntimeModule = require(
  '../android/app/src/main/assets/chatgpt_web_dictation_runtime.js'
);
const dictationSessionPolicy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_dictation_session_policy.js'
);
const composerSubmenu = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_composer_submenu.js'
);

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

function runComposer(sandbox) {
  sandbox.window.__elonChatGptComposerSubmenu = composerSubmenu;
  sandbox.window.__elonChatGptDictationActions =
    dictationActionsModule.createForRoot(sandbox.window);
  vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_adapter_composer.js' });
}

async function runDictationTriggerTest() {
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
  let resolveDictationMedia;
  let dictationTrackEnded;
  const dictationTrack = {
    kind: 'audio',
    readyState: 'live',
    addEventListener(eventName, listener) {
      if (eventName === 'ended') dictationTrackEnded = listener;
    }
  };
  const dictationMediaDevices = {
    getUserMedia() {
      return {
        then(resolve) {
          resolveDictationMedia = resolve;
        }
      };
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
      navigator: { mediaDevices: dictationMediaDevices },
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
          return semantic === 'dictation' && region === 'composer' ? dictationButton : null;
        },
        requestSemanticTouch(semantic, purpose, emitEvent, region) {
          assert.equal(semantic, 'dictation');
          assert.equal(purpose, 'start_dictation');
          assert.equal(region, 'composer');
          emitEvent({ type: 'web_touch_request', purpose, xRatio: 0.825, yRatio: 0.8375 });
          return true;
        }
      }
    }
  };
  dictationSandbox.window.window = dictationSandbox.window;
  dictationSandbox.window.document = dictationSandbox.document;
  dictationSandbox.window.location = dictationSandbox.location;
  dictationSandbox.window.setTimeout = setTimeout;
  dictationSandbox.window.clearTimeout = clearTimeout;
  dictationSandbox.window.__elonChatGptDictationRuntime =
    dictationRuntimeModule.create(dictationSandbox.window);

  runComposer(dictationSandbox);
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
  assert.equal(dictationResults.length, 0, 'touch dispatch alone must not report dictation started');
  assert.equal(dictationEvents[0].type, 'web_touch_request');
  assert.equal(dictationEvents[0].purpose, 'start_dictation');
  assert.equal(dictationEvents[0].xRatio, 0.825);
  assert.equal(dictationEvents[0].yRatio, 0.8375);
  dictationMediaDevices.getUserMedia({ audio: true });
  assert.equal(
    dictationSandbox.window.__elonChatGptComposer.dictationCapturePending(),
    true,
    'an armed official microphone request must remain pending until media is granted'
  );
  resolveDictationMedia({ getAudioTracks: () => [dictationTrack] });
  await new Promise((resolve) => setTimeout(resolve, 100));
  assert.equal(dictationSandbox.window.__elonChatGptComposer.dictationCapturePending(), false);
  assert.equal(
    dictationSandbox.window.__elonChatGptComposer.dictationCaptureActive(),
    true,
    'official controls alone are insufficient; the live audio track confirms capture'
  );
  assert.deepEqual(Array.from(dictationResults[0]), ['start_dictation', true, 'capture_started']);
  dictationTrack.readyState = 'ended';
  dictationTrackEnded();
  assert.equal(dictationSandbox.window.__elonChatGptComposer.dictationCaptureActive(), false);

  const sessionEvents = [];
  const sessionResults = [];
  const sessionPlus = {
    id: '',
    textContent: '',
    getAttribute: () => null,
    getBoundingClientRect: () => ({ left: 20, top: 710, right: 75, bottom: 765, width: 55, height: 55 })
  };
  const sessionCancel = {
    id: '',
    textContent: '',
    getAttribute: () => null,
    getBoundingClientRect: () => ({ left: 285, top: 710, right: 335, bottom: 760, width: 50, height: 50 })
  };
  const sessionSubmit = {
    id: '',
    textContent: '',
    getAttribute: () => null,
    getBoundingClientRect: () => ({ left: 340, top: 710, right: 390, bottom: 760, width: 50, height: 50 })
  };
  const sessionNodes = [sessionPlus, sessionCancel, sessionSubmit];
  const sessionSandbox = {
    document: {
      querySelector: () => null,
      querySelectorAll(selector) {
        return selector === 'button, [role="button"], [tabindex], svg' ? sessionNodes : [];
      }
    },
    location: { origin: 'https://chatgpt.com' },
    window: {
      innerWidth: 400,
      innerHeight: 800,
      getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
      __elonChatGptActionTargetPolicy: {
        actionPoint(node) {
          if (!sessionNodes.includes(node)) return null;
          const rect = node.getBoundingClientRect();
          return { x: (rect.left + rect.right) / 2, y: (rect.top + rect.bottom) / 2 };
        },
        signature: () => 'visible'
      },
      __elonChatGptComposerOptionPolicy: {
        filter: (_section, options) => options
      },
      __elonChatGptDictationSessionPolicy: dictationSessionPolicy,
      __elonChatGptDictationRuntime: {
        createCaptureTracker: () => ({
          arm() {}, finish() {}, active: () => false, pending: () => false,
          waitForActive: () => Promise.resolve(false), waitForInactive: () => Promise.resolve(true)
        }),
        waitUntil: () => Promise.resolve(true)
      }
    }
  };
  sessionSandbox.window.window = sessionSandbox.window;
  sessionSandbox.window.document = sessionSandbox.document;
  sessionSandbox.window.location = sessionSandbox.location;

  runComposer(sessionSandbox);
  assert.equal(sessionSandbox.window.__elonChatGptComposer.dictationActive(null), true);
  await sessionSandbox.window.__elonChatGptComposer.cancelDictation(
    (event) => sessionEvents.push(event),
    (...args) => sessionResults.push(args)
  );
  await sessionSandbox.window.__elonChatGptComposer.submitDictation(
    (event) => sessionEvents.push(event),
    (...args) => sessionResults.push(args)
  );
  assert.deepEqual(Array.from(sessionResults[0]), ['cancel_dictation', true, 'capture_finished']);
  assert.deepEqual(Array.from(sessionResults[1]), ['submit_dictation', true, 'capture_finished']);
  assert.equal(sessionEvents[0].purpose, 'cancel_dictation');
  assert.equal(sessionEvents[0].xRatio, 0.775);
  assert.equal(sessionEvents[1].purpose, 'submit_dictation');
  assert.equal(sessionEvents[1].xRatio, 0.9125);

  sessionEvents.length = 0;
  sessionResults.length = 0;
  sessionSandbox.window.__elonChatGptActionTargetPolicy.actionPoint = () => null;
  assert.equal(sessionSandbox.window.__elonChatGptComposer.dictationActive(null), true);
  await sessionSandbox.window.__elonChatGptComposer.cancelDictation(
    (event) => sessionEvents.push(event),
    (...args) => sessionResults.push(args)
  );
  assert.deepEqual(Array.from(sessionResults[0]), ['cancel_dictation', true, 'capture_finished']);
  assert.equal(sessionEvents[0].purpose, 'cancel_dictation');
  assert.equal(sessionEvents[0].xRatio, 0.775);
}

module.exports = runDictationTriggerTest;

if (require.main === module) {
  runDictationTriggerTest().then(() => {
    process.stdout.write('CHATGPT_COMPOSER_DICTATION_TRIGGER=passed\n');
  }).catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
