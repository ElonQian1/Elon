(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptContextMenuPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function shouldArm(control) {
    return !!control && control.semantic === 'conversation_options' && !!control.contextId;
  }

  function requiresNativeTouch(control) {
    return shouldArm(control);
  }

  function canUseAfterTouchMiss(control, currentConversationId, overlayCount) {
    return shouldArm(control) && control.region === 'header' &&
      control.contextId === String(currentConversationId || '') && Number(overlayCount) === 0;
  }

  function isProjectMoveStep(control) {
    if (!control || control.region !== 'overlay' || control.enabled === false) return false;
    if (control.semantic === 'save_to_project') return !!control.contextId;
    if (control.semantic === 'confirm' && ['button', 'menuitem'].includes(control.role)) return true;
    if (
      ['button', 'menuitem'].includes(control.role) &&
      /^(?:confirm|move(?: chat| conversation)?|save|add|done|ok|确定|确认|移动(?:会话|聊天)?|保存|添加|完成)$/i
        .test(String(control.label || '').trim())
    ) return true;
    return control.semantic === 'project' &&
      ['button', 'menuitem', 'menuitemradio', 'option', 'radio'].includes(control.role);
  }

  function activate(control, node) {
    // React's conversation trigger ignores synthetic click on some mobile builds.
    // Let the Android touch bridge open it after ownership has been armed.
    if (requiresNativeTouch(control) || !isProjectMoveStep(control) ||
        !node || typeof node.click !== 'function') return false;
    try {
      node.click();
      return true;
    } catch {
      return false;
    }
  }

  function activateAfterTouchMiss(control, node, currentConversationId, overlayCount) {
    if (!canUseAfterTouchMiss(control, currentConversationId, overlayCount) ||
        !node) return false;
    try {
      const view = node.ownerDocument && node.ownerDocument.defaultView;
      if (view && typeof view.PointerEvent === 'function' &&
          typeof node.dispatchEvent === 'function') {
        const common = {
          bubbles: true,
          cancelable: true,
          composed: true,
          pointerId: 1,
          pointerType: 'touch',
          isPrimary: true,
          button: 0,
          ctrlKey: false
        };
        node.dispatchEvent(new view.PointerEvent('pointerdown', Object.assign({ buttons: 1 }, common)));
        node.dispatchEvent(new view.PointerEvent('pointerup', Object.assign({ buttons: 0 }, common)));
        return true;
      }
      if (typeof node.click !== 'function') return false;
      node.click();
      return true;
    } catch {
      return false;
    }
  }

  return Object.freeze({
    activate,
    activateAfterTouchMiss,
    canUseAfterTouchMiss,
    isProjectMoveStep,
    requiresNativeTouch,
    shouldArm
  });
});
