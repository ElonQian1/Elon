(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptAttachmentPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const ENGLISH_REMOVE = /^(?:remove|delete)\s+(?:file|attachment)(?:\s*\d+)?(?:\s*[:\-]|\s|$)/iu;
  const CHINESE_REMOVE = /^(?:移除|删除)(?:第?\s*\d+\s*个?)?(?:文件|附件)|^(?:移除|删除)(?:文件|附件)(?:\s*\d+)?(?:\s*[:：-]|\s|$)/u;
  const REMOVE_TEXT = /\b(?:remove|delete)\s+(?:file|attachment)(?:\s*\d+)?(?:\s*[:\-])?|(?:移除|删除)(?:第?\s*\d+\s*个?)?(?:文件|附件)(?:\s*\d+)?(?:\s*[:：-])?/giu;

  function clean(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isRemoveActionLabel(value) {
    const label = clean(value);
    return ENGLISH_REMOVE.test(label) || CHINESE_REMOVE.test(label);
  }

  function withoutRemoveAction(value) {
    return clean(clean(value).replace(REMOVE_TEXT, ' '));
  }

  function invokeRemoveAction(node, label) {
    if (!node || node.isConnected === false || typeof node.click !== 'function') return false;
    if (!isRemoveActionLabel(label)) return false;
    node.click();
    return true;
  }

  return Object.freeze({ isRemoveActionLabel, withoutRemoveAction, invokeRemoveAction });
});
