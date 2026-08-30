(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateSendObserver;
  if (existing && Number(existing.version) >= 2) return;

  const delegateFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!delegateFetch) return;

  let sequence = 0;
  let latest = null;

  function requestUrl(input) {
    try {
      return new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
  }

  function isOfficialConversationSend(input, init) {
    if (init && init.__elonPrivateTransport) return false;
    const url = requestUrl(input);
    const method = String(init && init.method || input && input.method || 'GET').toUpperCase();
    return method === 'POST' && !!url && url.origin === location.origin &&
      /^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation\/?$/.test(url.pathname);
  }

  const wrappedFetch = function () {
    const args = arguments;
    const input = args[0];
    const init = args[1] || {};
    const officialSend = isOfficialConversationSend(input, init);
    let response;
    try {
      response = delegateFetch.apply(this, args);
    } catch (error) {
      throw error;
    }
    if (officialSend) {
      latest = Object.freeze({
        sequence: ++sequence,
        pagePath: String(location.pathname || '/').slice(0, 256),
        observedAt: Date.now()
      });
    }
    return response;
  };
  window.fetch = wrappedFetch;

  function marker() {
    return Object.freeze({
      sequence,
      pagePath: String(location.pathname || '/').slice(0, 256)
    });
  }

  function dispatchedAfter(value) {
    if (!value || !latest) return false;
    const before = Number(value.sequence);
    const pagePath = String(value.pagePath || '');
    return Number.isFinite(before) && latest.sequence > before &&
      (!pagePath || latest.pagePath === pagePath);
  }

  window.__elonChatGptPrivateSendObserver = Object.freeze({
    version: 2,
    marker,
    dispatchedAfter
  });
})();
