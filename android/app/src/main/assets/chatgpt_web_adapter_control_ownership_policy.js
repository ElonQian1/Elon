(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptControlOwnershipPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  function contains(owner, candidate) {
    return !!owner && typeof owner.contains === 'function' && owner.contains(candidate);
  }

  function isPrimaryComposerTextControl(node, region, composer, describe) {
    if (region !== 'composer' || !node || !composer || typeof describe !== 'function') return false;
    const details = describe(node);
    if (!details || details.role !== 'textbox') return false;
    return node === composer || contains(node, composer) || contains(composer, node);
  }

  return Object.freeze({ isPrimaryComposerTextControl });
});
