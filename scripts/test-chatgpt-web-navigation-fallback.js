'use strict';

const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_navigation_policy.js'
);

let assignedPath = '';
let sidebarExpanded = false;
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
    if (name === 'aria-label') return 'Open sidebar';
    if (name === 'aria-expanded') return sidebarExpanded ? 'true' : 'false';
    return null;
  },
  getBoundingClientRect() {
    return { left: 8, top: 8, right: 48, bottom: 48, width: 40, height: 40 };
  }
};
const persistentFeature = {
  textContent: 'Health',
  getAttribute(name) {
    if (name === 'href') return '/health';
    return null;
  },
  getBoundingClientRect() {
    return { left: 16, top: 120, right: 200, bottom: 168, width: 184, height: 48 };
  },
  closest() {
    return null;
  }
};
const persistentNavigation = {
  getBoundingClientRect() {
    return { left: 0, top: 0, right: 400, bottom: 800, width: 400, height: 800 };
  },
  querySelectorAll() {
    return [persistentFeature];
  }
};
global.document = {
  querySelectorAll(selector) {
    if (selector === 'button') return [openSidebarButton];
    if (selector.includes('aside')) return [persistentNavigation];
    return [];
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
  throw new Error('persistent page navigation suppressed the official sidebar trigger');
}
if (!results.some((entry) => entry.action === 'list_navigation' && entry.ok)) {
  throw new Error('official sidebar request did not succeed');
}

sidebarExpanded = true;
events.length = 0;
results.length = 0;
window.__elonChatGptNavigation.requestList(emitEvent, result);
if (events.some((event) => event.type === 'web_touch_request')) {
  throw new Error('expanded sidebar toggle was touched again');
}
window.__elonChatGptNavigation.dismiss(emitEvent, result);
if (!events.some((event) => event.type === 'web_touch_request' && event.purpose === 'dismiss_navigation')) {
  throw new Error('expanded sidebar toggle was not reused to dismiss navigation');
}
if (!results.some((entry) => entry.action === 'dismiss_navigation' && entry.ok)) {
  throw new Error('expanded sidebar dismiss did not succeed');
}

window.__elonChatGptNavigation.collectList(emitEvent, result);
const snapshot = events.find((event) => event.type === 'navigation_snapshot');
if (!snapshot || snapshot.features.length !== 2) {
  throw new Error('built-in image feature was not published');
}
const imageFeature = snapshot.features.find((feature) => feature.kind === 'images');
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
