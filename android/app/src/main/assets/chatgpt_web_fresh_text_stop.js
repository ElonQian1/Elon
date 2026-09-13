(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextStop = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const history = options.reconciliation || page.__elonChatGptFreshTextReconcile.create();
  async function run(owner) {
    const controller = new page.AbortController();
    let timer, listener;
    const expired = new Promise((_, reject) => {
      timer = page.setTimeout(() => { controller.abort(); reject(Error('stop_timeout')); }, options.timeoutMs || 12000);
      listener = () => { controller.abort(); reject(Error('context_changed')); };
      owner.stopBoundary?.addEventListener('abort', listener, { once: true });
    });
    const wait = promise => Promise.race([promise, expired]);
    function current() {
      return !controller.signal.aborted && owner.stopCurrent() &&
        owner.binding.canStop(owner.request.userMessageId);
    }
    function check() { if (!current()) throw Error('context_changed'); }
    try {
      check();
      const { shared } = owner.binding;
      if (typeof shared.$3 !== 'function' || typeof shared.textApi?.safePost !== 'function') throw Error('stop_unavailable');
      let payload, source;
      for (let attempt = 0; attempt < 3; attempt++) {
        check();
        payload = await wait(history.read(owner.binding, owner.request, controller.signal));
        check();
        source = history.branch(payload, owner.binding, owner.request.userMessageId);
        if (source || owner.stopAttempted || attempt === 2 ||
            history.awaitingUser?.(payload, owner.binding, owner.request.userMessageId) !== true) break;
        await wait(new Promise(resolve => page.setTimeout(resolve, 200)));
      }
      if (!source) throw Error('stop_owner_unconfirmed');
      const terminal = history.ownsResponse(payload, owner.binding, owner.request.userMessageId, true);
      if (!terminal && !owner.stopAttempted) {
        // KM's two observed gates are read, never forced. The conduit is from
        // this exact preparation, not the page's last generation or another tab.
        const enabled = shared.$3('3922476776'), excludesPro = shared.$3('877631007');
        if (enabled !== true || typeof excludesPro !== 'boolean') throw Error('stop_unavailable');
        if (shared.v7?.STREAMING !== 3 || source.asyncStatus !== null && source.asyncStatus !== shared.v7.STREAMING ||
            source.leaf.author.role !== 'user' &&
            !['in_progress', 'finished_successfully', 'finished_partial_completion'].includes(source.leaf.status)) {
          throw Error('stop_scope_unconfirmed');
        }
        const request = owner.request.consumeStop(excludesPro && !source.leaf.metadata?.chime_version ? ['pro_mode'] : [], current);
        check();
        owner.stopAttempted = true;
        await wait(shared.textApi.safePost('/stop_conversation', {
          ...request, signal: controller.signal, disableAutomaticRetry: true
        }));
        check();
        owner.stopAcknowledged = true;
        owner.stopReading();
      }
      // A response to stop is not terminal history. Reconcile a bounded number
      // of reads, preserving the native stream text until the same turn settles.
      for (let attempt = 0; attempt < 3; attempt++) {
        check();
        if (await wait(history.reconcile(owner.binding, owner.request, controller.signal, true, owner.stopAcknowledged === true))) {
          check();
          owner.stopConfirmed = true;
          owner.stopReading();
          return { status: 'accepted', code: owner.stopAttempted ? 'stopped' : 'already_stopped' };
        }
        if (attempt < 2) await wait(new Promise(resolve => page.setTimeout(resolve, 200)));
      }
      return { status: 'unknown', code: 'stop_reconciliation_pending' };
    } catch (error) {
      return { status: 'unknown', code: ['context_changed', 'stop_timeout', 'stop_unavailable',
        'stop_owner_unconfirmed', 'stop_scope_unconfirmed'].includes(error?.message) ? error.message : 'stop_unconfirmed' };
    } finally {
      page.clearTimeout(timer);
      owner.stopBoundary?.removeEventListener('abort', listener);
    }
  }
  function stop(owner) {
    if (owner.stopJob) return owner.stopJob;
    owner.stopping = true;
    owner.stopJob = run(owner).finally(() => { owner.stopping = false; owner.stopJob = null; });
    return owner.stopJob;
  }
  return Object.freeze({ stop });
});
