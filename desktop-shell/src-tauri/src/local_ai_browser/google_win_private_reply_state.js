(function () {
  'use strict';

  var STATE_VERSION = 3;
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

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function contentText(message) {
    const content = Array.isArray(message && message.content) ? message.content : [];
    return cleanText(content.map(function (part) {
      if (typeof part === 'string') return part;
      return part && (part.type === 'text' || part.type === 'markdown') ? part.text : '';
    }).filter(Boolean).join(' '));
  }

  function richerPrivateText(current, next) {
    const existing = cleanText(current);
    const candidate = cleanText(next);
    if (!candidate || candidate === existing || candidate.length <= existing.length) return false;
    return !existing || candidate.includes(existing) ||
      (existing.length <= 120 && candidate.length >= existing.length + 24);
  }

  function upgradedContent(content, value) {
    const source = Array.isArray(content) ? content : [];
    const next = [];
    let inserted = false;
    source.forEach(function (part) {
      const textual = typeof part === 'string' ||
        part && (part.type === 'text' || part.type === 'markdown');
      if (!textual) {
        next.push(part);
      } else if (!inserted) {
        const type = part && typeof part === 'object' && part.type === 'markdown'
          ? 'markdown'
          : 'text';
        next.push({ type: type, text: value });
        inserted = true;
      }
    });
    if (!inserted) next.unshift({ type: 'text', text: value });
    return next;
  }

  function applyPrivateReply(event, value) {
    const messages = Array.isArray(event && event.messages) ? event.messages : null;
    const prompt = cleanText(value && value.prompt);
    const answer = cleanText(value && value.text);
    if (!messages || !prompt || !answer) return false;
    let userIndex = -1;
    for (let index = messages.length - 1; index >= 0; index -= 1) {
      if (messages[index] && messages[index].role === 'user' &&
          contentText(messages[index]) === prompt) {
        userIndex = index;
        break;
      }
    }
    if (userIndex < 0) return false;
    let assistantIndex = -1;
    for (let index = messages.length - 1; index > userIndex; index -= 1) {
      if (messages[index] && messages[index].role === 'assistant') {
        assistantIndex = index;
        break;
      }
    }
    if (assistantIndex < 0) {
      messages.splice(userIndex + 1, 0, {
        id: 'google-private-answer-win-' + generation + '-' + revision,
        role: 'assistant',
        state: value.streaming ? 'streaming' : 'completed',
        content: [{ type: 'text', text: answer }],
      });
      return true;
    }
    const assistant = messages[assistantIndex];
    if (!richerPrivateText(contentText(assistant), answer)) return false;
    messages[assistantIndex] = Object.assign({}, assistant, {
      state: value.streaming ? 'streaming' : 'completed',
      content: upgradedContent(assistant.content, answer),
    });
    return true;
  }

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
        const privateReply = snapshot();
        payload.event.privateStreamObserved = observed;
        payload.event.privateStreamRevision = revision;
        payload.event.privateStreamState = state;
        payload.event.privateStreamContentApplied = applyPrivateReply(payload.event, privateReply);
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
      if (typeof observerDelegate.setListener === 'function') {
        try { observerDelegate.setListener(null); } catch (_) {}
      }
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
