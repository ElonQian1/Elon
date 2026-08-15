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

async function allowsSlowVirtualizedHistoryToReachTheEnd() {
  let now = 0;
  let page = 0;
  const scroller = {
    scrollTop: 0,
    clientHeight: 400,
    scrollHeight: 3600,
    dispatchEvent() {}
  };
  const result = await runCollection({
    initial: [conversation('zero')],
    read() {
      page += 1;
      if (page >= 7) {
        scroller.scrollHeight = scroller.clientHeight + scroller.scrollTop;
      }
      return Array.from({ length: Math.min(page + 1, 8) }, (_, index) =>
        conversation(String(index))
      );
    },
    findScroller: () => scroller,
    schedule(callback) {
      now += 600;
      callback();
    },
    now: () => now,
    timeoutMs: 10000,
    delayMs: 180,
    stablePasses: 3,
    maxSteps: 40
  });

  assert.strictEqual(result.collection.reachedEnd, true);
  assert.strictEqual(result.collection.timedOut, false);
  assert.strictEqual(result.collection.scrollRestored, true);
  assert.ok(result.conversations.length >= 8);
}

async function renewsTheStallBudgetWhileHistoryKeepsGrowing() {
  let now = 0;
  let page = 0;
  const scroller = {
    scrollTop: 0,
    clientHeight: 400,
    scrollHeight: 4800,
    dispatchEvent() {}
  };
  const result = await runCollection({
    initial: [conversation('zero')],
    read() {
      page += 1;
      if (page >= 7) {
        scroller.scrollHeight = scroller.clientHeight + scroller.scrollTop;
      }
      return Array.from({ length: Math.min(page + 1, 8) }, (_, index) =>
        conversation(String(index))
      );
    },
    findScroller: () => scroller,
    schedule(callback) {
      now += 2500;
      callback();
    },
    now: () => now,
    timeoutMs: 10000,
    absoluteTimeoutMs: 30000,
    delayMs: 180,
    stablePasses: 2,
    maxSteps: 40
  });

  assert.ok(now > 10000);
  assert.strictEqual(result.collection.reachedEnd, true);
  assert.strictEqual(result.collection.timedOut, false);
}

async function timesOutWhenHistoryStopsMakingProgress() {
  let now = 0;
  const scroller = {
    scrollTop: 0,
    clientHeight: 400,
    scrollHeight: 4800,
    dispatchEvent() {}
  };
  const result = await runCollection({
    initial: [conversation('zero')],
    read: () => [conversation('zero')],
    findScroller: () => scroller,
    schedule(callback) {
      now += 2500;
      callback();
    },
    now: () => now,
    timeoutMs: 10000,
    absoluteTimeoutMs: 30000,
    delayMs: 180,
    stablePasses: 2,
    maxSteps: 40
  });

  assert.strictEqual(result.collection.reachedEnd, false);
  assert.strictEqual(result.collection.timedOut, true);
  assert.ok(now < 30000);
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

async function mergesActivityDatesWhenVirtualizedRowsAreObservedAgain() {
  const first = Object.assign(conversation('one'), { activityDates: ['2026-08-13'] });
  const refreshed = Object.assign(conversation('one'), { activityDates: ['2026-08-14'] });
  const result = await runCollection({
    initial: [first],
    read: () => [refreshed],
    findScroller: () => null,
  });

  assert.deepStrictEqual(result.conversations[0].activityDates, ['2026-08-13', '2026-08-14']);
}

async function collapsesRecentAndProjectRoutesForTheSameConversation() {
  const recent = Object.assign(conversation('one'), { activityDates: ['2026-08-13'] });
  const project = Object.assign(conversation('one'), {
    path: '/g/g-p-demo/c/one',
    projectId: 'g-p-demo',
    projectTitle: 'Mobile project',
    projectPath: '/g/g-p-demo/project',
    activityDates: ['2026-08-14']
  });
  const result = await runCollection({
    initial: [recent, project],
    read: () => [recent, project],
    findScroller: () => null,
  });

  assert.strictEqual(result.conversations.length, 1);
  assert.strictEqual(result.conversations[0].path, '/g/g-p-demo/c/one');
  assert.deepStrictEqual(result.conversations[0].activityDates, ['2026-08-13', '2026-08-14']);
}

Promise.resolve()
  .then(collectsVirtualizedPagesAndRestoresTheOriginalScrollPosition)
  .then(capsResultsAndReportsTruncation)
  .then(allowsSlowVirtualizedHistoryToReachTheEnd)
  .then(renewsTheStallBudgetWhileHistoryKeepsGrowing)
  .then(timesOutWhenHistoryStopsMakingProgress)
  .then(reportsAConservativeSnapshotWhenNoScrollerExists)
  .then(mergesActivityDatesWhenVirtualizedRowsAreObservedAgain)
  .then(collapsesRecentAndProjectRoutesForTheSameConversation)
  .then(() => process.stdout.write('chatgpt conversation history policy tests passed\n'))
  .catch((error) => {
    process.stderr.write(`${error.stack || error}\n`);
    process.exitCode = 1;
  });
