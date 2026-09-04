(function () {
  'use strict';

  const existing = window.__elonChatGptPrivateConversationDirectory;
  if (existing && Number(existing.version) >= 6) return;
  if (location.origin !== 'https://chatgpt.com') return;

  const originalFetch = typeof window.fetch === 'function' ? window.fetch.bind(window) : null;
  const xhrPrototype = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
  const originalOpen = xhrPrototype && xhrPrototype.open;
  const originalSend = xhrPrototype && xhrPrototype.send;
  const xhrMetadata = new WeakMap();
  const conversations = new Map();
  const projects = new Map();
  const projectRefreshes = new Map();
  const pinnedStateOverrides = new Map();
  let listener = null;
  let revision = 0;
  const MAX_CONVERSATIONS = 200;
  const MAX_PROJECTS = 40;
  const MAX_RESPONSE_BYTES = 1024 * 1024;
  const MAX_TITLE_LENGTH = 160;
  const SAFE_ID = /^[A-Za-z0-9_-]{1,160}$/;
  const SAFE_PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const PROJECT_REFRESH_TIMEOUT_MS = 4000;
  const PIN_OVERRIDE_TTL_MS = 120000;

  function cleanText(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().slice(0, MAX_TITLE_LENGTH);
  }

  function requestMetadata(input, init) {
    let url;
    try { url = new URL(String(input && input.url || input || ''), location.href); }
    catch (_) { return null; }
    const method = String(init && init.method || input && input.method || 'GET').toUpperCase();
    if (method !== 'GET' || url.origin !== location.origin) return null;
    if (url.pathname === '/backend-api/conversations') {
      return { family: 'conversations', projectId: '' };
    }
    if (url.pathname === '/backend-api/gizmos/snorlax/sidebar') {
      return { family: 'projects', projectId: '' };
    }
    const match = url.pathname.match(
      /^\/backend-api\/gizmos\/(g-p-[A-Za-z0-9_-]{1,160})\/conversations$/
    );
    return match ? { family: 'project_conversations', projectId: match[1] } : null;
  }

  function parsePayload(text) {
    const normalized = String(text || '').replace(/^\)\]\}'\s*/, '').trim();
    if (!normalized || normalized.length > MAX_RESPONSE_BYTES) return null;
    try { return JSON.parse(normalized); }
    catch (_) { return null; }
  }

  function candidateArrays(payload) {
    if (Array.isArray(payload)) return [payload];
    if (!payload || typeof payload !== 'object') return [];
    const data = payload.data && typeof payload.data === 'object' ? payload.data : null;
    return [
      payload.items,
      payload.conversations,
      payload.results,
      data && data.items,
      data && data.conversations,
      data && data.results
    ].filter(Array.isArray);
  }

  function projectIdFrom(value, fallback) {
    const candidates = [
      fallback,
      value && value.project_id,
      value && value.projectId,
      value && value.gizmo_id,
      value && value.gizmo && value.gizmo.id
    ];
    return candidates.map((candidate) => cleanText(candidate))
      .find((candidate) => SAFE_PROJECT_ID.test(candidate)) || '';
  }

  function explicitPinnedState(value) {
    if (!value || typeof value !== 'object') return null;
    const keys = ['is_starred', 'is_pinned', 'pinned'];
    for (const key of keys) {
      if (Object.prototype.hasOwnProperty.call(value, key) && typeof value[key] === 'boolean') {
        return value[key];
      }
    }
    return null;
  }

  function resolvedPinnedState(id, observed) {
    const override = pinnedStateOverrides.get(id);
    if (override) {
      if (override.expiresAt <= Date.now()) {
        pinnedStateOverrides.delete(id);
      } else {
        if (typeof observed === 'boolean' && observed === override.pinned) {
          pinnedStateOverrides.delete(id);
        }
        return override.pinned;
      }
    }
    if (typeof observed === 'boolean') return observed;
    const previous = conversations.get(id);
    return previous && typeof previous.pinned === 'boolean' ? previous.pinned : null;
  }

  function conversationFrom(value, fallbackProjectId, order) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
    const id = cleanText(value.id || value.conversation_id || value.conversationId);
    const title = cleanText(value.title || value.name);
    if (!SAFE_ID.test(id) || !title) return null;
    const projectId = projectIdFrom(value, fallbackProjectId);
    return Object.freeze({
      id,
      title,
      path: projectId ? '/g/' + projectId + '/c/' + id : '/c/' + id,
      active: false,
      pinned: resolvedPinnedState(id, explicitPinnedState(value)),
      groupLabel: '',
      projectId: projectId || null,
      projectTitle: null,
      projectPath: projectId ? '/g/' + projectId + '/project' : null,
      activityDates: [],
      order
    });
  }

  function collectConversations(payload, projectId) {
    const rows = [];
    candidateArrays(payload).some((values) => {
      values.slice(0, MAX_CONVERSATIONS).forEach((value, index) => {
        const row = conversationFrom(value, projectId, index);
        if (row) rows.push(row);
      });
      return rows.length > 0;
    });
    return rows;
  }

  function acceptConversationMembership(rawId, rawTitle, rawProjectId) {
    const id = cleanText(rawId);
    const projectId = cleanText(rawProjectId);
    const previous = conversations.get(id);
    const title = cleanText(rawTitle) || (previous && previous.title) || '';
    if (!SAFE_ID.test(id) || !SAFE_PROJECT_ID.test(projectId) || !title) return false;
    const path = '/g/' + projectId + '/c/' + id;
    const project = projects.get(projectId);
    const next = Object.freeze({
      id,
      title,
      path,
      active: location.pathname === path,
      pinned: previous && typeof previous.pinned === 'boolean' ? previous.pinned : null,
      groupLabel: previous && previous.groupLabel || '',
      projectId,
      projectTitle: project ? project.title : null,
      projectPath: '/g/' + projectId + '/project',
      activityDates: previous && Array.isArray(previous.activityDates)
        ? previous.activityDates.slice(0, 32)
        : [],
      order: previous && Number.isFinite(previous.order) ? previous.order : conversations.size
    });
    conversations.set(id, next);
    trimMap(conversations, MAX_CONVERSATIONS);
    if (!previous || previous.path !== next.path || previous.title !== next.title) notify();
    return true;
  }

  function acceptPinnedState(rawId, pinned) {
    const id = cleanText(rawId);
    const previous = conversations.get(id);
    if (!SAFE_ID.test(id) || !previous || typeof pinned !== 'boolean') return false;
    pinnedStateOverrides.set(id, Object.freeze({
      pinned,
      expiresAt: Date.now() + PIN_OVERRIDE_TTL_MS
    }));
    trimMap(pinnedStateOverrides, MAX_CONVERSATIONS);
    if (previous.pinned === pinned) return true;
    conversations.set(id, Object.freeze(Object.assign({}, previous, { pinned })));
    notify();
    return true;
  }

  function projectTitle(value) {
    const gizmo = value && value.gizmo && typeof value.gizmo === 'object' ? value.gizmo : null;
    const display = value && value.display && typeof value.display === 'object' ? value.display : null;
    const gizmoDisplay = gizmo && gizmo.display && typeof gizmo.display === 'object'
      ? gizmo.display
      : null;
    return cleanText(
      value && (value.title || value.name || value.display_name) ||
      display && (display.name || display.title) ||
      gizmoDisplay && (gizmoDisplay.name || gizmoDisplay.title) ||
      gizmo && (gizmo.name || gizmo.title)
    );
  }

  function collectProjects(payload) {
    const rows = [];
    const seen = new Set();
    let inspected = 0;
    function visit(value, depth) {
      if (!value || rows.length >= MAX_PROJECTS || inspected >= 800 || depth > 7) return;
      inspected += 1;
      if (Array.isArray(value)) {
        value.slice(0, 100).forEach((item) => visit(item, depth + 1));
        return;
      }
      if (typeof value !== 'object') return;
      const id = projectIdFrom(value, value.id);
      const title = projectTitle(value);
      if (id && title && !seen.has(id)) {
        seen.add(id);
        rows.push(Object.freeze({
          id,
          title,
          path: '/g/' + id + '/project',
          active: false
        }));
      }
      Object.keys(value).slice(0, 30).forEach((key) => visit(value[key], depth + 1));
    }
    visit(payload, 0);
    return rows;
  }

  function trimMap(values, maximum) {
    while (values.size > maximum) values.delete(values.keys().next().value);
  }

  function notify(emitListener) {
    revision += 1;
    if (emitListener !== false && typeof listener === 'function') listener();
  }

  function accept(metadata, text, emitListener) {
    const payload = parsePayload(text);
    if (!payload || !metadata) return false;
    let changed = false;
    if (metadata.family === 'projects') {
      collectProjects(payload).forEach((row) => {
        projects.set(row.id, row);
        changed = true;
      });
      trimMap(projects, MAX_PROJECTS);
    } else {
      if (!candidateArrays(payload).length) return false;
      collectConversations(payload, metadata.projectId).forEach((row) => {
        conversations.set(row.id, row);
        changed = true;
      });
      trimMap(conversations, MAX_CONVERSATIONS);
    }
    if (changed) notify(emitListener);
    return true;
  }

  function replaceProjectConversations(projectId, text) {
    const payload = parsePayload(text);
    if (!payload || !candidateArrays(payload).length) return false;
    const rows = collectConversations(payload, projectId);
    Array.from(conversations.entries()).forEach(([id, row]) => {
      if (row && row.projectId === projectId) conversations.delete(id);
    });
    rows.forEach((row) => conversations.set(row.id, row));
    trimMap(conversations, MAX_CONVERSATIONS);
    return true;
  }

  function inspectFetchResponse(metadata, response) {
    if (!metadata || !response || response.status < 200 || response.status >= 300 ||
        typeof response.clone !== 'function') return;
    Promise.resolve().then(() => response.clone().text())
      .then((text) => accept(metadata, text))
      .catch(() => {});
  }

  if (originalFetch) {
    window.fetch = function (input, init) {
      const metadata = requestMetadata(input, init);
      const result = originalFetch(input, init);
      if (metadata) Promise.resolve(result).then((response) => inspectFetchResponse(metadata, response))
        .catch(() => {});
      return result;
    };
  }

  if (originalOpen && originalSend) {
    xhrPrototype.open = function (method, rawUrl) {
      xhrMetadata.set(this, requestMetadata(rawUrl, { method }));
      return originalOpen.apply(this, arguments);
    };
    xhrPrototype.send = function () {
      const metadata = xhrMetadata.get(this);
      if (metadata && typeof this.addEventListener === 'function') {
        this.addEventListener('load', () => {
          if (this.status < 200 || this.status >= 300) return;
          let text = '';
          try { text = String(this.responseText || ''); }
          catch (_) { return; }
          accept(metadata, text);
        }, { once: true });
      }
      return originalSend.apply(this, arguments);
    };
  }

  function refreshProject(rawProjectId) {
    const projectId = cleanText(rawProjectId);
    if (!originalFetch || !SAFE_PROJECT_ID.test(projectId)) return Promise.resolve(false);
    const active = projectRefreshes.get(projectId);
    if (active) return active;
    const fetchResult = Promise.resolve().then(() => originalFetch(
      '/backend-api/gizmos/' + encodeURIComponent(projectId) + '/conversations',
      { method: 'GET', credentials: 'same-origin', cache: 'no-store' }
    )).then((response) => {
      if (!response || response.status < 200 || response.status >= 300 ||
          typeof response.text !== 'function') return false;
      return response.text().then((text) => replaceProjectConversations(projectId, text));
    }).catch(() => false);
    const request = new Promise((resolve) => {
      let settled = false;
      const timeout = window.setTimeout(() => {
        if (settled) return;
        settled = true;
        resolve(false);
      }, PROJECT_REFRESH_TIMEOUT_MS);
      fetchResult.then((value) => {
        if (settled) return;
        settled = true;
        window.clearTimeout(timeout);
        resolve(value);
      });
    }).finally(() => {
      if (projectRefreshes.get(projectId) === request) projectRefreshes.delete(projectId);
    });
    projectRefreshes.set(projectId, request);
    return request;
  }

  function snapshot() {
    const currentPath = location.pathname;
    const projectRows = Array.from(projects.values()).map((row) => Object.assign({}, row, {
      active: currentPath.startsWith('/g/' + row.id)
    }));
    const projectById = new Map(projectRows.map((row) => [row.id, row]));
    const conversationRows = Array.from(conversations.values()).map((row) => {
      const project = row.projectId && projectById.get(row.projectId);
      return Object.assign({}, row, {
        active: row.path === currentPath,
        projectTitle: project ? project.title : null
      });
    });
    return Object.freeze({
      version: 1,
      revision,
      conversations: conversationRows,
      projects: projectRows,
      complete: false
    });
  }

  window.__elonChatGptPrivateConversationDirectory = Object.freeze({
    version: 6,
    snapshot,
    refreshProject,
    acceptConversationMembership,
    acceptPinnedState,
    setListener: (value) => { listener = typeof value === 'function' ? value : null; }
  });
})();
