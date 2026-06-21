// server/src/assets/pc_app_task_snapshots.js
(function () {
  function create(deps) {
    const state = deps.state;
    const api = deps.api;
    const localNodeApi = deps.localNodeApi;
    const clean = deps.clean;
    const sameId = deps.sameId;
    const devTasks = deps.devTasks;
    const snapshots = new Map();
    const cursors = new Map();
    const localCursors = new Map();
    const localFailures = new Set();
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
      await mergeLocalJournal(snapshot, taskId);
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
        pc_req_id: clean(data.pc_req_id || data.pcReqId),
        attach: data.attach || null,
        resume: data.resume || null,
        local_journal: null
      };
    }

    async function mergeLocalJournal(snapshot, taskId) {
      if (typeof localNodeApi !== 'function' || !taskId) return;
      const journalTaskId = clean(snapshot && (snapshot.pc_req_id || snapshot.pcReqId));
      if (!journalTaskId) return;
      const since = localCursors.get(journalTaskId) || 0;
      try {
        const data = await localNodeApi(`/api/task-journal/${encodeURIComponent(journalTaskId)}?since=${encodeURIComponent(String(since))}&limit=100`, { cache: 'no-store' });
        const journal = normalizeLocalJournal(data);
        if (!journal) return;
        snapshot.local_journal = journal;
        snapshot.attach = mergeAttach(snapshot, journal);
        snapshot.resume = mergeResume(snapshot, journal);
        appendLocalJournalMessages(snapshot, journal, taskId);
        if (journal.last_event_seq > since) localCursors.set(journalTaskId, journal.last_event_seq);
        localFailures.delete(journalTaskId);
      } catch (error) {
        // 本机 journal 是恢复增强，不是云端 snapshot 的必要条件；节点未启动或版本旧时继续展示云端状态。
        if (!localFailures.has(journalTaskId) && typeof deps.logLocalError === 'function') {
          deps.logLocalError(error);
        }
        localFailures.add(journalTaskId);
      }
    }

    function normalizeLocalJournal(data) {
      if (!data || data.ok === false) return null;
      const lastSeq = Number(data.last_event_seq || data.lastEventSeq || 0);
      return {
        task_id: clean(data.task_id || data.taskId),
        record: data.record || null,
        events: Array.isArray(data.events) ? data.events : [],
        last_event_seq: Number.isFinite(lastSeq) ? lastSeq : 0,
        has_more: !!(data.has_more || data.hasMore),
        attach: data.attach || null,
        resume: data.resume || null
      };
    }

    function mergeAttach(snapshot, journal) {
      const localAttach = journal && journal.attach ? journal.attach : null;
      const localStatus = clean(localAttach && localAttach.status).toLowerCase();
      if (!localStatus || localStatus === 'missing') return snapshot.attach || null;
      if (localStatus === 'live') return Object.assign({}, snapshot.attach || {}, localAttach, { source: 'local_journal' });
      const cloudAttach = snapshot.attach || null;
      const cloudStatus = clean(cloudAttach && cloudAttach.status).toLowerCase();
      if (cloudStatus === 'live') return cloudAttach;
      if (taskIsTerminal(snapshot.task) && localStatus !== 'terminal') return cloudAttach;
      return Object.assign({}, cloudAttach || {}, localAttach, { source: 'local_journal' });
    }

    function mergeResume(snapshot, journal) {
      if (journal && journal.resume) return journal.resume;
      return snapshot.resume || null;
    }

    function appendLocalJournalMessages(snapshot, journal, taskId) {
      const messages = Array.isArray(snapshot.messages) ? snapshot.messages : [];
      const seen = new Set(messages.map(messageKey));
      (journal && Array.isArray(journal.events) ? journal.events : []).forEach((entry) => {
        const message = localJournalMessage(entry, taskId);
        if (!message) return;
        const key = messageKey(message);
        if (seen.has(key)) return;
        seen.add(key);
        messages.push(message);
      });
      snapshot.messages = messages;
    }

    function localJournalMessage(entry, taskId) {
      const event = entry && entry.event ? entry.event : null;
      const type = clean(event && event.type).toLowerCase();
      if (!['cli_chunk', 'tool_event'].includes(type)) return null;
      const inner = event && event.event ? event.event : null;
      const innerType = clean(inner && inner.type).toLowerCase();
      const content = innerType === 'tool_approval_required'
        ? localApprovalReplayText(inner)
        : clean(event.text || (inner ? JSON.stringify(inner) : ''));
      if (!content) return null;
      return {
        kind: 'ai_progress',
        task_id: taskId,
        content,
        local_journal_seq: Number(entry.seq || 0),
        local_journal: true
      };
    }

    function localApprovalReplayText(event) {
      const tool = clean(event && event.tool) || 'tool';
      const approvalId = clean(event && event.approval_id);
      return `[本机回放] ${tool} 等待审批${approvalId ? `（${approvalId}）` : ''}`;
    }

    function shouldRenderSnapshot(previous, snapshot, since) {
      if (!previous) return true;
      if (lastSeqOf(snapshot) > since) return true;
      if (taskStatus(previous.task) !== taskStatus(snapshot.task)) return true;
      return attachStatus(previous.attach) !== attachStatus(snapshot.attach)
        || resumeStatus(previous.resume) !== resumeStatus(snapshot.resume);
    }

    function lastSeqOf(snapshot) {
      const seq = Number(snapshot && (snapshot.last_event_seq || snapshot.lastEventSeq || 0));
      return Number.isFinite(seq) ? seq : 0;
    }

    function taskStatus(task) {
      return clean(task && task.status).toLowerCase();
    }

    function attachStatus(attach) {
      const status = clean(attach && attach.status).toLowerCase();
      const source = clean(attach && attach.source).toLowerCase();
      return `${status}:${source}`;
    }

    function messageKey(message) {
      return [
        clean(message && (message.kind || message.role || message.message_kind)).toLowerCase(),
        clean(message && (message.task_id || message.taskId)),
        clean(message && (message.content || message.text || message.message))
      ].join(':');
    }

    function resumeStatus(resume) {
      const status = clean(resume && resume.status).toLowerCase();
      const action = clean(resume && resume.next_action).toLowerCase();
      const strategy = clean(resume && resume.strategy && resume.strategy.kind).toLowerCase();
      return `${status}:${action}:${strategy}`;
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
