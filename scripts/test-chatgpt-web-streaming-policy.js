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
  'chatgpt_web_adapter_streaming_policy.js'
), 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context);

let now = 1000;
const policy = context.window.__elonChatGptStreamingPolicy.create({
  now: () => now,
  settleMs: 2500,
  waitTimeoutMs: 30000
});
const oldAnswer = observation('assistant-1', 'old answer', true);

assert.deepEqual(plain(policy.observe(oldAnswer, false)), {
  active: false,
  assistantKey: ''
});

policy.begin(oldAnswer);
assert.deepEqual(plain(policy.observe(oldAnswer, false)), {
  active: true,
  assistantKey: ''
}, 'arming a send keeps the request active without marking the previous answer as streaming');

now += 120;
const pending = observation('assistant-2', '', false, true);
assert.deepEqual(plain(policy.observe(pending, false)), {
  active: true,
  assistantKey: 'assistant-2'
});

now += 120;
const firstToken = observation('assistant-2', '今天', false);
assert.deepEqual(plain(policy.observe(firstToken, false)), {
  active: true,
  assistantKey: 'assistant-2'
}, 'the first substantive token must not complete the answer');

now += 900;
const growing = observation('assistant-2', '今天市场震荡', false);
assert.deepEqual(plain(policy.observe(growing, false)), {
  active: true,
  assistantKey: 'assistant-2'
});

now += 2100;
assert.equal(policy.observe(growing, false).active, true, 'a short token pause remains streaming');

now += 500;
assert.deepEqual(plain(policy.observe(growing, false)), {
  active: false,
  assistantKey: ''
}, 'a stable answer eventually completes when the official page exposes no finish control');

policy.begin(growing, { allowSameTurn: true });
now += 100;
assert.deepEqual(plain(policy.observe(observation('assistant-2', '重新生成中', false), true)), {
  active: true,
  assistantKey: 'assistant-2'
}, 'same-turn regeneration stays attached to the existing assistant turn');

now += 5000;
assert.equal(
  policy.observe(observation('assistant-2', '重新生成中', false), true).active,
  true,
  'an official active marker overrides quiet-time completion'
);

now += 100;
const regenerated = observation('assistant-2', '重新生成完成', true);
assert.equal(policy.observe(regenerated, false).active, true);
now += 400;
assert.deepEqual(plain(policy.observe(regenerated, false)), {
  active: false,
  assistantKey: ''
}, 'an official completion affordance ends streaming after one quiet confirmation');

policy.begin(regenerated, { allowSameTurn: true });
now += 100;
assert.deepEqual(plain(context.window.__elonChatGptStreamingPolicy.readState(
  policy,
  { lastAssistantObservation: () => observation('assistant-2', 'private completion', false) },
  { querySelector: () => null },
  null,
  () => false,
  { privateStreamState: 'completed' }
)), {
  active: false,
  assistantKey: ''
}, 'a completed private stream immediately releases stale native streaming when the official stop control is absent');

policy.begin(regenerated, { allowSameTurn: true });
assert.equal(context.window.__elonChatGptStreamingPolicy.readState(
  policy,
  { lastAssistantObservation: () => observation('assistant-2', 'private completion', false) },
  { querySelector: () => ({}) },
  null,
  () => true,
  { privateStreamState: 'completed' }
).active, true, 'an official stop control remains authoritative over a premature private completion');
policy.reset();

policy.begin(observation('assistant-2', '完成', true));
now += 30001;
assert.equal(
  policy.observe(observation('assistant-2', '完成', true), false).active,
  false,
  'a missing next assistant cannot keep the native UI blocked forever'
);

{
  const timers = new Map();
  let timerId = 0;
  let heartbeats = 0;
  const heartbeatPolicy = context.window.__elonChatGptStreamingPolicy.create({
    now: () => now,
    scheduleTimer(delay, action) {
      const id = ++timerId;
      timers.set(id, {
        delay,
        action() {
          timers.delete(id);
          action();
        }
      });
      return id;
    },
    cancelTimer(id) { timers.delete(id); },
    heartbeatMs: 400,
    privateStreamWatchdogMs: 4000
  });
  heartbeatPolicy.scheduleNext(true, () => { heartbeats += 1; });
  assert.equal(Array.from(timers.values())[0].delay, 400);
  heartbeatPolicy.scheduleNext(true, () => { heartbeats += 1; });
  assert.equal(timers.size, 1, 'new DOM snapshots replace the pending heartbeat');
  const timer = Array.from(timers.values())[0];
  timer.action();
  assert.equal(heartbeats, 1);
  heartbeatPolicy.scheduleNext(true, () => { heartbeats += 1; }, {
    privateStreamObserved: true
  });
  assert.equal(Array.from(timers.values())[0].delay, 4000,
    'an observed private stream replaces dense DOM heartbeats with a sparse watchdog');
  heartbeatPolicy.scheduleNext(true, () => { heartbeats += 1; }, {
    privateStreamObserved: true
  });
  assert.equal(timers.size, 1, 'private-stream progress cannot stack watchdog timers');
  Array.from(timers.values())[0].action();
  assert.equal(heartbeats, 2);
  heartbeatPolicy.dispose();
}

function observation(key, fingerprint, completionVisible, pending = false) {
  return { key, fingerprint, completionVisible, pending };
}

function plain(value) {
  return JSON.parse(JSON.stringify(value));
}

console.log('CHATGPT_WEB_STREAMING_POLICY_TESTS=passed');
