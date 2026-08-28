(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (!root || root.location && root.location.origin !== 'https://chatgpt.com') return;
  const installed = root.__elonWinChatGptPrivateGuestConversationTransport;
  if (installed && Number(installed.version || 0) >= api.version) return;
  const runtime = api.install(root);
  if (!runtime) return;
  root.__elonWinChatGptPrivateGuestConversationTransport = Object.freeze(runtime);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const VERSION = 1;
  const MAX_HEADER_NAMES = 64;
  const MAX_HEADER_VALUE_LENGTH = 16384;
  const API_DETAIL = /^\/backend-api\/conversations\/([A-Za-z0-9_-]{1,160})$/;
  const API_STREAM = /^\/backend-api\/(?:f\/)?conversation(?:\/|$)/;
  const GUEST_DETAIL = /^\/backend-anon\/(f\/)?(conversation|conversations)\/([A-Za-z0-9_-]{1,160})$/;
  const GUEST_STREAM = /^\/backend-anon\/(f\/)?conversation(?:\/|$)/;

  function requestUrl(rootValue, input) {
    try {
      return new rootValue.URL(
        typeof input === 'string' ? input : input && input.url,
        rootValue.location.href,
      );
    } catch (_) {
      return null;
    }
  }

  function requestMethod(input, init) {
    return String(init && init.method || input && input.method || 'GET').toUpperCase();
  }

  function readHeaders(input, init) {
    const values = {};
    function read(headers) {
      if (!headers || Object.keys(values).length >= MAX_HEADER_NAMES) return;
      try {
        if (typeof headers.forEach === 'function') {
          headers.forEach(function (value, name) {
            if (Object.keys(values).length >= MAX_HEADER_NAMES) return;
            values[String(name)] = String(value).slice(0, MAX_HEADER_VALUE_LENGTH);
          });
        } else if (Array.isArray(headers)) {
          headers.slice(0, MAX_HEADER_NAMES).forEach(function (entry) {
            if (!Array.isArray(entry) || entry.length < 2) return;
            values[String(entry[0])] = String(entry[1]).slice(0, MAX_HEADER_VALUE_LENGTH);
          });
        } else if (typeof headers === 'object') {
          Object.keys(headers).slice(0, MAX_HEADER_NAMES).forEach(function (name) {
            values[String(name)] = String(headers[name]).slice(0, MAX_HEADER_VALUE_LENGTH);
          });
        }
      } catch (_) {}
    }
    read(input && input.headers);
    read(init && init.headers);
    return values;
  }

  function hasKeys(value) {
    return Boolean(value && typeof value === 'object' && Object.keys(value).length);
  }

  function cloneHeaders(value) {
    return hasKeys(value) ? Object.assign({}, value) : {};
  }

  function install(rootValue) {
    if (!rootValue || typeof rootValue.fetch !== 'function' ||
        !rootValue.location || rootValue.location.origin !== 'https://chatgpt.com') return null;
    const delegateFetch = rootValue.fetch.bind(rootValue);
    const baseProbe = rootValue.__elonChatGptPrivateResearchProbe || null;
    let mode = 'unknown';
    let guestPrefix = '';
    let exactTemplate = '';
    let guestHeaders = {};
    let rewrittenRequests = 0;
    let fallbackRequests = 0;
    let lastStatus = 0;

    function rememberOfficialRequest(input, init, url, method) {
      if (!url || url.origin !== rootValue.location.origin ||
          init && init.__elonPrivateTransport) return;
      if (API_DETAIL.test(url.pathname) || method === 'POST' && API_STREAM.test(url.pathname)) {
        mode = 'api';
        guestPrefix = '';
        exactTemplate = '';
        guestHeaders = {};
        return;
      }
      const detail = method === 'GET' ? url.pathname.match(GUEST_DETAIL) : null;
      const stream = method === 'POST' ? url.pathname.match(GUEST_STREAM) : null;
      if (!detail && !stream) return;
      mode = 'guest';
      guestPrefix = detail ? String(detail[1] || '') : String(stream[1] || '');
      if (detail) {
        exactTemplate = '/backend-anon/' + guestPrefix + detail[2] + '/{id}';
      } else {
        exactTemplate = '';
      }
      const observedHeaders = readHeaders(input, init);
      guestHeaders = Object.assign({ Accept: 'application/json' }, observedHeaders);
    }

    function guestCandidates(conversationId) {
      const id = encodeURIComponent(conversationId);
      const values = [];
      function add(template) {
        if (!template) return;
        const path = template.replace('{id}', id);
        if (!values.includes(path)) values.push(path);
      }
      add(exactTemplate);
      add('/backend-anon/' + guestPrefix + 'conversation/{id}');
      add('/backend-anon/' + guestPrefix + 'conversations/{id}');
      return values;
    }

    async function fetchGuestConversation(input, init, conversationId) {
      const candidates = guestCandidates(conversationId);
      let lastResponse = null;
      for (let index = 0; index < candidates.length; index += 1) {
        try {
          const response = await delegateFetch(candidates[index], Object.assign({}, init, {
            // Never carry a previously observed signed-in request context into an
            // anonymous endpoint. The current guest request is the only authority.
            headers: Object.assign({}, guestHeaders, { Accept: 'application/json' }),
            credentials: 'include',
            __elonPrivateTransport: 'conversation_prefetch',
            __elonWinGuestConversationRefresh: true,
          }));
          lastResponse = response;
          lastStatus = Number(response && response.status || 0);
          if (response && response.ok) return response;
          if (!response || ![404, 405].includes(lastStatus) || index === candidates.length - 1) {
            return response;
          }
          fallbackRequests += 1;
        } catch (error) {
          lastStatus = 0;
          throw error;
        }
      }
      if (lastResponse) return lastResponse;
      throw new Error('guest_conversation_refresh_failed');
    }

    function wrappedFetch(input, init) {
      const options = init || {};
      const url = requestUrl(rootValue, input);
      const method = requestMethod(input, options);
      rememberOfficialRequest(input, options, url, method);
      const privateMatch = method === 'GET' && url && url.origin === rootValue.location.origin &&
        options.__elonPrivateTransport === 'conversation_prefetch'
        ? url.pathname.match(API_DETAIL)
        : null;
      if (mode !== 'guest' || !privateMatch || !hasKeys(guestHeaders)) {
        return delegateFetch(input, options);
      }
      rewrittenRequests += 1;
      return fetchGuestConversation(input, options, privateMatch[1]);
    }

    Object.defineProperty(wrappedFetch, '__elonWinGuestConversationTransportWrapped', {
      configurable: false,
      enumerable: false,
      value: true,
    });
    rootValue.fetch = wrappedFetch;

    const probe = Object.freeze(Object.assign({}, baseProbe || {}, {
      copyRequestContext: function (family) {
        let inherited = {};
        if (baseProbe && typeof baseProbe.copyRequestContext === 'function') {
          try { inherited = baseProbe.copyRequestContext(family) || {}; } catch (_) {}
        }
        if (String(family || '') === 'conversation_content' && mode === 'guest' && hasKeys(guestHeaders)) {
          return cloneHeaders(guestHeaders);
        }
        return cloneHeaders(inherited);
      },
    }));
    rootValue.__elonChatGptPrivateResearchProbe = probe;

    return {
      version: VERSION,
      fetch: wrappedFetch,
      probe: probe,
      diagnostics: function () {
        return Object.freeze({
          version: VERSION,
          mode: mode,
          exactTemplateObserved: Boolean(exactTemplate),
          contextReady: hasKeys(guestHeaders),
          rewrittenRequests: rewrittenRequests,
          fallbackRequests: fallbackRequests,
          lastStatus: lastStatus,
        });
      },
    };
  }

  return Object.freeze({ version: VERSION, install: install });
});
