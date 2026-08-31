'use strict';

const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_navigation_policy.js'
);

let assignedPath = '';
global.location = {
  origin: 'https://chatgpt.com',
  pathname: '/',
  assign(path) {
    assignedPath = path;
  }
};
const openSidebarButton = {
  textContent: '',
  getAttribute(name) {
    return name === 'aria-label' ? 'Open sidebar' : null;
  },
  getBoundingClientRect() {
    return { left: 8, top: 8, right: 48, bottom: 48, width: 40, height: 40 };
  }
};
global.document = {
  querySelectorAll(selector) {
    return selector === 'button' ? [openSidebarButton] : [];
  }
};
global.window = {
  __elonChatGptNavigationPolicy: policy,
  innerHeight: 800,
  innerWidth: 400,
  getComputedStyle() {
    return { display: 'block', visibility: 'visible' };
  }
};

require('../android/app/src/main/assets/chatgpt_web_adapter_navigation.js');

const events = [];
const results = [];
const emitEvent = (event) => events.push(event);
const result = (action, ok, error) => results.push({ action, ok, error });

window.__elonChatGptNavigation.requestList(emitEvent, result);
if (!events.some((event) => event.type === 'web_touch_request' && event.purpose === 'list_navigation')) {
  throw new Error('built-in fallback suppressed the official sidebar trigger');
}
if (!results.some((entry) => entry.action === 'list_navigation' && entry.ok)) {
  throw new Error('official sidebar request did not succeed');
}

window.__elonChatGptNavigation.collectList(emitEvent, result);
const snapshot = events.find((event) => event.type === 'navigation_snapshot');
if (!snapshot || snapshot.features.length !== 1) {
  throw new Error('built-in image feature was not published');
}
const imageFeature = snapshot.features[0];
if (imageFeature.kind !== 'images' || imageFeature.label !== '图像') {
  throw new Error('built-in image feature metadata is invalid');
}

window.__elonChatGptNavigation.selectFeature(imageFeature.id, emitEvent, result);
if (assignedPath !== '/images') {
  throw new Error(`expected /images navigation, got ${JSON.stringify(assignedPath)}`);
}
if (!results.some((entry) => entry.action === 'select_navigation' && entry.ok)) {
  throw new Error('built-in image feature selection did not succeed');
}

console.log('CHATGPT_WEB_NAVIGATION_FALLBACK=passed');
