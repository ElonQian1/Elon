(function () {
  'use strict';

  if (window.__elonChatGptConversationDirectoryRequests) return;

  const PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  const CONVERSATION_PATH = /^\/(?:c\/[A-Za-z0-9_-]{1,160}|g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/[A-Za-z0-9_-]{1,160})$/;

  function create(dependencies) {
    const conversationAdapter = dependencies.conversationAdapter;
    const privateDirectory = dependencies.privateDirectory;
    const privateTransport = dependencies.privateTransport;
    const emitEvent = dependencies.emitEvent;
    const optional = dependencies.optional;
    const lastSnapshots = new Map();
    let generation = 0;

    function emitSnapshot(requestedProjectId, scopedComplete) {
      if (!privateDirectory || typeof privateDirectory.snapshot !== 'function') return;
      const value = optional(null, () => privateDirectory.snapshot());
      if (!value || !Array.isArray(value.conversations) || !Array.isArray(value.projects) ||
          (!value.conversations.length && !value.projects.length)) return;
      const projectId = PROJECT_ID.test(String(requestedProjectId || ''))
        ? String(requestedProjectId)
        : '';
      const conversations = projectId
        ? value.conversations.filter((item) => item && item.projectId === projectId)
        : value.conversations;
      const complete = Boolean(projectId && scopedComplete === true);
      const scopeKey = projectId || 'global';
      const fingerprint = JSON.stringify({ conversations, projects: value.projects, complete });
      if (fingerprint === lastSnapshots.get(scopeKey)) return;
      lastSnapshots.set(scopeKey, fingerprint);
      emitEvent({
        type: 'conversation_snapshot',
        conversations,
        projects: value.projects,
        scopeProjectId: projectId || null,
        collection: {
          scrollerFound: false,
          scrolled: false,
          scrollRestored: true,
          reachedEnd: false,
          truncated: conversations.length >= 200,
          timedOut: false,
          observedCount: conversations.length,
          steps: 0,
          complete,
          source: 'official_private',
          officialLoadState: 'ready'
        }
      });
    }

    function cancel() {
      generation += 1;
      if (conversationAdapter && typeof conversationAdapter.cancelDirectoryWork === 'function') {
        conversationAdapter.cancelDirectoryWork();
      }
    }

    function requestList(command, respond) {
      const requestGeneration = ++generation;
      const projectId = String(command && command.projectScopeId || '').trim();
      const current = () => requestGeneration === generation;
      const fallback = () => {
        if (current()) conversationAdapter.requestList(command, emitEvent, respond);
      };
      if (!PROJECT_ID.test(projectId) || !privateDirectory ||
          typeof privateDirectory.refreshProject !== 'function') {
        fallback();
        return;
      }
      Promise.resolve(privateDirectory.refreshProject(projectId)).then((refreshed) => {
        if (!current()) return;
        if (!refreshed) return fallback();
        emitSnapshot(projectId, true);
        respond('list_conversations', true, '');
      }).catch(fallback);
    }

    function probeMembership(command, respond) {
      const path = String(command && command.value || '').trim();
      const projectId = String(command && command.projectScopeId || '').trim();
      if (!CONVERSATION_PATH.test(path) || !PROJECT_ID.test(projectId) ||
          !privateTransport || typeof privateTransport.probeConversationProject !== 'function') {
        return false;
      }
      return privateTransport.probeConversationProject(path, projectId, (matched) => {
        if (matched) emitSnapshot(projectId);
        respond(
          'probe_conversation_project',
          matched === true,
          matched ? '' : 'membership_unconfirmed'
        );
      });
    }

    function installListener() {
      if (!privateDirectory || typeof privateDirectory.setListener !== 'function') return;
      privateDirectory.setListener(() => emitSnapshot(null));
      emitSnapshot(null);
    }

    return Object.freeze({ cancel, emitSnapshot, installListener, probeMembership, requestList });
  }

  window.__elonChatGptConversationDirectoryRequests = Object.freeze({ create });
})();
