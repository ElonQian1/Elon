'use strict';

const assert = require('assert');
const history = require('../android/app/src/main/assets/chatgpt_web_adapter_conversation_history.js');

function conversation(id) {
  return { id, title: `Conversation ${id}`, path: `/c/${id}`, active: false };
}

function runCollection(options) {
  return new Promise((resolve) => history.collect(options, resolve));
}

async function collectsVirtualizedPagesAndRestoresTheOriginalScrollPosition() {
  let now = 0;
  const scroller = {
    scrollTop: 120,
    clientHeight: 400,
    scrollHeight: 1600,
    dispatchEvent() {}
  };
  const pages = [
    [conversation('one'), conversation('two')],
    [conversation('two'), conversation('three')],
    [conversation('three'), conversation('four')]
  ];
  const result = await runCollection({
    initial: pages[0],
    read() {
      const page = Math.min(pages.length - 1, Math.floor(scroller.scrollTop / 400));
      if (page === pages.length - 1) scroller.scrollHeight = scroller.clientHeight + scroller.scrollTop;
      return pages[page];
    },
    findScroller: () => scroller,
    schedule(callback, delayMs) {
      now += delayMs;
      callback();
    },
    now: () => now,
    delayMs: 20,
    stablePasses: 2,
    maxSteps: 20,
    timeoutMs: 2000
  });

  assert.deepStrictEqual(result.conversations.map((item) => item.path), [
    '/c/one', '/c/two', '/c/three', '/c/four'
  ]);
  assert.strictEqual(result.collection.scrolled, true);
  assert.strictEqual(result.collection.scrollRestored, true);
  assert.strictEqual(result.collection.reachedEnd, true);
  assert.strictEqual(result.collection.timedOut, false);
  assert.strictEqual(scroller.scrollTop, 120);
}

async function capsResultsAndReportsTruncation() {
  const scroller = {
    scrollTop: 0,
    clientHeight: 100,
    scrollHeight: 1000,
    dispatchEvent() {}
  };
  const result = await runCollection({
    initial: [conversation('one')],
    read: () => [conversation('one'), conversation('two'), conversation('three')],
    findScroller: () => scroller,
    schedule: (callback) => callback(),
    maximum: 2
  });

  assert.deepStrictEqual(result.conversations.map((item) => item.path), ['/c/one', '/c/two']);
  assert.strictEqual(result.collection.truncated, true);
  assert.strictEqual(result.collection.reachedEnd, false);
  assert.strictEqual(scroller.scrollTop, 0);
}

async function reportsAConservativeSnapshotWhenNoScrollerExists() {
  const result = await runCollection({
    initial: [conversation('one')],
    read: () => [conversation('one')],
    findScroller: () => null
  });

  assert.strictEqual(result.collection.scrollerFound, false);
  assert.strictEqual(result.collection.reachedEnd, false);
  assert.strictEqual(result.collection.scrollRestored, true);
}

Promise.resolve()
  .then(collectsVirtualizedPagesAndRestoresTheOriginalScrollPosition)
  .then(capsResultsAndReportsTruncation)
  .then(reportsAConservativeSnapshotWhenNoScrollerExists)
  .then(() => process.stdout.write('chatgpt conversation history policy tests passed\n'))
  .catch((error) => {
    process.stderr.write(`${error.stack || error}\n`);
    process.exitCode = 1;
  });
