(function () {
  'use strict';

  if (Number(window.__elonWinGooglePrivateReplyStateVersion || 0) >= 1) return;
  const baseObserver = window.__elonGoogleWebPrivateReplyObserver;
  const baseNative = window.elonGoogleWebNative;
  if (!baseObserver || typeof baseObserver.observePrompt !== 'function' ||
      typeof baseObserver.snapshot !== 'function' || !baseNative ||
      typeof baseNative.postMessage !== 'function') return;

  let generation = 0;
  let revision = 0;
  let lastFingerprint = '';
  let observed = false;
  let state = 'idle';
  let acceptingReplies = false;

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
    return baseObserver.observePrompt(value);
  }

  function snapshot() {
    if (!acceptingReplies) return null;
    const value = baseObserver.snapshot();
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

  const observer = Object.freeze({
    version: Number(baseObserver.version || 0),
    observePrompt,
    snapshot,
    diagnostics: typeof baseObserver.diagnostics === 'function'
      ? function () { return baseObserver.diagnostics(); }
      : function () { return ''; },
    setListener: typeof baseObserver.setListener === 'function'
      ? function (value) { return baseObserver.setListener(value); }
      : function () {},
  });
  window.__elonGoogleWebPrivateReplyObserver = observer;

  window.elonGoogleWebNative = Object.freeze({
    postMessage: function (raw) {
      let payload;
      try { payload = JSON.parse(String(raw || '')); }
      catch (_) { return baseNative.postMessage(raw); }
      if (payload && payload.schema === 'yilong.ai.ui.v1' && payload.event &&
          payload.event.type === 'message_snapshot') {
        snapshot();
        payload.event.privateStreamObserved = observed;
        payload.event.privateStreamRevision = revision;
        payload.event.privateStreamState = state;
        if (observed && state === 'streaming') payload.event.streaming = true;
        if (observed && state === 'completed') payload.event.streaming = false;
        return baseNative.postMessage(JSON.stringify(payload));
      }
      return baseNative.postMessage(raw);
    },
  });

  window.__elonWinGooglePrivateReplyStateVersion = 1;
  window.__elonWinGooglePrivateReplyState = Object.freeze({
    reset,
    snapshot: function () {
      return Object.freeze({ generation, revision, observed, state });
    },
  });
})();
