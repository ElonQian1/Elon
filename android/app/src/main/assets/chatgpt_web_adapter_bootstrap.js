(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;

  const adapterVersion = 4;
  if (window.__elonChatGptAdapterVersion === adapterVersion) return;

  const previousBridge = window.__elonChatGptBridge;
  if (previousBridge && typeof previousBridge.dispose === 'function') {
    previousBridge.dispose();
  }

  [
    '__elonChatGptConversations',
    '__elonChatGptMessages',
    '__elonChatGptComposer',
    '__elonChatGptNavigation',
    '__elonChatGptLayout',
    '__elonChatGptBridge'
  ].forEach((name) => {
    try { delete window[name]; }
    catch { window[name] = undefined; }
  });

  window.__elonChatGptAdapterVersion = adapterVersion;
})();
