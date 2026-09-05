'use strict';

const assert = require('node:assert/strict');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_composer_dismiss_policy.js'
);

function hit(kind) {
  return {
    closest(selector) {
      if (kind === 'overlay' && selector.includes('[role="dialog"]')) return this;
      if (kind === 'interactive' && selector.includes('button')) return this;
      return null;
    }
  };
}

const view = { innerWidth: 400, innerHeight: 800 };
const hits = [hit('overlay'), hit('interactive'), hit('safe')];
const points = [];
const documentRef = {
  elementFromPoint(x, y) {
    points.push([x, y]);
    return hits.shift();
  }
};
const events = [];
assert.equal(policy.emitTouch(documentRef, view, (event) => events.push(event)), true);
assert.equal(points.length, 3);
assert.deepEqual(events, [{
  type: 'web_touch_request',
  purpose: 'dismiss_composer_menu',
  xRatio: 0.75,
  yRatio: 0.3
}]);

assert.equal(policy.safePoint({ elementFromPoint: () => hit('overlay') }, view), null);
assert.equal(policy.emitTouch(null, view, () => {}), false);

process.stdout.write('CHATGPT_WEB_COMPOSER_DISMISS_POLICY=passed\n');
