'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_composer_dismiss_policy.js');

const OVERLAY_SELECTOR = '[role="dialog"]';
const INTERACTIVE_SELECTOR = '[role="button"]';

function createEnvironment() {
  const view = { innerWidth: 400, innerHeight: 800 };
  let hit = null;
  const documentRef = { elementFromPoint: () => hit };

  function node({ parent = null, role = '', rect = { width: 20, height: 20 } } = {}) {
    const value = {
      parentElement: parent,
      getBoundingClientRect: () => rect,
      closest(selector) {
        let current = value;
        while (current) {
          if (selector === OVERLAY_SELECTOR && current.role === 'dialog') return current;
          if (selector === INTERACTIVE_SELECTOR && current.role === 'button') return current;
          if (selector.includes('[role="dialog"]') && current.role === 'dialog') return current;
          if (selector.includes('[role="button"]') && current.role === 'button') return current;
          current = current.parentElement;
        }
        return null;
      },
      role
    };
    return value;
  }

  return { documentRef, node, setHit: (value) => { hit = value; }, view };
}

{
  const env = createEnvironment();
  env.setHit(env.node());
  assert.deepEqual(policy.safePoint(env.documentRef, env.view), { xRatio: 0.5, yRatio: 0.12 });
}

{
  const env = createEnvironment();
  const portal = env.node({ role: 'dialog', rect: { width: 400, height: 800 } });
  const backdrop = env.node({ parent: portal, rect: { width: 400, height: 800 } });
  env.setHit(backdrop);
  assert.deepEqual(policy.safePoint(env.documentRef, env.view), { xRatio: 0.5, yRatio: 0.12 });
}

{
  const env = createEnvironment();
  const popup = env.node({ role: 'dialog', rect: { width: 280, height: 420 } });
  env.setHit(env.node({ parent: popup }));
  assert.equal(policy.safePoint(env.documentRef, env.view), null);
}

{
  const env = createEnvironment();
  const portal = env.node({ role: 'dialog', rect: { width: 400, height: 800 } });
  env.setHit(env.node({ parent: portal, role: 'button', rect: { width: 160, height: 48 } }));
  assert.equal(policy.safePoint(env.documentRef, env.view), null);
}

{
  const env = createEnvironment();
  const portal = env.node({ role: 'dialog', rect: { width: 400, height: 800 } });
  env.setHit(env.node({ parent: portal, rect: { width: 400, height: 800 } }));
  const events = [];
  assert.equal(policy.emitTouch(env.documentRef, env.view, (event) => events.push(event)), true);
  assert.deepEqual(events, [{
    type: 'web_touch_request',
    purpose: 'dismiss_composer_menu',
    xRatio: 0.5,
    yRatio: 0.12
  }]);
}

assert.equal(policy.emitTouch(null, { innerWidth: 400, innerHeight: 800 }, () => {}), false);

process.stdout.write('CHATGPT_WEB_COMPOSER_DISMISS_POLICY=passed\n');
