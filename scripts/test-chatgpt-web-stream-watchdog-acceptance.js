'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const load = (name, context) => vm.runInNewContext(fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', name
), 'utf8'), context, { filename: name });
const context = { window: {} };
load('chatgpt_web_stream_watchdog_probe.js', context);
load('chatgpt_web_stream_watchdog_acceptance.js', context);

let now = 1000;
let scheduled = null;
let snapshotCount = 0;
const results = [];
const policy = {
  scheduleNext(active, next, options) {
    assert.equal(active, true);
    assert.equal(options.privateStreamObserved, true);
    scheduled = { next, schedule: { mode: 'private_stream_watchdog', delayMs: 4000 } };
    return scheduled.schedule;
  },
  reset() {}
};
const acceptance = context.window.__elonChatGptStreamWatchdogAcceptance.create({
  probeModule: context.window.__elonChatGptStreamWatchdogProbe,
  streamingPolicyModule: { create: () => policy },
  now: () => now,
  scheduleTimer: () => 1,
  cancelTimer() {},
  onResult: (requestId, ok, detail) => results.push({ requestId, ok, detail }),
  scheduleSnapshot: () => { snapshotCount += 1; }
});

assert.equal(acceptance.run('mcp_busy', true).detail, 'stream_already_active');
assert.equal(acceptance.run('mcp_acceptance', false).accepted, true);
assert.ok(scheduled);
now += 4000;
scheduled.next(scheduled.schedule);
assert.deepEqual(results.pop(), {
  requestId: 'mcp_acceptance',
  ok: true,
  detail: 'private_stream_watchdog_fired'
});
assert.equal(snapshotCount, 1);
acceptance.dispose();

console.log('ChatGPT stream watchdog acceptance tests passed.');
