'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_action_target_policy.js');

function createEnvironment() {
  const styles = new Map();
  let hitResolver = () => null;
  const view = {
    innerWidth: 400,
    innerHeight: 800,
    getComputedStyle(node) {
      return styles.get(node) || {
        display: 'block',
        visibility: 'visible',
        contentVisibility: 'visible',
        pointerEvents: 'auto',
        opacity: '1',
        overflow: 'visible',
        overflowX: 'visible',
        overflowY: 'visible'
      };
    }
  };
  const documentRef = {
    defaultView: view,
    elementFromPoint(x, y) { return hitResolver(x, y); }
  };
  function node(rect, parent = null, attributes = {}) {
    const value = {
      nodeType: 1,
      ownerDocument: documentRef,
      parentElement: parent,
      textContent: attributes.textContent || '',
      getBoundingClientRect: () => rect,
      getAttribute: (name) => attributes[name] || null,
      contains(target) {
        let current = target;
        while (current) {
          if (current === value) return true;
          current = current.parentElement;
        }
        return false;
      }
    };
    return value;
  }
  return {
    styles,
    node,
    setHitResolver(value) { hitResolver = value; }
  };
}

{
  const env = createEnvironment();
  const target = env.node({ left: 20, top: 30, right: 180, bottom: 90 });
  env.setHitResolver(() => target);
  assert.deepEqual(policy.actionPoint(target), { x: 100, y: 60 });
}

{
  const env = createEnvironment();
  const hiddenParent = env.node({ left: 0, top: 0, right: 300, bottom: 300 });
  const target = env.node({ left: 20, top: 30, right: 180, bottom: 90 }, hiddenParent);
  env.styles.set(hiddenParent, {
    display: 'block', visibility: 'visible', contentVisibility: 'visible',
    pointerEvents: 'auto', opacity: '0', overflow: 'visible', overflowX: 'visible', overflowY: 'visible'
  });
  env.setHitResolver(() => target);
  assert.equal(policy.actionPoint(target), null);
}

{
  const env = createEnvironment();
  const clippingParent = env.node({ left: 0, top: 0, right: 100, bottom: 100 });
  const target = env.node({ left: 80, top: 80, right: 180, bottom: 180 }, clippingParent);
  env.styles.set(clippingParent, {
    display: 'block', visibility: 'visible', contentVisibility: 'visible',
    pointerEvents: 'auto', opacity: '1', overflow: 'hidden', overflowX: 'hidden', overflowY: 'hidden'
  });
  env.setHitResolver((x, y) => x < 100 && y < 100 ? target : null);
  assert.deepEqual(policy.actionPoint(target), { x: 90, y: 90 });
}

{
  const env = createEnvironment();
  const target = env.node({ left: 20, top: 30, right: 180, bottom: 90 });
  const overlay = env.node({ left: 0, top: 0, right: 400, bottom: 800 });
  env.setHitResolver(() => overlay);
  assert.equal(policy.actionPoint(target), null);
}

{
  const env = createEnvironment();
  const attributes = { role: 'menuitem', 'aria-label': 'Model A', textContent: 'Model A' };
  const target = env.node({ left: 20, top: 30, right: 180, bottom: 90 }, null, attributes);
  const before = policy.signature(target);
  attributes['aria-label'] = 'Model B';
  attributes.textContent = 'Model B';
  assert.notEqual(policy.signature(target), before);
}

process.stdout.write('CHATGPT_ACTION_TARGET_POLICY=passed\n');
