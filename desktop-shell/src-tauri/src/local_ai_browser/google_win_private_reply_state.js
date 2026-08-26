(function () {
  'use strict';

  var STATE_VERSION = 2;
  const baseObserver = window.__elonGoogleWebPrivateReplyObserver;
  const baseNative = window.elonGoogleWebNative;
  if (!baseObserver || typeof baseObserver.observePrompt !== 'function' ||
      typeof baseObserver.snapshot !== 'function' || !baseNative ||
      typeof baseNative.postMessage !== 'function') return;

  const existingState = window.__elonWinGooglePrivateReplyState;
  if (existingState && Number(existingState.version || 0) >= STATE_VERSION &&
      typeof existingState.rebind === 'function') {
    existingState.rebind(baseObserver, baseNative);
    return;
  }

  let generation = 0;
  let revision = 0;
  let lastFingerprint = '';
  let observed = false;
  let state = 'idle';
  let acceptingReplies = false;
  let observerDelegate = baseObserver;
  let nativeDelegate = baseNative;
  let observerListener = null;
  let bindingRevision = 1;
  let observer;
  let nativeProxy;

  function reset() {
    generation += 1;
    lastFingerprint = '';
    observed = false;
    state = 'idle';
    acceptingReplies = false;
  }

  function observePrompt(value) {
    reset();
    acceptingReplies = true;
    return observerDelegate.observePrompt(value);
  }

  function snapshot() {
    if (!acceptingReplies) return null;
    const value = observerDelegate.snapshot();
    if (!value || !value.prompt || !value.text) return value;
    const nextState = value.streaming ? 'streaming' : 'completed';
    const fingerprint = [generation, nextState, String(value.text)].join('|');
    if (fingerprint !== lastFingerprint) {
      lastFingerprint = fingerprint;
      revision += 1;
    }
    observed = true;
    state = nextState;
    return value;
  }

  function setListener(value) {
    observerListener = typeof value === 'function' ? value : null;
    if (typeof observerDelegate.setListener === 'function') {
      return observerDelegate.setListener(observerListener);
    }
  }

  observer = {
    observePrompt,
    snapshot,
    diagnostics: function () {
      return typeof observerDelegate.diagnostics === 'function'
        ? observerDelegate.diagnostics()
        : '';
    },
    setListener,
  };
  Object.defineProperty(observer, 'version', {
    enumerable: true,
    get: function () { return Number(observerDelegate.version || 0); },
  });
  observer = Object.freeze(observer);

  nativeProxy = Object.freeze({
    __elonWinGooglePrivateReplyStateWrapped: true,
    postMessage: function (raw) {
      let payload;
      try { payload = JSON.parse(String(raw || '')); }
      catch (_) { return nativeDelegate.postMessage(raw); }
      if (payload && payload.schema === 'yilong.ai.ui.v1' && payload.event &&
          payload.event.type === 'message_snapshot') {
        snapshot();
        payload.event.privateStreamObserved = observed;
        payload.event.privateStreamRevision = revision;
        payload.event.privateStreamState = state;
        if (observed && state === 'streaming') payload.event.streaming = true;
        if (observed && state === 'completed') payload.event.streaming = false;
        return nativeDelegate.postMessage(JSON.stringify(payload));
      }
      return nativeDelegate.postMessage(raw);
    },
  });

  function rebind(nextObserver, nextNative) {
    let changed = false;
    if (nextObserver && nextObserver !== observer && nextObserver !== observerDelegate &&
        typeof nextObserver.observePrompt === 'function' &&
        typeof nextObserver.snapshot === 'function') {
      observerDelegate = nextObserver;
      if (typeof observerDelegate.setListener === 'function') {
        observerDelegate.setListener(observerListener);
      }
      reset();
      changed = true;
    }
    if (nextNative && nextNative !== nativeProxy && nextNative !== nativeDelegate &&
        typeof nextNative.postMessage === 'function') {
      nativeDelegate = nextNative;
      changed = true;
    }
    if (changed) bindingRevision += 1;
    window.__elonGoogleWebPrivateReplyObserver = observer;
    window.elonGoogleWebNative = nativeProxy;
    return changed;
  }

  window.__elonGoogleWebPrivateReplyObserver = observer;
  window.elonGoogleWebNative = nativeProxy;
  window.__elonWinGooglePrivateReplyStateVersion = STATE_VERSION;
  window.__elonWinGooglePrivateReplyState = Object.freeze({
    version: STATE_VERSION,
    reset,
    rebind,
    snapshot: function () {
      return Object.freeze({ generation, revision, observed, state, bindingRevision });
    },
    diagnostics: function () {
      return 'v' + STATE_VERSION + '|bindings=' + bindingRevision + '|state=' + state;
    },
  });
})();
