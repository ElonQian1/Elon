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
  'chatgpt_web_stream_watchdog_probe.js'
), 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context, { filename: 'chatgpt_web_stream_watchdog_probe.js' });

let now = 1000;
let timerId = 0;
const timers = new Map();
const results = [];
const create = () => context.window.__elonChatGptStreamWatchdogProbe.create({
  now: () => now,
  timeoutMs: 5000,
  minimumStallMs: 3500,
  scheduleTimer(delay, action) {
    const id = ++timerId;
    timers.set(id, { delay, action });
    return id;
  },
  cancelTimer(id) { timers.delete(id); },
  onResult(requestId, ok, detail) { results.push({ requestId, ok, detail }); }
});

{
  const probe = create();
  assert.equal(probe.arm('bad', false).accepted, false);
  assert.equal(probe.arm('mcp_probe1', true).detail, 'stream_already_active');
  assert.equal(probe.arm('mcp_probe1', false).accepted, true);
  assert.equal(probe.arm('mcp_probe2', false).detail, 'watchdog_probe_already_active');
  assert.equal(probe.observePrivateUpdate('idle'), false, 'the pre-send reset must not arm a stall');
  assert.equal(probe.state().phase, 'armed');
  assert.equal(probe.observePrivateUpdate('streaming'), false, 'the first stream update enters native streaming');
  assert.equal(probe.observePrivateUpdate('streaming'), true, 'later stream updates are withheld for the probe');
  now += 3499;
  assert.equal(probe.watchdogFired({ mode: 'private_stream_watchdog' }), false);
  now += 1;
  assert.equal(probe.watchdogFired({ mode: 'dom_heartbeat' }), false);
  assert.equal(probe.watchdogFired({ mode: 'private_stream_watchdog' }), true);
  assert.deepEqual(results.pop(), {
    requestId: 'mcp_probe1',
    ok: true,
    detail: 'private_stream_watchdog_fired'
  });
  assert.equal(probe.state().active, false);
}

{
  const probe = create();
  assert.equal(probe.arm('mcp_probe4', false).accepted, true);
  assert.equal(probe.observePrivateUpdate('idle'), false);
  assert.equal(probe.observePrivateUpdate('completed'), false);
  assert.deepEqual(results.pop(), {
    requestId: 'mcp_probe4',
    ok: false,
    detail: 'private_stream_completed_before_watchdog'
  });
  assert.equal(probe.state().active, false);
}

{
  const probe = create();
  assert.equal(probe.arm('mcp_probe3', false).accepted, true);
  const timeout = Array.from(timers.values()).find((entry) => entry.delay === 5000);
  assert.ok(timeout);
  timeout.action();
  assert.deepEqual(results.pop(), {
    requestId: 'mcp_probe3',
    ok: false,
    detail: 'private_stream_watchdog_timeout'
  });
  probe.dispose();
}

console.log('ChatGPT stream watchdog probe tests passed.');
