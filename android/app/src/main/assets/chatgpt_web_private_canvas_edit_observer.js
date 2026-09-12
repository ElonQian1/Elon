(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasEditObserver = api;
})(typeof window === 'object' ? window : null, function (page, runtime, conversation, client) {
  'use strict';
  const fail = code => { throw Error('canvas_' + code); };
  if (typeof runtime?.reactApi !== 'function' || typeof runtime.reactDom !== 'function' ||
      typeof runtime.reactRoot !== 'function' || typeof conversation?.canvasDirtyInit !== 'function' ||
      typeof client?.getMutationCache !== 'function') fail('runtime_unavailable');
  const react = runtime.reactApi(), dom = runtime.reactDom(), renderer = runtime.reactRoot();
  conversation.canvasDirtyInit();
  if (typeof react?.createElement !== 'function' || typeof react.useLayoutEffect !== 'function' ||
      typeof dom?.flushSync !== 'function' || typeof renderer?.createRoot !== 'function' ||
      typeof conversation.useCanvasDirty !== 'function') fail('runtime_unavailable');
  const cache = client.getMutationCache(), failures = new Map();
  let uncertain = false, disposed = false;
  if (typeof cache?.getAll !== 'function' || typeof cache.subscribe !== 'function') fail('runtime_unavailable');
  const isSave = mutation => {
    const key = mutation?.options?.mutationKey;
    return Array.isArray(key) && key.length === 3 && key[0] === 'canvas' && key[1] === 'textdoc' && key[2] === 'persist';
  };
  function remember(mutation) {
    if (!isSave(mutation)) return;
    const state = mutation.state, id = state?.variables?.textdocId, base = state?.variables?.lastVersion;
    if (typeof id !== 'string') { if (state?.status !== 'idle') uncertain = true; return; }
    if (state.status === 'error') {
      if (!Number.isSafeInteger(base) || !Number.isFinite(state.submittedAt)) { uncertain = true; return; }
      if (!failures.has(id) && failures.size >= 256) { uncertain = true; return; }
      const previous = failures.get(id);
      failures.set(id, { base: Math.max(previous?.base ?? -1, base), at: Math.max(previous?.at ?? 0, state.submittedAt) });
    }
    const failed = failures.get(id);
    if (failed && state.status === 'success' && Number.isSafeInteger(base) && base >= failed.base &&
        Number.isFinite(state.submittedAt) && state.submittedAt >= failed.at &&
        Number.isSafeInteger(state.data) && state.data > failed.base) failures.delete(id);
  }
  const existing = cache.getAll();
  if (!Array.isArray(existing)) fail('runtime_unavailable');
  existing.filter(value => value?.state?.status === 'error').forEach(remember);
  existing.filter(value => value?.state?.status === 'success').forEach(remember);
  // Keep failure metadata after TanStack GC; removal alone is not a successful save.
  const unsubscribe = cache.subscribe(event => { if (event?.type !== 'removed') remember(event?.mutation); });
  if (typeof unsubscribe !== 'function') fail('runtime_unavailable');
  function dispose() { if (!disposed) { disposed = true; unsubscribe(); page.removeEventListener?.('pagehide', onPageHide); } }
  function onPageHide(event) { if (!event?.persisted) dispose(); }
  page.addEventListener?.('pagehide', onPageHide);

  function dirty(id) {
    // Use the actual hook in a real React root, never a forged dispatcher or a hook call
    // outside React. The detached root renders nothing and unsubscribes in the same task.
    const host = page.document.createElement('div');
    let value, failed = false, root;
    function Snapshot() {
      const pending = conversation.useCanvasDirty(id);
      react.useLayoutEffect(() => { value = pending; });
      return null;
    }
    try {
      root = renderer.createRoot(host, { onUncaughtError: () => { failed = true; },
        onCaughtError: () => { failed = true; }, onRecoverableError: () => { failed = true; } });
      dom.flushSync(() => root.render(react.createElement(Snapshot)));
    } catch (_) { failed = true; }
    finally {
      try { if (root) dom.flushSync(() => root.unmount()); }
      catch (_) { failed = true; }
    }
    if (failed || typeof value !== 'boolean') fail('runtime_unavailable');
    return value;
  }

  function check(id, conversationId) {
    if (disposed || uncertain) fail('runtime_unavailable');
    if (dirty(id)) fail('web_edit_pending');
    const all = cache.getAll();
    if (!Array.isArray(all)) fail('runtime_unavailable');
    const mutations = [];
    for (const mutation of all) {
      const key = mutation?.options?.mutationKey, state = mutation?.state;
      if (Array.isArray(key) && key.length === 2 && key[0] === conversationId && key[1] === 'textdocs' &&
          state?.variables?.textdocId === id && Object.prototype.hasOwnProperty.call(state.variables, 'newTitle')) {
        if (state.status === 'pending') fail('web_edit_pending');
        if (!['idle', 'success', 'error'].includes(state.status)) fail('runtime_unavailable');
      }
      if (!isSave(mutation)) continue;
      if (state?.status === 'idle') continue;
      if (typeof state?.variables?.textdocId !== 'string') fail('runtime_unavailable');
      if (state.variables.textdocId !== id) continue;
      if (!['success', 'error', 'pending'].includes(state.status)) fail('runtime_unavailable');
      if (state.status === 'pending') fail('web_edit_pending');
      mutations.push(mutation);
    }
    // A failed website save is not clean merely because its debounce has fired.
    // Only a later successfully persisted version can supersede that failure.
    mutations.filter(value => value.state.status === 'error').forEach(remember);
    mutations.filter(value => value.state.status === 'success').forEach(remember);
    if (uncertain) fail('runtime_unavailable');
    if (failures.has(id)) fail('web_edit_pending');
  }

  return Object.freeze({ check, dispose });
});
