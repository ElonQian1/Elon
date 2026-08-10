(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptControlOwnershipPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const COMPOSER_SELECTORS = Object.freeze([
    '[data-testid="prompt-textarea"]',
    '#prompt-textarea',
    'form [contenteditable="true"]',
    'form textarea',
    'main [contenteditable="true"]',
    'textarea[placeholder]'
  ]);

  function contains(owner, candidate) {
    return !!owner && typeof owner.contains === 'function' && owner.contains(candidate);
  }

  function findVisibleComposer(root, isVisible) {
    if (!root || typeof root.querySelectorAll !== 'function' || typeof isVisible !== 'function') {
      return null;
    }
    for (const selector of COMPOSER_SELECTORS) {
      const match = Array.from(root.querySelectorAll(selector)).find(isVisible);
      if (match) return match;
    }
    return null;
  }

  function isPrimaryComposerTextControl(node, region, composer, describe) {
    if (region !== 'composer' || !node || !composer || typeof describe !== 'function') return false;
    const details = describe(node);
    if (!details || details.role !== 'textbox') return false;
    return node === composer || contains(node, composer) || contains(composer, node);
  }

  return Object.freeze({ findVisibleComposer, isPrimaryComposerTextControl });
});
