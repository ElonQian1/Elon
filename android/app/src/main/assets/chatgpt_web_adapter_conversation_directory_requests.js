(function () {
  'use strict';

  if (Number(window.__elonChatGptConversationDirectoryRequests?.version) >= 4) return;

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
          (!value.conversations.length && !value.projects.length && !value.removedConversationIds?.length)) return;
      const projectId = PROJECT_ID.test(String(requestedProjectId || ''))
        ? String(requestedProjectId)
        : '';
      const conversations = projectId
        ? value.conversations.filter((item) => item && item.projectId === projectId)
        : value.conversations;
      const removedConversationIds = Array.isArray(value.removedConversationIds)
        ? value.removedConversationIds.slice(0, 200)
        : [];
      const deletedConversationIds = Array.isArray(value.deletedConversationIds)
        ? value.deletedConversationIds.slice(0, 200) : [];
      const complete = Boolean(projectId && scopedComplete === true);
      const scopeKey = projectId || 'global';
      const fingerprint = JSON.stringify({
        conversations,
        projects: value.projects,
        removedConversationIds,
        deletedConversationIds,
        complete
      });
      if (fingerprint === lastSnapshots.get(scopeKey)) return;
      lastSnapshots.set(scopeKey, fingerprint);
      emitEvent({
        type: 'conversation_snapshot',
        conversations,
        projects: value.projects,
        removedConversationIds,
        deletedConversationIds,
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

    function handleCommand(command, respond) {
      const action = command.action;
      if (action === 'cancel_conversation_directory') {
        cancel();
        respond(action, true, '');
      } else if (action === 'probe_conversation_project') {
        if (!probeMembership(command, respond)) respond(action, false, 'membership_probe_unavailable');
      } else if (action === 'list_conversation_files') {
        if (!privateTransport || typeof privateTransport.listConversationFiles !== 'function') {
          respond(action, false, 'files_not_ready');
        } else {
          privateTransport.listConversationFiles(command.value, command.requestId, emitEvent, respond);
        }
      } else if (action === 'download_conversation_file') {
        const downloads = window.__elonChatGptPrivateFileDownload;
        if (!downloads) respond(action, false, 'download_not_ready');
        else downloads.start(command.value, respond);
      } else return false;
      return true;
    }

    return Object.freeze({ cancel, emitSnapshot, handleCommand, installListener, probeMembership, requestList });
  }

  window.__elonChatGptConversationDirectoryRequests = Object.freeze({ version: 4, create });
})();
