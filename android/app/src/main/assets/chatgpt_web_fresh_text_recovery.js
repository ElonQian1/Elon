(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextRecovery = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const history = options.reconciliation || page.__elonChatGptFreshTextReconcile.create();
  const now = options.now || Date.now;
  const delays = options.delays || [0, 400, 1200];
  const unknown = code => ({ status: 'unknown', code });
  const foregroundOnline = () => page.document.visibilityState !== 'hidden' && page.navigator?.onLine !== false;

  function cancelScheduled(owner) {
    if (!owner) return;
    owner.recoveryResumeRequested = false;
    owner.recoveryWakeup?.cancel();
  }

  function requestAutomatic(owner) {
    if (!foregroundOnline() || !owner.dispatched || !owner.finished || owner.stopping ||
        owner.stopConfirmed || owner.recoveryConfirmed || (owner.autoRecoveryCount || 0) >= 3) return;
    if (owner.recoveryJob) owner.recoveryResumeRequested = true;
    else scheduleAutomatic(owner);
  }

  function scheduleAutomatic(owner) {
    if (owner.recoveryWakeup || owner.stopBoundary?.aborted || typeof options.onAutomaticReady !== 'function') return;
    let timer;
    function cancel() {
      page.clearTimeout(timer);
      owner.stopBoundary?.removeEventListener('abort', cancel);
      if (owner.recoveryWakeup?.cancel === cancel) owner.recoveryWakeup = null;
    }
    owner.recoveryWakeup = { cancel };
    owner.stopBoundary?.addEventListener('abort', cancel, { once: true });
    // Retain one observed resume event through cooldown, not a polling loop.
    timer = page.setTimeout(() => {
      cancel();
      try {
        if (foregroundOnline() && owner.dispatched && owner.finished && !owner.stopping &&
            !owner.stopConfirmed && !owner.recoveryConfirmed && !owner.stopBoundary?.aborted &&
            (owner.autoRecoveryCount || 0) < 3 && owner.stopCurrent() &&
            owner.binding.canReconcile(owner.request.userMessageId)) options.onAutomaticReady();
      } catch (_) {}
    }, Math.max(1, owner.nextAutoRecoveryAt - now()));
  }

  function recover(owner, automatic = false) {
    if (owner.recoveryJob) {
      if (automatic) requestAutomatic(owner);
      return owner.recoveryJob;
    }
    if (!owner.dispatched || !owner.finished || owner.stopping) return Promise.resolve(unknown('recovery_not_ready'));
    if (automatic && (!foregroundOnline() || (owner.autoRecoveryCount || 0) >= 3)) {
      cancelScheduled(owner);
      return Promise.resolve(unknown('recovery_deferred'));
    }
    try {
      if (!owner.stopCurrent() || !owner.binding.canReconcile(owner.request.userMessageId)) {
        cancelScheduled(owner);
        return Promise.resolve(unknown('context_changed'));
      }
    } catch (_) { cancelScheduled(owner); return Promise.resolve(unknown('context_changed')); }
    if (automatic && now() < (owner.nextAutoRecoveryAt || 0)) {
      scheduleAutomatic(owner);
      return Promise.resolve(unknown('recovery_deferred'));
    }
    cancelScheduled(owner);
    if (automatic) owner.autoRecoveryCount = (owner.autoRecoveryCount || 0) + 1;
    owner.nextAutoRecoveryAt = now() + (options.cooldownMs ?? 10000);
    owner.recovering = true;
    owner.recoveryJob = run(owner).finally(() => {
      owner.recovering = false;
      owner.recoveryJob = null;
      owner.recoveryController = null;
      if (owner.recoveryResumeRequested) {
        owner.recoveryResumeRequested = false;
        requestAutomatic(owner);
      }
    });
    return owner.recoveryJob;
  }

  async function run(owner) {
    const controller = new page.AbortController();
    owner.recoveryController = controller;
    let deadline, pause, rejectAbort;
    const cancelled = new Promise((_, reject) => { rejectAbort = reject; });
    function abort(code) { rejectAbort(Error(code)); controller.abort(); }
    const boundary = () => abort('context_changed');
    const aborted = () => rejectAbort(Error('recovery_cancelled'));
    owner.stopBoundary?.addEventListener('abort', boundary, { once: true });
    controller.signal.addEventListener('abort', aborted, { once: true });
    deadline = page.setTimeout(() => abort('reconciliation_timeout'), options.timeoutMs || 15000);
    const wait = value => Promise.race([value, cancelled]);
    function current() {
      return !controller.signal.aborted && !owner.stopping && owner.stopCurrent() &&
        owner.binding.canReconcile(owner.request.userMessageId);
    }
    try {
      for (const delay of delays) {
        if (!current()) return unknown('context_changed');
        if (delay) await wait(new Promise(resolve => { pause = page.setTimeout(resolve, delay); }));
        if (!current()) return unknown('context_changed');
        // Re-read the same submitted message, never send or recreate it. Stop
        // delivery uncertainty only permits a partial result with terminal history.
        const stopped = owner.stopAttempted === true;
        const done = await wait(history.reconcile(owner.binding, owner.request, controller.signal,
          stopped, stopped && owner.stopAcknowledged === true, code => { owner.historyCode = code; }));
        if (!current()) return unknown('context_changed');
        if (done) {
          owner.recoveryConfirmed = true;
          return { status: 'accepted', code: 'history_reconciled' };
        }
      }
      return unknown('history_reconciliation_pending');
    } catch (error) {
      return unknown(['context_changed', 'reconciliation_timeout', 'recovery_cancelled'].includes(error?.message)
        ? error.message : 'history_unavailable');
    } finally {
      page.clearTimeout(deadline);
      page.clearTimeout(pause);
      owner.stopBoundary?.removeEventListener('abort', boundary);
      controller.signal.removeEventListener('abort', aborted);
      // Even a runtime helper ignoring cancellation cannot apply a late response.
      controller.abort();
    }
  }
  return Object.freeze({ recover, requestAutomatic, cancelScheduled });
});
