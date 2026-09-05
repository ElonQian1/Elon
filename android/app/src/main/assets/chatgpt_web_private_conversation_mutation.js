(function (root, factory) {
  'use strict';

  const exported = Object.freeze({ version: 5, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (!root || !root.location || root.location.origin !== 'https://chatgpt.com') return;
  const current = root.__elonChatGptPrivateConversationMutation;
  if (current && Number(current.version) >= exported.version) return;
  root.__elonChatGptPrivateConversationMutation = Object.freeze(factory(root, {
    enabled: root.__elonChatGptPrivateConversationMutationsEnabled === true,
    privateTransport: root.__elonChatGptPrivateTransport,
    directory: root.__elonChatGptPrivateConversationDirectory
  }));
})(typeof window === 'object' ? window : globalThis, function (root, dependencies) {
  'use strict';

  const VERSION = 5;
  const WRITE_TIMEOUT_MS = 9000;
  const RECONCILE_TIMEOUT_MS = 4000;
  const UNCERTAIN_RECONCILE_WINDOW_MS = 16000;
  const UNCERTAIN_RECONCILE_BACKOFF_MS = Object.freeze([750, 1750, 3500, 5000]);
  const RETRY_COOLDOWN_MS = 5000;
  const CIRCUIT_COOLDOWN_MS = 45000;
  const MAX_FAILURES = 2;
  const MAX_TITLE_LENGTH = 160;
  const SAFE_PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const PRODUCTION_PROJECT_ID = /^(g-p-[A-Fa-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?$/;
  const BLOCKED_INHERITED_HEADERS = new Set([
    'connection',
    'content-length',
    'cookie',
    'host',
    'origin',
    'referer',
    'transfer-encoding',
    'user-agent'
  ]);
  const enabled = dependencies && dependencies.enabled === true;
  const privateTransport = dependencies && dependencies.privateTransport;
  const directory = dependencies && dependencies.directory;
  let active = null;
  let failures = 0;
  let cooldownUntil = 0;
  let lastOutcome = 'none';
  let lastLatencyMs = 0;

  function now() {
    return Date.now();
  }

  function sleep(delayMs) {
    return new Promise((resolve) => root.setTimeout(resolve, Math.max(0, delayMs)));
  }

  function targetFromPath(rawPath) {
    const path = String(rawPath || '').trim();
    const match = path.match(
      /^(?:\/c\/([A-Za-z0-9_-]{1,160})|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/([A-Za-z0-9_-]{1,160}))$/
    );
    return match ? Object.freeze({ path, id: match[1] || match[2] }) : null;
  }

  function titleFromInput(rawTitle) {
    const title = String(rawTitle || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
    return title && title.length <= MAX_TITLE_LENGTH ? title : '';
  }

  function projectIdFromInput(rawProjectId) {
    const projectId = String(rawProjectId || '').trim();
    if (!SAFE_PROJECT_ID.test(projectId)) return '';
    const production = projectId.match(PRODUCTION_PROJECT_ID);
    return production ? production[1] : projectId;
  }

  function supported() {
    return Boolean(enabled && root && typeof root.fetch === 'function' && privateTransport &&
      root.__elonChatGptPrivateJsonRequest &&
      typeof privateTransport.acquireSameOriginRequestHeaders === 'function');
  }

  function state() {
    return Object.freeze({
      version: VERSION,
      enabled,
      supported: supported(),
      state: active ? 'busy' : cooldownUntil > now() ? 'cooldown' : supported() ? 'ready' : 'unavailable',
      failures,
      cooldownRemainingMs: Math.max(0, cooldownUntil - now()),
      lastOutcome,
      lastLatencyMs
    });
  }

  function sanitizeHeaders(source) {
    const headers = { Accept: 'application/json', 'Content-Type': 'application/json' };
    if (!source || typeof source !== 'object') return headers;
    Object.keys(source).forEach((name) => {
      const lower = String(name).toLowerCase();
      if (!lower || BLOCKED_INHERITED_HEADERS.has(lower) || lower === 'content-type') return;
      headers[name] = String(source[name]);
    });
    return headers;
  }

  function authorizationPresent(headers) {
    return Object.keys(headers).some((name) =>
      String(name).toLowerCase() === 'authorization' && /^Bearer\s+\S{8,65536}$/.test(headers[name])
    );
  }

  async function acquireHeaders() {
    const value = sanitizeHeaders(await privateTransport.acquireSameOriginRequestHeaders());
    if (!authorizationPresent(value)) throw new Error('auth_unavailable');
    return value;
  }

  function fetchWithTimeout(url, options, timeoutMs, mode) {
    const request = root.__elonChatGptPrivateJsonRequest;
    if (!request) return Promise.reject(new Error('request_unavailable'));
    return request.request(root, url, options, {
      timeoutMs, maxBytes: 4 * 1024 * 1024, mode: mode || 'none'
    });
  }

  function candidateArrays(payload) {
    if (Array.isArray(payload)) return [payload];
    if (!payload || typeof payload !== 'object') return [];
    const data = payload.data && typeof payload.data === 'object' ? payload.data : null;
    return [
      payload.items,
      payload.pins,
      payload.conversations,
      data && data.items,
      data && data.pins,
      data && data.conversations
    ].filter(Array.isArray);
  }

  function itemConversationId(value) {
    if (typeof value === 'string') return value;
    if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
    return String(value.id || value.conversation_id || value.conversationId || '').trim();
  }

  function pinStateFromPayload(payload, conversationId) {
    const arrays = candidateArrays(payload);
    if (!arrays.length) return Object.freeze({ known: false, pinned: false });
    const pinned = arrays.some((values) => values.some((value) =>
      itemConversationId(value) === conversationId
    ));
    const hasMore = Boolean(payload && typeof payload === 'object' && (
      payload.has_more === true || payload.hasMore === true || payload.next_cursor || payload.nextCursor ||
      payload.data && (payload.data.has_more === true || payload.data.hasMore === true ||
        payload.data.next_cursor || payload.data.nextCursor)
    ));
    return Object.freeze({ known: pinned || !hasMore, pinned });
  }

  function conversationCandidates(payload) {
    return [
      payload,
      payload && payload.conversation,
      payload && payload.data,
      payload && payload.data && payload.data.conversation,
      payload && payload.result,
      payload && payload.result && payload.result.conversation
    ].filter((value) => value && typeof value === 'object' && !Array.isArray(value));
  }

  function conversationStateFromPayload(payload) {
    const candidates = conversationCandidates(payload);
    const titleSource = candidates.find((value) => typeof value.title === 'string');
    const archiveSource = candidates.find((value) => typeof value.is_archived === 'boolean');
    let projectKnown = false;
    let projectId = '';
    candidates.some((value) => {
      const gizmo = value.gizmo && typeof value.gizmo === 'object' ? value.gizmo : null;
      const project = value.project && typeof value.project === 'object' ? value.project : null;
      const fields = [
        ['project_id', value.project_id],
        ['projectId', value.projectId],
        ['gizmo_id', value.gizmo_id],
        ['gizmo.id', gizmo && gizmo.id],
        ['project.id', project && project.id],
        ['project.project_id', project && project.project_id]
      ];
      const present = fields.find(([name]) => {
        const topLevelName = name.split('.')[0];
        return Object.prototype.hasOwnProperty.call(value, topLevelName);
      });
      if (!present) return false;
      projectKnown = true;
      projectId = fields.map(([, candidate]) => projectIdFromInput(candidate))
        .find(Boolean) || '';
      return true;
    });
    return Object.freeze({
      titleKnown: Boolean(titleSource),
      title: titleSource ? titleFromInput(titleSource.title) : '',
      archivedKnown: Boolean(archiveSource),
      archived: archiveSource ? archiveSource.is_archived : false,
      projectKnown,
      projectId,
      metadata: titleSource || archiveSource || candidates[0] || null
    });
  }

  function acceptPinnedState(conversationId, pinned) {
    if (!directory || typeof directory.acceptPinnedState !== 'function') return false;
    try { return directory.acceptPinnedState(conversationId, pinned); }
    catch (_) { return false; }
  }

  function acceptTitleState(conversationId, title) {
    if (!directory || typeof directory.acceptTitleState !== 'function') return false;
    try { return directory.acceptTitleState(conversationId, title); }
    catch (_) { return false; }
  }

  function acceptArchivedState(conversationId, archived, metadata) {
    if (!directory || typeof directory.acceptArchivedState !== 'function') return false;
    try { return directory.acceptArchivedState(conversationId, archived, metadata); }
    catch (_) { return false; }
  }

  function acceptProjectState(conversationId, title, projectId) {
    if (!directory || typeof directory.acceptConversationMembership !== 'function') return false;
    try { return directory.acceptConversationMembership(conversationId, title, projectId); }
    catch (_) { return false; }
  }

  async function readJson(url, headers, timeoutMs, transportLabel) {
    let response;
    try {
      response = await fetchWithTimeout(url, {
        method: 'GET',
        credentials: 'include',
        cache: 'no-store',
        headers,
        __elonPrivateTransport: transportLabel
      }, Math.max(250, Number(timeoutMs) || RECONCILE_TIMEOUT_MS), 'json');
    } catch (_) {
      return null;
    }
    return response && response.ok ? response.payload : null;
  }

  async function reconcilePinned(conversationId, expectedPinned, headers, timeoutMs) {
    const payload = await readJson(
      '/backend-api/pins', headers, timeoutMs, 'conversation_pin_reconcile_v1'
    );
    if (!payload) return Object.freeze({ confirmed: false, known: false });
    const observed = pinStateFromPayload(payload, conversationId);
    if (observed.known && observed.pinned === expectedPinned) {
      acceptPinnedState(conversationId, observed.pinned);
    }
    return Object.freeze({
      confirmed: observed.known && observed.pinned === expectedPinned,
      known: observed.known
    });
  }

  async function reconcileConversation(conversationId, mutation, headers, timeoutMs) {
    const payload = await readJson(
      '/backend-api/conversations/' + encodeURIComponent(conversationId),
      headers,
      timeoutMs,
      'conversation_metadata_reconcile_v1'
    );
    const observed = payload ? conversationStateFromPayload(payload) : Object.freeze({
      titleKnown: false,
      title: '',
      archivedKnown: false,
      archived: false,
      projectKnown: false,
      projectId: '',
      metadata: null
    });
    if (mutation.kind === 'rename') {
      const confirmed = observed.titleKnown && observed.title === mutation.title;
      if (confirmed) acceptTitleState(conversationId, observed.title);
      return Object.freeze({ confirmed, known: observed.titleKnown });
    }
    if (mutation.kind === 'project') {
      const directConfirmed = observed.projectKnown && observed.projectId === mutation.projectId;
      if (directConfirmed) {
        acceptProjectState(conversationId, observed.title, observed.projectId);
        return Object.freeze({ confirmed: true, known: true });
      }
      if (directory && typeof directory.refreshProject === 'function' &&
          typeof directory.snapshot === 'function') {
        try {
          const refreshed = await directory.refreshProject(mutation.projectId);
          const snapshot = refreshed ? directory.snapshot() : null;
          const row = snapshot && Array.isArray(snapshot.conversations)
            ? snapshot.conversations.find((value) => value && value.id === conversationId &&
              projectIdFromInput(value.projectId) === mutation.projectId)
            : null;
          if (row) {
            acceptProjectState(conversationId, row.title || observed.title, mutation.projectId);
            return Object.freeze({ confirmed: true, known: true });
          }
        } catch (_) {}
      }
      return Object.freeze({ confirmed: false, known: observed.projectKnown });
    }
    if (!payload) return Object.freeze({ confirmed: false, known: false });
    const confirmed = observed.archivedKnown && observed.archived === mutation.archived;
    if (confirmed) acceptArchivedState(conversationId, observed.archived, observed.metadata);
    return Object.freeze({ confirmed, known: observed.archivedKnown });
  }

  function reconcileMutation(conversationId, mutation, headers, timeoutMs) {
    return mutation.kind === 'pin'
      ? reconcilePinned(conversationId, mutation.pinned, headers, timeoutMs)
      : reconcileConversation(conversationId, mutation, headers, timeoutMs);
  }

  async function reconcileUncertainWrite(conversationId, mutation, headers) {
    const deadline = now() + UNCERTAIN_RECONCILE_WINDOW_MS;
    let known = false;
    for (const delayMs of UNCERTAIN_RECONCILE_BACKOFF_MS) {
      const beforeDelay = deadline - now();
      if (beforeDelay <= 0) break;
      await sleep(Math.min(delayMs, beforeDelay));
      const remaining = deadline - now();
      if (remaining <= 0) break;
      const observed = await reconcileMutation(
        conversationId,
        mutation,
        headers,
        Math.min(RECONCILE_TIMEOUT_MS, remaining)
      );
      known = known || observed.known;
      if (observed.confirmed) return Object.freeze({ confirmed: true, known: true });
    }
    return Object.freeze({ confirmed: false, known });
  }

  function recordFailure(outcome) {
    failures = Math.min(10, failures + 1);
    const longCooldown = failures >= MAX_FAILURES || outcome === 'auth';
    cooldownUntil = now() + (longCooldown ? CIRCUIT_COOLDOWN_MS : RETRY_COOLDOWN_MS);
    lastOutcome = outcome;
  }

  function recordSuccess(latencyMs) {
    failures = 0;
    cooldownUntil = 0;
    lastOutcome = 'success';
    lastLatencyMs = Math.max(0, Math.min(30000, Number(latencyMs) || 0));
  }

  function failureCode(error) {
    const message = String(error && error.message || 'network');
    if (message === 'timeout') return 'mutation_timeout';
    if (/auth|401|403/.test(message)) return 'mutation_auth_unavailable';
    return 'mutation_network_failure';
  }

  function rejected(code, attempted, reconciled) {
    return Object.freeze({
      ok: false,
      code,
      attempted: attempted === true,
      reconciled: reconciled === true
    });
  }

  function acknowledgeMutation(target, mutation) {
    if (mutation.kind === 'pin') return acceptPinnedState(target.id, mutation.pinned);
    if (mutation.kind === 'rename') return acceptTitleState(target.id, mutation.title);
    if (mutation.kind === 'project') {
      return acceptProjectState(target.id, mutation.conversationTitle, mutation.projectId);
    }
    return acceptArchivedState(target.id, mutation.archived, null);
  }

  async function executeMutation(target, mutation) {
    let headers;
    try {
      headers = await acquireHeaders();
    } catch (error) {
      recordFailure('auth');
      return rejected(failureCode(error), false);
    }
    const startedAt = now();
    try {
      await fetchWithTimeout(
        '/backend-api/conversation/' + encodeURIComponent(target.id),
        {
          method: 'PATCH',
          credentials: 'include',
          cache: 'no-store',
          headers,
          body: JSON.stringify(mutation.body),
          __elonPrivateTransport: mutation.transport
        },
        WRITE_TIMEOUT_MS
      );
    } catch (error) {
      const http = String(error && error.message || '').match(/^http_(\d+)$/);
      if (http) {
        const status = Number(http[1]);
        if (status === 401 || status === 403) {
          const authContext = root.__elonChatGptPrivateAuthContext;
          if (authContext && typeof authContext.invalidate === 'function') {
            authContext.invalidate('conversation_mutation_rejected');
          }
        }
        recordFailure(status === 401 || status === 403 ? 'auth' : 'http');
        return rejected('mutation_http_' + status, true);
      }
      const code = failureCode(error);
      if (code === 'mutation_timeout' || code === 'mutation_network_failure') {
        const reconciliation = await reconcileUncertainWrite(target.id, mutation, headers);
        if (reconciliation.confirmed) {
          recordSuccess(now() - startedAt);
          return Object.freeze({
            ok: true,
            code: code === 'mutation_timeout'
              ? 'mutation_confirmed_after_timeout'
              : 'mutation_confirmed_after_transport_error',
            attempted: true,
            reconciled: true
          });
        }
        recordFailure(code === 'mutation_timeout' ? 'timeout' : 'network');
        return rejected(code, true, reconciliation.known);
      }
      recordFailure(code === 'mutation_auth_unavailable' ? 'auth' : 'network');
      return rejected(code, true);
    }
    recordSuccess(now() - startedAt);
    acknowledgeMutation(target, mutation);
    const reconciliation = await reconcileMutation(
      target.id, mutation, headers, RECONCILE_TIMEOUT_MS
    );
    return Object.freeze({
      ok: true,
      code: reconciliation.confirmed ? 'mutation_confirmed' : 'mutation_server_acknowledged',
      attempted: true,
      reconciled: reconciliation.confirmed
    });
  }

  function startMutation(rawPath, mutation) {
    const target = targetFromPath(rawPath);
    if (!target || !mutation) return Promise.resolve(rejected('invalid_mutation', false));
    if (!supported()) return Promise.resolve(rejected('mutation_unavailable', false));
    if (active) return Promise.resolve(rejected('mutation_busy', false));
    if (cooldownUntil > now()) return Promise.resolve(rejected('mutation_circuit_open', false));
    const request = executeMutation(target, mutation).finally(() => {
      if (active === request) active = null;
    });
    active = request;
    return request;
  }

  function setPinned(rawPath, pinned) {
    if (typeof pinned !== 'boolean') return Promise.resolve(rejected('invalid_mutation', false));
    return startMutation(rawPath, Object.freeze({
      kind: 'pin',
      pinned,
      body: { is_starred: pinned },
      transport: 'conversation_pin_v1'
    }));
  }

  function setArchived(rawPath, archived) {
    if (typeof archived !== 'boolean') return Promise.resolve(rejected('invalid_mutation', false));
    return startMutation(rawPath, Object.freeze({
      kind: 'archive',
      archived,
      body: { is_archived: archived },
      transport: 'conversation_archive_v1'
    }));
  }

  function rename(rawPath, rawTitle) {
    const title = titleFromInput(rawTitle);
    if (!title) return Promise.resolve(rejected('invalid_mutation', false));
    return startMutation(rawPath, Object.freeze({
      kind: 'rename',
      title,
      body: { title },
      transport: 'conversation_rename_v1'
    }));
  }

  function moveToProject(rawPath, rawProjectId, rawConversationTitle) {
    const projectId = projectIdFromInput(rawProjectId);
    if (!projectId) return Promise.resolve(rejected('invalid_mutation', false));
    return startMutation(rawPath, Object.freeze({
      kind: 'project',
      projectId,
      conversationTitle: titleFromInput(rawConversationTitle),
      body: { gizmo_id: projectId },
      transport: 'conversation_project_move_v1'
    }));
  }

  function commandMutation(action, command) {
    if (action === 'set_conversation_pinned') {
      return { run: () => setPinned(String(command && command.value || ''), command && command.selected) };
    }
    if (action === 'set_conversation_archived') {
      return { run: () => setArchived(String(command && command.value || ''), command && command.selected) };
    }
    if (action === 'rename_conversation') {
      return { run: () => rename(String(command && command.value || ''), command && command.title) };
    }
    if (action === 'move_conversation_to_project') {
      return { run: () => moveToProject(
        String(command && command.value || ''),
        command && command.projectScopeId,
        command && command.title
      ) };
    }
    return null;
  }

  function handle(action, command, respond, scheduleSnapshot, directoryRequests) {
    const mutation = commandMutation(action, command);
    if (!mutation) return false;
    mutation.run().then((outcome) => {
      const value = outcome && typeof outcome === 'object' ? outcome : {};
      if (value.ok === true) {
        if (directoryRequests && typeof directoryRequests.emitSnapshot === 'function') {
          directoryRequests.emitSnapshot(null);
        }
        if (typeof scheduleSnapshot === 'function') scheduleSnapshot(true);
      }
      respond(action, value.ok === true, String(value.code || 'mutation_failed'));
    }).catch(() => respond(action, false, 'mutation_failed'));
    return true;
  }

  return Object.freeze({
    version: VERSION,
    enabled,
    setPinned,
    setArchived,
    rename,
    moveToProject,
    handle,
    state
  });
});
