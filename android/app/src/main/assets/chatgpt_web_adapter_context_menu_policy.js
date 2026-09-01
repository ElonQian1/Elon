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
    if (!(shouldArm(control) || isProjectMoveStep(control)) ||
        !node || typeof node.click !== 'function') return false;
    try {
      node.click();
      return true;
    } catch {
      return false;
    }
  }

  return Object.freeze({ activate, isProjectMoveStep, shouldArm });
});
