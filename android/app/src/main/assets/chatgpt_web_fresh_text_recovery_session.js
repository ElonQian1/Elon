(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextRecoverySession = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const now = options.now || Date.now;
  let scope = null, task = null, timer = null, notify = null, disposed = false;
  let attempts = 0, completed = false, phase = 'idle', nextAt = 0;
  const enabled = () => !disposed && options.enabled() === true;
  const paused = () => page.document.visibilityState === 'hidden' || page.navigator?.onLine === false;
  function key() {
    try { return [page.document, page.__elonChatGptDocumentToken, page.location.href, options.stamp()]; }
    catch (_) { return [page.document, page.__elonChatGptDocumentToken, page.location.href, null]; }
  }
  const equal = (left, right) => !!left && left.every((value, index) => value === right[index]);
  function changed() { try { notify?.(); } catch (_) {} }
  function clearTimer() { if (timer !== null) page.clearTimeout(timer); timer = null; }
  function cancel() {
    clearTimer();
    const previous = task;
    task = null;
    previous?.controller.abort();
    attempts = 0; completed = false; nextAt = 0; phase = 'idle';
  }
  function schedule(delay) {
    clearTimer();
    timer = page.setTimeout(() => { timer = null; snapshot(); }, delay);
  }
  function pause() {
    const uncertain = phase === 'checking' || phase === 'unconfirmed' || phase === 'unavailable';
    const retryAt = nextAt;
    cancel(); nextAt = retryAt; phase = uncertain ? 'unconfirmed' : 'idle';
  }
  function snapshot(onChange) {
    if (onChange) notify = onChange;
    if (!enabled()) { cancel(); scope = null; return 'disabled'; }
    const incoming = key();
    if (!equal(scope, incoming)) { cancel(); scope = incoming; }
    if (options.hasWriter()) { cancel(); return 'idle'; }
    if (paused()) {
      pause();
      return phase;
    }
    if (!task && !completed && attempts < 3 && now() >= nextAt) start();
    return phase;
  }
  function start() {
    const owner = { scope, controller: new page.AbortController(), pending: false };
    task = owner; attempts++; nextAt = now() + 15000;
    const current = () => task === owner && enabled() && !paused() && !options.hasWriter() &&
      !owner.controller.signal.aborted && equal(owner.scope, key());
    let timeout, abort;
    const stopped = new Promise((_, reject) => {
      abort = () => reject(Error('context_changed'));
      owner.controller.signal.addEventListener('abort', abort, { once: true });
      timeout = page.setTimeout(() => reject(Error('recovery_history_timeout')), options.timeoutMs ?? 15000);
    });
    const work = Promise.resolve().then(async () => {
      const binding = await options.context.capture(owner.controller.signal);
      if (!current()) throw Error('context_changed');
      if (!binding) return 0;
      const guarded = { ...binding, owns: () => current() && binding.owns() === true };
      return options.journal().recoverSelected(guarded, owner.controller.signal, count => {
        if (count > 0 && current()) { owner.pending = true; phase = 'checking'; changed(); }
      });
    });
    Promise.race([work, stopped]).then(count => {
      if (!current()) return;
      completed = true; phase = count > 0 ? 'recovered' : 'clear';
    }).catch(error => {
      if (!current()) return;
      const cold = ['runtime_unavailable', 'recovery_identity_unavailable', 'context_unavailable',
        'context_changed'].includes(error?.message);
      phase = owner.pending ? 'unconfirmed' : cold ? 'idle' : 'unavailable';
      // Late runtime mounting gets two bounded retries. Network/history failures
      // wait for an actual resume/online event, not every DOM snapshot.
      if (cold && !owner.pending && attempts < 3) {
        nextAt = now() + (attempts === 1 ? 1000 : 5000);
        schedule(nextAt - now());
      } else { completed = true; nextAt = now() + 15000; }
    }).finally(() => {
      page.clearTimeout(timeout);
      owner.controller.signal.removeEventListener('abort', abort);
      if (task !== owner) return;
      task = null; owner.controller.abort(); changed();
    });
  }
  function suspend() {
    pause(); changed();
  }
  function resume() {
    if (!enabled() || paused() || task) return;
    completed = false; attempts = 0;
    if (nextAt > now()) schedule(nextAt - now());
    else snapshot();
  }
  const visibility = () => paused() ? suspend() : resume();
  const document = page.document;
  document.addEventListener?.('visibilitychange', visibility);
  page.addEventListener?.('offline', suspend);
  page.addEventListener?.('online', resume);
  page.addEventListener?.('pageshow', resume);
  function dispose() {
    disposed = true; cancel(); notify = null;
    document.removeEventListener?.('visibilitychange', visibility);
    page.removeEventListener?.('offline', suspend);
    page.removeEventListener?.('online', resume);
    page.removeEventListener?.('pageshow', resume);
  }
  return Object.freeze({ snapshot, cancel, dispose });
});
