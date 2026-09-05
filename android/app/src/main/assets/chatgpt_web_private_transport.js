(function () {
  'use strict';

  const existingTransport = window.__elonChatGptPrivateTransport;
  const prefetchEnabled = window.__elonChatGptPrivateConversationPrefetchEnabled === true;
  const researchEnabled = window.__elonChatGptPrivateResearchEnabled === true;
  if ((existingTransport && Number(existingTransport.version) >= 19) ||
      (!prefetchEnabled && !researchEnabled) ||
      location.origin !== 'https://chatgpt.com') return;

  const policyModule = window.__elonChatGptPrivateTransportPolicy;
  if (!policyModule || typeof policyModule.create !== 'function') return;
  const SAFE_PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const inheritedHeaders = new Map();
  const activeConversationRequests = new Map();
  const activeMembershipRequests = new Map();
  const activeContentReads = new Map();
  const authContext = window.__elonChatGptPrivateAuthContext;
  const privateConversationDirectory = window.__elonChatGptPrivateConversationDirectory;
  const delegateFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  let privateFetchDepth = 0;

  function optionalSessionStorage() {
    try {
      return window.sessionStorage || null;
    } catch (_) {
      return null;
    }
  }

  const policy = policyModule.create({
    enabled: prefetchEnabled,
    now: Date.now,
    storage: optionalSessionStorage()
  });
  let acceptedAuthSuccessAt = 0;

  function acceptAuthHealth(state) {
    const value = state && typeof state === 'object' ? state : {};
    const successAt = Math.max(0, Number(value.lastSuccessAt) || 0);
    if (value.ready !== true || value.lastOutcome !== 'session_ready' ||
        successAt <= acceptedAuthSuccessAt) return;
    acceptedAuthSuccessAt = successAt;
    policy.recordOfficial(200, Math.max(0, Number(value.lastLatencyMs) || 0));
  }

  if (authContext && typeof authContext.subscribe === 'function') {
    authContext.subscribe(acceptAuthHealth);
  }
  if (authContext && typeof authContext.state === 'function') {
    try { acceptAuthHealth(authContext.state()); } catch (_) {}
  }

  function requestUrl(input) {
    try {
      return new URL(typeof input === 'string' ? input : input && input.url, location.href);
    } catch (_) {
      return null;
    }
  }

  function isConversationContent(url) {
    return Boolean(url && url.origin === location.origin &&
      /^\/backend-api\/conversations\/[A-Za-z0-9_-]{1,160}$/.test(url.pathname));
  }

  function headerEntries(input, init) {
    const entries = new Map();
    function read(headers) {
      if (!headers) return;
      try {
        if (typeof headers.forEach === 'function') {
          headers.forEach((value, name) => entries.set(String(name), String(value)));
        } else if (Array.isArray(headers)) {
          headers.forEach((entry) => {
            if (Array.isArray(entry) && entry.length >= 2) entries.set(String(entry[0]), String(entry[1]));
          });
        } else if (typeof headers === 'object') {
          Object.keys(headers).forEach((name) => entries.set(name, String(headers[name])));
        }
      } catch (_) {
        // Browser-managed headers that cannot be read remain browser-managed.
      }
    }
    read(input && input.headers);
    read(init && init.headers);
    return entries;
  }

  if (delegateFetch) {
    window.fetch = function () {
      const input = arguments[0];
      const init = arguments[1] || {};
      const officialDetail = privateFetchDepth === 0 && isConversationContent(requestUrl(input));
      const startedAt = officialDetail ? Date.now() : 0;
      if (officialDetail) {
        const entries = headerEntries(input, init);
        if (entries.size) {
          inheritedHeaders.set('conversation_content', entries);
          if (authContext && typeof authContext.acceptObservedHeaders === 'function') {
            authContext.acceptObservedHeaders(entries);
          }
        }
      }
      let result;
      try {
        result = delegateFetch.apply(window, arguments);
      } catch (error) {
        if (officialDetail) policy.recordOfficial(0, Date.now() - startedAt);
        throw error;
      }
      if (!officialDetail) return result;
      return Promise.resolve(result).then(
        (response) => {
          policy.recordOfficial(Number(response && response.status), Date.now() - startedAt);
          return response;
        },
        (error) => {
          policy.recordOfficial(0, Date.now() - startedAt);
          throw error;
        }
      );
    };
  }

  function cleanText(value, maximum) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim()
      .slice(0, maximum);
  }

  function copiedRequestHeaders() {
    const local = inheritedHeaders.get('conversation_content');
    if (local) return local;
    const probe = window.__elonChatGptPrivateResearchProbe;
    const copied = probe && typeof probe.copyRequestContext === 'function'
      ? probe.copyRequestContext('conversation_content')
      : null;
    if (copied && typeof copied === 'object' && Object.keys(copied).length) {
      return new Map(Object.entries(copied));
    }
    const warmed = authContext && typeof authContext.copyRequestHeaders === 'function'
      ? authContext.copyRequestHeaders()
      : null;
    return warmed && typeof warmed === 'object' && Object.keys(warmed).length
      ? new Map(Object.entries(warmed))
      : null;
  }

  async function acquireRequestHeaders() {
    const copied = copiedRequestHeaders();
    if (copied) return copied;
    if (!authContext || typeof authContext.acquireRequestHeaders !== 'function') {
      throw new Error('missing_context');
    }
    const acquired = await authContext.acquireRequestHeaders();
    if (!acquired || typeof acquired !== 'object' || !Object.keys(acquired).length) {
      throw new Error('missing_context');
    }
    return new Map(Object.entries(acquired));
  }

  function fetchConversation(id, freshMembership) {
    if (freshMembership === true) return fetchConversationUnshared(id, true);
    if (activeContentReads.has(id)) return activeContentReads.get(id);
    const request = fetchConversationUnshared(id, false).finally(() => {
      if (activeContentReads.get(id) === request) activeContentReads.delete(id);
    });
    activeContentReads.set(id, request);
    return request;
  }

  async function fetchConversationUnshared(id, freshMembership) {
    const inherited = await acquireRequestHeaders();
    const startedAt = Date.now();
    const request = window.__elonChatGptPrivateJsonRequest;
    if (!request) throw new Error('request_unavailable');
    const headers = { Accept: 'application/json' };
    inherited.forEach((value, name) => { headers[name] = value; });
    try {
      privateFetchDepth += 1;
      const response = await request.request(window, '/backend-api/conversations/' + encodeURIComponent(id), {
        method: 'GET',
        credentials: 'include',
        cache: freshMembership === true ? 'no-store' : 'default',
        headers,
        __elonPrivateTransport: freshMembership === true
          ? 'conversation_membership'
          : 'conversation_prefetch'
      }, { timeoutMs: policy.attemptBudgetMs(), maxBytes: 4 * 1024 * 1024 });
      return { payload: response.payload, elapsedMs: Date.now() - startedAt };
    } catch (error) {
      if (/^http_(401|403)$/.test(String(error && error.message || ''))) {
        if (authContext && typeof authContext.invalidate === 'function') {
          authContext.invalidate('auth_rejected');
        }
      }
      throw error;
    } finally {
      privateFetchDepth = Math.max(0, privateFetchDepth - 1);
    }
  }

  function conversationMessages(payload) {
    const projection = window.__elonChatGptPrivateHistoryProjection;
    if (!projection || typeof projection.create !== 'function') return [];
    return projection.create({ streamPolicy: window.__elonChatGptPrivateStreamPolicy }).project(payload);
  }

  function normalizedConversationPayload(payload) {
    const values = [
      payload,
      payload && payload.conversation,
      payload && payload.data,
      payload && payload.data && payload.data.conversation,
      payload && payload.result,
      payload && payload.result && payload.result.conversation
    ];
    return values.find((value) => value && typeof value === 'object' && (
      value.mapping && typeof value.mapping === 'object' || Array.isArray(value.messages) ||
      Array.isArray(value.linear_conversation) || Array.isArray(value.items))) || payload;
  }

  function copySameOriginRequestHeaders() {
    const copied = copiedRequestHeaders();
    if (!copied) return null;
    const result = {};
    copied.forEach((value, name) => { result[String(name)] = String(value); });
    return result;
  }

  async function acquireSameOriginRequestHeaders() {
    const copied = await acquireRequestHeaders();
    const result = {};
    copied.forEach((value, name) => { result[String(name)] = String(value); });
    return result;
  }

  function conversationProjectId(payload) {
    const normalized = normalizedConversationPayload(payload);
    const values = [
      normalized,
      payload,
      payload && payload.conversation,
      payload && payload.data,
      payload && payload.data && payload.data.conversation,
      payload && payload.result,
      payload && payload.result && payload.result.conversation
    ];
    for (const value of values) {
      if (!value || typeof value !== 'object') continue;
      const gizmo = value.gizmo && typeof value.gizmo === 'object' ? value.gizmo : null;
      const candidates = [
        value.project_id,
        value.projectId,
        value.gizmo_id,
        value.conversation_template_id,
        value.conversationTemplateId,
        value.workspace_id,
        value.workspaceId,
        gizmo && gizmo.id,
        value.project && value.project.id,
        value.project && value.project.project_id
      ];
      const match = candidates.map((candidate) => cleanText(candidate, 180))
        .find((candidate) => SAFE_PROJECT_ID.test(candidate));
      if (match) return match;
    }
    return '';
  }

  function canonicalProjectId(value) {
    const cleaned = cleanText(value, 180);
    const production = cleaned.match(/^(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]+)?$/);
    if (production) return production[1];
    return SAFE_PROJECT_ID.test(cleaned) ? cleaned : '';
  }

  function conversationHasProject(payload, expectedProjectId) {
    const expected = canonicalProjectId(expectedProjectId);
    if (!expected) return false;
    if (canonicalProjectId(conversationProjectId(payload)) === expected) return true;
    const seen = new Set();
    let visited = 0;
    function visit(value, depth, projectContext) {
      if (depth > 6 || visited >= 512 || value == null) return false;
      if (typeof value === 'string') {
        return projectContext && canonicalProjectId(value) === expected;
      }
      if (typeof value !== 'object' || seen.has(value)) return false;
      seen.add(value);
      visited += 1;
      if (Array.isArray(value)) {
        return value.slice(0, 64).some((entry) => visit(entry, depth + 1, projectContext));
      }
      return Object.keys(value).slice(0, 96).some((key) => {
        const semantic = projectContext || /project|gizmo|template|scope|workspace/i.test(key);
        return visit(value[key], depth + 1, semantic);
      });
    }
    return visit(payload, 0, false);
  }

  function projectDirectoryHasConversation(conversationId, expectedProjectId) {
    if (!privateConversationDirectory ||
        typeof privateConversationDirectory.refreshProject !== 'function' ||
        typeof privateConversationDirectory.snapshot !== 'function') {
      return Promise.resolve(false);
    }
    return Promise.resolve(privateConversationDirectory.refreshProject(expectedProjectId))
      .then((refreshed) => {
        if (!refreshed) return false;
        const snapshot = privateConversationDirectory.snapshot();
        return Boolean(snapshot && Array.isArray(snapshot.conversations) &&
          snapshot.conversations.some((row) => row && row.id === conversationId &&
            canonicalProjectId(row.projectId) === canonicalProjectId(expectedProjectId)));
      }).catch(() => false);
  }

  function conversationPrefetchReady() {
    const canAcquire = authContext && typeof authContext.canAcquire === 'function' &&
      authContext.canAcquire();
    return policy.canAttempt(Boolean(copiedRequestHeaders()) || canAcquire);
  }

  function failureKind(error) {
    const message = String(error && error.message || 'network');
    if (message === 'timeout') return 'timeout';
    if (message === 'missing_context') return 'context';
    if (/^(?:auth_http_|http_)(401|403)$/.test(message) || /^auth_/.test(message)) return 'auth';
    if (/^http_/.test(message)) return 'http';
    if (/json|parse/i.test(message)) return 'parse';
    return 'network';
  }

  function recordPrivateOutcome(outcome, messageCount, elapsedMs) {
    const probe = window.__elonChatGptPrivateResearchProbe;
    if (probe && typeof probe.recordPrivateOutcome === 'function') {
      probe.recordPrivateOutcome(outcome, messageCount, elapsedMs);
    }
  }

  function recordPrivatePayloadShape(payload) {
    const probe = window.__elonChatGptPrivateResearchProbe;
    if (probe && typeof probe.recordPrivatePayloadShape === 'function') {
      probe.recordPrivatePayloadShape(payload);
    }
  }

  function conversationTarget(path) {
    const value = String(path || '').trim();
    const match = value.match(
      /^(?:\/c\/([A-Za-z0-9_-]{1,160})|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/([A-Za-z0-9_-]{1,160}))$/
    );
    if (!match) return null;
    return { path: value, id: match[1] || match[2] };
  }

  function emitConversationSnapshot(target, result, emitEvent) {
    recordPrivatePayloadShape(result.payload);
    const payload = normalizedConversationPayload(result.payload);
    const messages = conversationMessages(payload);
    if (!messages.length) {
      policy.recordFailure('empty');
      recordPrivateOutcome('empty', 0, result.elapsedMs);
      return;
    }
    policy.recordSuccess(result.elapsedMs);
    recordPrivateOutcome('success', messages.length, result.elapsedMs);
    emitEvent({
      type: 'message_snapshot',
      snapshotScope: 'content',
      title: cleanText(payload && payload.title, 120),
      url: location.origin + target.path,
      draft: '',
      messages,
      observedMessageCount: messages.length,
      messageWindowStart: 0,
      authenticated: true,
      pageKind: 'conversation',
      loginRequired: false,
      composerReady: false,
      streaming: messages.some((message) => message.state === 'streaming'),
      currentModel: cleanText(payload && (payload.default_model_slug || payload.model), 80),
      attachments: [],
      dictationActive: false,
      capabilities: ['conversation_history']
    });
  }

  function settle(action) {
    if (typeof action !== 'function') return;
    try { action(); } catch (_) { /* The official navigation fallback owns its errors. */ }
  }

  function requestConversationSnapshot(path, emitEvent, onSettled) {
    const target = conversationTarget(path);
    if (!target || typeof emitEvent !== 'function') return false;
    if (!conversationPrefetchReady()) return false;
    let request = activeConversationRequests.get(target.id);
    if (!request) {
      request = fetchConversation(target.id).then((result) => {
        emitConversationSnapshot(target, result, emitEvent);
      }).catch((error) => {
        const outcome = failureKind(error);
        policy.recordFailure(outcome);
        recordPrivateOutcome(outcome, 0, 0);
      }).finally(() => {
        if (activeConversationRequests.get(target.id) === request) {
          activeConversationRequests.delete(target.id);
        }
      });
      activeConversationRequests.set(target.id, request);
    }
    request.then(() => settle(onSettled));
    return true;
  }

  function prefetchConversation(path, emitEvent, navigate) {
    return requestConversationSnapshot(path, emitEvent, navigate);
  }

  function refreshCurrentConversation(path, emitEvent) {
    const target = conversationTarget(path);
    if (!target || target.path !== location.pathname) return false;
    return requestConversationSnapshot(target.path, emitEvent, null);
  }

  function probeConversationProject(path, expectedProjectId, onSettled) {
    const target = conversationTarget(path);
    const expected = cleanText(expectedProjectId, 180);
    if (!target || !SAFE_PROJECT_ID.test(expected) || !conversationPrefetchReady() ||
        !privateConversationDirectory ||
        typeof privateConversationDirectory.acceptConversationMembership !== 'function') return false;
    const key = target.id + ':' + expected;
    let request = activeMembershipRequests.get(key);
    if (!request) {
      request = fetchConversation(target.id, true).then((result) => {
        const payload = normalizedConversationPayload(result.payload);
        const directMatch = conversationHasProject(result.payload, expected);
        const membership = directMatch
          ? Promise.resolve(true)
          : projectDirectoryHasConversation(target.id, expected);
        return membership.then((matched) => {
          if (matched) {
            privateConversationDirectory.acceptConversationMembership(
              target.id,
              cleanText(payload && payload.title, 160),
              expected
            );
          }
          policy.recordSuccess(result.elapsedMs);
          return matched;
        });
      }).catch((error) => {
        policy.recordFailure(failureKind(error));
        return false;
      }).finally(() => {
        if (activeMembershipRequests.get(key) === request) activeMembershipRequests.delete(key);
      });
      activeMembershipRequests.set(key, request);
    }
    request.then((matched) => settle(() => onSettled && onSettled(matched)));
    return true;
  }

  async function listConversationFiles(path, requestId, emitEvent, respond) {
    const target = conversationTarget(path);
    const action = 'list_conversation_files';
    if (!target || !/^mcp_[a-z0-9]{1,32}$/.test(String(requestId || ''))) {
      return respond(action, false, 'invalid_file_request');
    }
    if (!conversationPrefetchReady()) return respond(action, false, 'files_not_ready');
    try {
      const result = await fetchConversation(target.id);
      const projection = window.__elonChatGptPrivateHistoryProjection;
      const index = projection && projection.create({}).files(result.payload);
      if (!index) throw new Error('parse_files_unknown');
      policy.recordSuccess(result.elapsedMs);
      emitEvent({ type: 'conversation_files_snapshot', conversationPath: target.path,
        requestId, files: index.files, truncated: index.truncated });
      respond(action, true, 'private_files_ready');
    } catch (error) {
      policy.recordFailure(failureKind(error));
      respond(action, false, 'files_read_failed');
    }
  }

  window.__elonChatGptPrivateTransport = Object.freeze({
    version: 19,
    conversationPrefetchEnabled: prefetchEnabled,
    conversationPrefetchAvailable: true,
    experimentalConversationPrefetchAvailable: true,
    conversationPrefetchReady,
    prefetchConversation,
    refreshCurrentConversation,
    probeConversationProject,
    listConversationFiles,
    copySameOriginRequestHeaders,
    acquireSameOriginRequestHeaders,
    health: policy.snapshot
  });
})();
