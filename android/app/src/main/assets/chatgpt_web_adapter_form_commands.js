(function (root, factory) {
  'use strict';

  const commands = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = commands;
  if (root) root.__elonChatGptFormCommands = Object.freeze(commands);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function inViewport(rect) {
    return rect.bottom > 0 && rect.right > 0 && rect.top < window.innerHeight && rect.left < window.innerWidth;
  }

  function setSelected(node, controlId, selected, formAdapter, emitEvent, result, emitSnapshot) {
    if (!formAdapter) return result('set_ui_control_selected', false, '官网表单适配器尚未就绪。');
    const plan = formAdapter.planSelectedState(node, selected);
    if (!plan.ok) {
      const detail = plan.reason === 'radio_cannot_clear'
        ? '单选项不能单独取消，请选择同组中的其他选项。'
        : '该官网控件不支持设置选中状态。';
      return result('set_ui_control_selected', false, detail);
    }
    if (!plan.needsActivation) {
      result('set_ui_control_selected', true, '');
      return emitSnapshot();
    }
    function dispatch() {
      if (!node.isConnected) {
        return result('set_ui_control_selected', false, '官网控件已变化，请刷新结构后重试。');
      }
      const rect = node.getBoundingClientRect();
      const xRatio = (rect.left + rect.width / 2) / Math.max(1, window.innerWidth);
      const yRatio = (rect.top + rect.height / 2) / Math.max(1, window.innerHeight);
      if (!inViewport(rect) || xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) {
        return result('set_ui_control_selected', false, '官网控件滚动后仍不在可操作区域。');
      }
      emitEvent({
        type: 'web_touch_request',
        purpose: 'invoke_ui_control',
        controlId,
        xRatio,
        yRatio
      });
      result('set_ui_control_selected', true, '');
      window.setTimeout(emitSnapshot, 180);
    }
    const rect = node.getBoundingClientRect();
    if (inViewport(rect)) return dispatch();
    node.scrollIntoView({ block: 'center', inline: 'nearest' });
    window.setTimeout(dispatch, 120);
  }

  function selectChoice(node, choiceIndex, formAdapter, result, emitSnapshot) {
    if (!formAdapter) return result('select_ui_control_choice', false, '官网表单适配器尚未就绪。');
    const update = formAdapter.selectChoice(node, choiceIndex);
    if (!update.ok) {
      const detail = update.reason === 'disabled_choice'
        ? '该官网选项当前不可用。'
        : '该官网控件不是可选择的原生列表，或选项已变化。';
      return result('select_ui_control_choice', false, detail);
    }
    result('select_ui_control_choice', true, '');
    window.setTimeout(emitSnapshot, update.changed ? 180 : 0);
  }

  return Object.freeze({ selectChoice, setSelected });
});
