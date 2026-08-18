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
global.document = {
  querySelectorAll() {
    return [];
  }
};
global.window = {
  __elonChatGptNavigationPolicy: policy,
  getComputedStyle() {
    return { display: 'block', visibility: 'visible' };
  }
};

require('../android/app/src/main/assets/chatgpt_web_adapter_navigation.js');

const events = [];
const results = [];
const emitEvent = (event) => events.push(event);
const result = (action, ok, error) => results.push({ action, ok, error });

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
