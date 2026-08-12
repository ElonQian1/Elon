(function (root, factory) {
  'use strict';

  const adapter = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptDisclosureControls = Object.freeze(adapter);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function attribute(node, name) {
    return node && typeof node.getAttribute === 'function'
      ? String(node.getAttribute(name) || '').toLowerCase()
      : '';
  }

  function describe(node) {
    const rawExpanded = attribute(node, 'aria-expanded');
    if (rawExpanded !== 'true' && rawExpanded !== 'false') return null;
    const disabled = !!(node && node.disabled) || attribute(node, 'aria-disabled') === 'true';
    return {
      expanded: rawExpanded === 'true',
      expandable: !disabled
    };
  }

  function inViewport(rect) {
    return rect.bottom > 0 && rect.right > 0 &&
      rect.top < window.innerHeight && rect.left < window.innerWidth;
  }

  function finishWhenObserved(node, controlId, desired, emitEvent, result, emitSnapshot, attempt, touchAttempt) {
    const current = describe(node);
    if (current && current.expanded === desired) {
      result('set_ui_control_expanded', true, '');
      return emitSnapshot();
    }
    if (!node || !node.isConnected) {
      result('set_ui_control_expanded', false, '官网控件未达到请求的展开状态。');
      return emitSnapshot();
    }
    if (attempt >= MAX_OBSERVATION_ATTEMPTS) {
      if (touchAttempt < MAX_TOUCH_ATTEMPTS) {
        return dispatchTouch(
          node, controlId, desired, emitEvent, result, emitSnapshot, touchAttempt + 1
        );
      }
      result('set_ui_control_expanded', false, '官网控件未达到请求的展开状态。');
      return emitSnapshot();
    }
    window.setTimeout(
      () => finishWhenObserved(
        node, controlId, desired, emitEvent, result, emitSnapshot, attempt + 1, touchAttempt
      ),
      OBSERVATION_INTERVAL_MS
    );
  }

  function dispatchTouch(node, controlId, desired, emitEvent, result, emitSnapshot, touchAttempt) {
    const current = describe(node);
    if (!current || !current.expandable) {
      return result('set_ui_control_expanded', false, '该官网控件不再支持设置展开状态。');
    }
    if (current.expanded === desired) {
      result('set_ui_control_expanded', true, '');
      return emitSnapshot();
    }
    const rect = node.getBoundingClientRect();
    const xRatio = (rect.left + rect.width / 2) / Math.max(1, window.innerWidth);
    const yRatio = (rect.top + rect.height / 2) / Math.max(1, window.innerHeight);
    if (!inViewport(rect) || xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) {
      return result('set_ui_control_expanded', false, '官网控件滚动后仍不在可操作区域。');
    }
    emitEvent({ type: 'web_touch_request', purpose: 'invoke_ui_control', controlId, xRatio, yRatio });
    window.setTimeout(
      () => finishWhenObserved(
        node, controlId, desired, emitEvent, result, emitSnapshot, 1, touchAttempt
      ),
      OBSERVATION_INTERVAL_MS
    );
  }

  function setExpanded(node, controlId, desiredExpanded, emitEvent, result, emitSnapshot) {
    const desired = desiredExpanded === true;
    const initial = describe(node);
    if (!initial || !initial.expandable) {
      return result('set_ui_control_expanded', false, '该官网控件不支持设置展开状态。');
    }
    if (initial.expanded === desired) {
      result('set_ui_control_expanded', true, '');
      return emitSnapshot();
    }

    function dispatch() {
      if (!node || !node.isConnected) {
        return result('set_ui_control_expanded', false, '官网控件已变化，请刷新结构后重试。');
      }
      const current = describe(node);
      if (!current || !current.expandable) {
        return result('set_ui_control_expanded', false, '该官网控件不再支持设置展开状态。');
      }
      if (current.expanded === desired) {
        result('set_ui_control_expanded', true, '');
        return emitSnapshot();
      }
      return dispatchTouch(node, controlId, desired, emitEvent, result, emitSnapshot, 1);
    }

    const rect = node.getBoundingClientRect();
    if (inViewport(rect)) return dispatch();
    node.scrollIntoView({ block: 'center', inline: 'nearest' });
    window.setTimeout(dispatch, 120);
  }

  const MAX_OBSERVATION_ATTEMPTS = 12;
  const MAX_TOUCH_ATTEMPTS = 2;
  const OBSERVATION_INTERVAL_MS = 120;
  return Object.freeze({ describe, setExpanded });
});
