'use strict';

const assert = require('node:assert/strict');

const scheduled = [];
global.window = {
  setTimeout(callback) {
    scheduled.push(callback);
  }
};
const selection = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_composer_tool_selection.js'
);

function drain(limit = 128) {
  let count = 0;
  while (scheduled.length && count < limit) {
    scheduled.shift()();
    count += 1;
  }
  assert.ok(count < limit, 'selection polling did not settle');
}

function context(overrides = {}) {
  const calls = [];
  const value = {
    semantic: 'web_search',
    toolLabel: '网页搜索',
    optionNode: {},
    desiredSelected: true,
    directSelection: () => ({ known: false, selected: false }),
    composerSelection: () => ({ known: false, selected: false }),
    menuSettled: () => true,
    menuSettledFor: () => true,
    openVerificationMenu: (ready) => ready([{
      semantic: 'web_search',
      directStateKnown: true,
      selected: true,
      node: {}
    }]),
    retryTouch: () => true,
    complete(ok, detail) {
      calls.push({ ok, detail });
    }
  };
  return { value: Object.assign(value, overrides), calls };
}

{
  const test = context();
  selection.select(test.value);
  drain();
  assert.deepEqual(test.calls, [{ ok: true, detail: '' }]);
}

{
  let observations = 0;
  const test = context({
    semantic: 'image_generation',
    toolLabel: '创建图片',
    composerSelection() {
      observations += 1;
      return observations < 3
        ? { known: false, selected: false }
        : { known: true, selected: true };
    },
    openVerificationMenu() {
      assert.fail('delayed composer state should settle without reopening the menu');
    }
  });
  selection.select(test.value);
  drain();
  assert.ok(observations >= 6);
  assert.deepEqual(test.calls, [{ ok: true, detail: '' }]);
}

{
  let selected = false;
  let retries = 0;
  const test = context({
    openVerificationMenu(ready) {
      ready([{
        semantic: 'web_search',
        directStateKnown: true,
        selected,
        node: {}
      }]);
    },
    retryTouch() {
      retries += 1;
      selected = true;
      return true;
    },
    directSelection() {
      return { known: true, selected };
    },
    menuSettledFor: () => false
  });
  selection.select(test.value);
  drain();
  assert.equal(retries, 1);
  assert.deepEqual(test.calls, [{ ok: true, detail: '' }]);
}

{
  let selected = false;
  let retries = 0;
  const test = context({
    menuSettled: () => false,
    directSelection: () => ({ known: true, selected }),
    retryTouch() {
      retries += 1;
      selected = true;
      return true;
    }
  });
  selection.select(test.value);
  drain();
  assert.equal(retries, 1);
  assert.deepEqual(test.calls, [{ ok: true, detail: '' }]);
}

{
  let retries = 0;
  const test = context({
    openVerificationMenu(ready) {
      ready([{
        semantic: 'web_search',
        directStateKnown: true,
        selected: false,
        node: {}
      }]);
    },
    retryTouch() {
      retries += 1;
      return true;
    }
  });
  selection.select(test.value);
  drain();
  assert.equal(retries, 1);
  assert.equal(test.calls.length, 1);
  assert.equal(test.calls[0].ok, false);
  assert.match(test.calls[0].detail, /未发生预期变化/);
}

{
  const test = context({
    semantic: 'image_generation',
    toolLabel: '创建图片',
    openVerificationMenu(ready) {
      ready([{ semantic: 'image_generation', directStateKnown: false, selected: false, node: {} }]);
    }
  });
  selection.select(test.value);
  drain();
  assert.equal(test.calls[0].ok, false);
  assert.match(test.calls[0].detail, /可验证的创建图片状态/);
}

process.stdout.write('CHATGPT_COMPOSER_TOOL_SELECTION=passed\n');
