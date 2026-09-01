'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_invocation.js');

function observation(state) {
  let onOpened = null;
  let onTimedOut = null;
  const observe = (opened, timedOut) => {
    onOpened = opened;
    onTimedOut = timedOut;
    return true;
  };
  observe.isOpen = () => state.open;
  observe.open = () => onOpened();
  observe.timeout = () => onTimedOut();
  return observe;
}

const successful = policy.createCoordinator();
const successfulState = { open: false };
const successfulObservation = observation(successfulState);
const successfulResults = [];
let successfulTouches = 0;
assert.equal(successful.start(
  successfulObservation,
  () => { successfulTouches += 1; return true; },
  (ok) => successfulResults.push(ok)
), true);
assert.equal(successfulTouches, 1);
assert.equal(successful.reconcile(), true);
assert.equal(successfulTouches, 2, 'the first Android snapshot retries one missing menu');
successfulState.open = true;
assert.equal(successful.reconcile(), true);
assert.deepEqual(successfulResults, [true]);
successfulObservation.timeout();
assert.deepEqual(successfulResults, [true], 'late page timers cannot settle twice');

const failed = policy.createCoordinator();
const failedResults = [];
let failedTouches = 0;
assert.equal(failed.start(
  observation({ open: false }),
  () => { failedTouches += 1; return true; },
  (ok) => failedResults.push(ok)
), true);
failed.reconcile();
failed.reconcile();
assert.equal(failedTouches, 2);
assert.deepEqual(failedResults, [false], 'a second missing snapshot fails without another touch');

process.stdout.write('chatgpt context menu invocation tests passed\n');
