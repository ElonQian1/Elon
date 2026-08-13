(function (root, factory) {
  'use strict';

  const adapter = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptTemporaryChat = Object.freeze(adapter);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function describe(pageSemanticPolicy, input) {
    if (!pageSemanticPolicy || typeof pageSemanticPolicy.temporaryChatState !== 'function') {
      return null;
    }
    return pageSemanticPolicy.temporaryChatState(input);
  }

  function setSelected(input) {
    const values = input || {};
    const control = values.control;
    if (!control || control.semantic !== 'temporary_chat') return false;
    const policy = values.pageSemanticPolicy;
    const plan = policy && typeof policy.planTemporaryChatSelection === 'function'
      ? policy.planTemporaryChatSelection(control.selected, values.desiredSelected)
      : { ok: false, needsActivation: false };
    if (!plan.ok) {
      values.result('set_ui_control_selected', false, '临时聊天状态暂时不可用，请刷新后重试。');
      return true;
    }
    if (!plan.needsActivation) {
      values.result('set_ui_control_selected', true, '');
      values.emitSnapshot();
      return true;
    }

    const node = values.node;
    function dispatch() {
      if (!node.isConnected || !values.isVisible(node)) {
        values.result('set_ui_control_selected', false, '官网控件已变化，请刷新结构后重试。');
        return;
      }
      const rect = node.getBoundingClientRect();
      const xRatio = (rect.left + rect.width / 2) / Math.max(1, window.innerWidth);
      const yRatio = (rect.top + rect.height / 2) / Math.max(1, window.innerHeight);
      if (!values.isInViewport(rect) || xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) {
        values.result('set_ui_control_selected', false, '官网控件滚动后仍不在可操作区域。');
        return;
      }
      values.emitEvent({
        type: 'web_touch_request',
        purpose: 'invoke_ui_control',
        controlId: values.controlId,
        xRatio,
        yRatio
      });
      values.result('set_ui_control_selected', true, '');
      window.setTimeout(values.emitSnapshot, 180);
    }

    const rect = node.getBoundingClientRect();
    if (values.isInViewport(rect)) dispatch();
    else {
      node.scrollIntoView({ block: 'center', inline: 'nearest' });
      window.setTimeout(dispatch, 120);
    }
    return true;
  }

  return Object.freeze({ describe, setSelected });
});
