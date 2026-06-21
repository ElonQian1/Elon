// server/src/assets/pc_app_task_snapshots.js
(function () {
  function create(deps) {
    const state = deps.state;
    const api = deps.api;
    const clean = deps.clean;
    const sameId = deps.sameId;
    const devTasks = deps.devTasks;
    const snapshots = new Map();
    const cursors = new Map();
    let timer = 0;

    function clear() {
      if (timer) {
        clearTimeout(timer);
        timer = 0;
      }
    }

    function contextExtras() {
      return { snapshots };
    }

    function schedule(messages, scope, context) {
      clear();
      if (scope !== 'project' || !isActiveDevChannel()) return false;
      if (!devTasks || typeof devTasks.openTaskIds !== 'function') return false;
      const openIds = devTasks.openTaskIds(messages, context);
      const taskId = openIds.length ? clean(openIds[openIds.length - 1]) : '';
      if (!taskId) return false;
      scheduleTask(state.activeProjectId, state.activeChannelId, taskId, 4500);
      return true;
    }

    function scheduleTask(projectId, channelId, taskId, delayMs) {
      clear();
      timer = setTimeout(() => {
        timer = 0;
        return pollTaskSnapshot(projectId, channelId, taskId).catch(reportError);
      }, delayMs || 4500);
    }

    async function pollTaskSnapshot(projectId, channelId, taskId) {
      if (!stillViewing(projectId, channelId)) return;
      const key = snapshotKey(projectId, channelId, taskId);
      const since = cursors.get(key) || 0;
      const data = await api(`/api/projects/${encodeURIComponent(projectId)}/channels/${encodeURIComponent(channelId)}/ai-tasks/${encodeURIComponent(taskId)}/snapshot?since=${encodeURIComponent(String(since))}&limit=200`);
      const snapshot = normalizeSnapshot(data);
      if (!snapshot) {
        if (stillViewing(projectId, channelId)) scheduleTask(projectId, channelId, taskId, 8000);
        return;
      }
      const previous = snapshots.get(taskId) || null;
      snapshots.set(taskId, snapshot);
      const nextSeq = lastSeqOf(snapshot);
      if (nextSeq > since) cursors.set(key, nextSeq);
      if (shouldRenderSnapshot(previous, snapshot, since)) {
        const messages = Array.isArray(snapshot.messages) ? snapshot.messages : [];
        if (messages.length && typeof deps.renderMessages === 'function') {
          deps.renderMessages(messages, 'project');
          return;
        }
        if (typeof deps.refreshActiveChannel === 'function') {
          await deps.refreshActiveChannel();
          return;
        }
      }
      if (!taskIsTerminal(snapshot.task) && stillViewing(projectId, channelId)) {
        scheduleTask(projectId, channelId, taskId, 4500);
      }
    }

    function normalizeSnapshot(data) {
      if (!data || !data.task) return null;
      return {
        task: data.task,
        messages: Array.isArray(data.messages) ? data.messages : [],
        events: Array.isArray(data.events) ? data.events : [],
        last_event_seq: Number(data.last_event_seq || data.lastEventSeq || 0),
        attach: data.attach || null
      };
    }

    function shouldRenderSnapshot(previous, snapshot, since) {
      if (!previous) return true;
      if (lastSeqOf(snapshot) > since) return true;
      if (taskStatus(previous.task) !== taskStatus(snapshot.task)) return true;
      return attachStatus(previous.attach) !== attachStatus(snapshot.attach);
    }

    function lastSeqOf(snapshot) {
      const seq = Number(snapshot && (snapshot.last_event_seq || snapshot.lastEventSeq || 0));
      return Number.isFinite(seq) ? seq : 0;
    }

    function taskStatus(task) {
      return clean(task && task.status).toLowerCase();
    }

    function attachStatus(attach) {
      return clean(attach && attach.status).toLowerCase();
    }

    function taskIsTerminal(task) {
      return ['done', 'failed', 'canceled', 'cancelled', 'interrupted'].includes(taskStatus(task));
    }

    function isActiveDevChannel() {
      return state.activeKind === 'project'
        && !!state.activeProjectId
        && !!state.activeChannelId
        && clean(state.activeChannelKind).toLowerCase() === 'ai_development';
    }

    function stillViewing(projectId, channelId) {
      return isActiveDevChannel()
        && sameId(state.activeProjectId, projectId)
        && sameId(state.activeChannelId, channelId);
    }

    function snapshotKey(projectId, channelId, taskId) {
      return `${clean(projectId)}:${clean(channelId)}:${clean(taskId)}`;
    }

    function reportError(error) {
      if (typeof deps.logError === 'function') deps.logError(error);
      else if (typeof console !== 'undefined' && console.warn) console.warn(error);
    }

    return { clear, schedule, contextExtras };
  }

  window.ElonPcTaskSnapshots = { create };
})();
