(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;

  const adapterVersion = Number(window.__elonChatGptAdapterTargetVersion || 0);
  if (!Number.isInteger(adapterVersion) || adapterVersion <= 0) return;
  if (window.__elonChatGptAdapterVersion === adapterVersion) return;

  const previousBridge = window.__elonChatGptBridge;
  if (previousBridge && typeof previousBridge.dispose === 'function') {
    previousBridge.dispose();
  }

  [
    '__elonChatGptConversations',
    '__elonChatGptConversationHistory',
    '__elonChatGptMessages',
    '__elonChatGptComposerOptionPolicy',
    '__elonChatGptActionTargetPolicy',
    '__elonChatGptModelLabelPolicy',
    '__elonChatGptComposer',
    '__elonChatGptNavigationPolicy',
    '__elonChatGptNavigation',
    '__elonChatGptPageSemanticPolicy',
    '__elonChatGptFormControls',
    '__elonChatGptControlOwnershipPolicy',
    '__elonChatGptFormCommands',
    '__elonChatGptDisclosureControls',
    '__elonChatGptLayout',
    '__elonChatGptBridge'
  ].forEach((name) => {
    try { delete window[name]; }
    catch { window[name] = undefined; }
  });

  window.__elonChatGptAdapterVersion = adapterVersion;
})();
