(function (root, factory) {
  'use strict';

  const adapter = factory();
  if (typeof module === 'object' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptComposerToolSelection = Object.freeze(adapter);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const MAX_OBSERVATION_ATTEMPTS = 24;
  const REQUIRED_CONFIRMATIONS = 4;
  const OBSERVATION_INTERVAL_MS = 120;
  const MAX_TOUCH_ATTEMPTS = 2;

  function completeWhenObserved(context, attempt, confirmations, touchAttempt) {
    const menuSettled = context.menuSettled();
    const optionSelection = context.directSelection(context.optionNode);
    const composerSelection = context.composerSelection();
    const observedSelection = !menuSettled && optionSelection.known
      ? optionSelection
      : composerSelection;
    const observed = observedSelection.known &&
      observedSelection.selected === context.desiredSelected;
    const nextConfirmations = observed ? confirmations + 1 : 0;
    if (nextConfirmations >= REQUIRED_CONFIRMATIONS) {
      return context.complete(true, '');
    }
    if (
      !menuSettled && optionSelection.known && !observed &&
      attempt >= REQUIRED_CONFIRMATIONS
    ) {
      if (touchAttempt >= MAX_TOUCH_ATTEMPTS || !context.retryTouch(context.optionNode)) {
        return context.complete(false, '官网网页搜索状态未发生预期变化。');
      }
      window.setTimeout(
        () => completeWhenObserved(context, 1, 0, touchAttempt + 1),
        OBSERVATION_INTERVAL_MS
      );
      return;
    }
    if (
      (menuSettled && !composerSelection.known && attempt >= REQUIRED_CONFIRMATIONS) ||
      attempt >= MAX_OBSERVATION_ATTEMPTS
    ) {
      return verifyInMenu(context, touchAttempt);
    }
    window.setTimeout(
      () => completeWhenObserved(context, attempt + 1, nextConfirmations, touchAttempt),
      OBSERVATION_INTERVAL_MS
    );
  }

  function verifyInMenu(context, touchAttempt) {
    context.openVerificationMenu((options) => {
      const target = options.find((option) => option.semantic === 'web_search');
      if (!target || !target.directStateKnown) {
        return context.complete(false, '官网没有提供可验证的网页搜索状态。');
      }
      if (target.selected === context.desiredSelected) return context.complete(true, '');
      if (touchAttempt >= MAX_TOUCH_ATTEMPTS || !context.retryTouch(target.node)) {
        return context.complete(false, '官网网页搜索状态未发生预期变化。');
      }
      const retryContext = Object.assign({}, context, {
        optionNode: target.node,
        menuSettled: () => context.menuSettledFor(target.node)
      });
      window.setTimeout(
        () => completeWhenObserved(retryContext, 1, 0, touchAttempt + 1),
        OBSERVATION_INTERVAL_MS
      );
    }, () => context.complete(false, '官网网页搜索状态无法复核。'));
  }

  function select(context) {
    completeWhenObserved(context, 1, 0, 1);
  }

  return Object.freeze({ select });
});
