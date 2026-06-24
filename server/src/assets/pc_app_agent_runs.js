// server/src/assets/pc_app_agent_runs.js
(function () {
  function create(deps) {
    const state = deps.state;
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const localNodeApi = deps.localNodeApi;
    const activeProject = deps.activeProject;
    const sameId = deps.sameId || ((left, right) => String(left || '') === String(right || ''));
    const draftContinuation = deps.draftContinuation;
    const confirmCancel = deps.confirm || ((message) => {
      if (typeof window !== 'undefined' && typeof window.confirm === 'function') return window.confirm(message);
      return true;
    });
    const cache = new Map();
    const loading = new Set();
    const failures = new Set();
    const continuationDrafts = new Map();
    let timer = 0;

    function clear() {
      if (timer) {
        clearTimeout(timer);
        timer = 0;
      }
    }

    function renderSection() {
      if (!isActiveDevChannel()) return '';
      const project = activeProject ? activeProject() : null;
      const workspacePath = projectWorkspacePath(project);
      if (!workspacePath) {
        return sectionShell('本机 Agent 运行', statusLine('未绑定本机目录', 'warning'), '');
      }
      const key = cacheKey(project, workspacePath);
      const cached = cache.get(key);
      if (!cached) {
        return sectionShell('本机 Agent 运行', statusLine('读取中', 'running'), '');
      }
      if (cached.error) {
        return sectionShell('本机 Agent 运行', statusLine(cached.error, 'failed'), refreshButton());
      }
      continuationDrafts.clear();
      const recoveryEntry = cached.recoveryEntry || null;
      const controls = Array.isArray(cached.activeControls) ? cached.activeControls : [];
      const recentTasks = Array.isArray(cached.recentTasks) ? cached.recentTasks : [];
      const runs = Array.isArray(cached.runs) ? cached.runs : [];
      if (!recoveryEntry && !controls.length && !recentTasks.length && !runs.length) {
        return sectionShell('本机 Agent 运行', statusLine('暂无本机运行记录', 'muted'), refreshButton());
      }
      const recoveryTaskId = recoveryEntryTaskId(recoveryEntry);
      const recoveryItem = recoveryEntry ? renderRecoveryEntry(recoveryEntry, cached) : '';
      const controlItems = controls
        .filter((control) => controlTaskId(control) !== recoveryTaskId)
        .map(renderControl)
        .join('');
      const taskItems = recentTasks
        .filter((task) => taskResumeId(task) !== recoveryTaskId)
        .slice(0, controls.length ? 2 : 3)
        .map((task) => renderRecentTask(task, cached))
        .join('');
      const runLimit = controls.length || recentTasks.length ? 2 : 4;
      const runItems = runs.slice(0, runLimit).map(renderRun).join('');
      const items = `${recoveryItem}${controlItems}${taskItems}${runItems}`;
      return sectionShell('本机 Agent 运行', items, refreshButton());
    }

    function bindActions(root, messages, scope) {
      if (!root) return;
      root.querySelectorAll('[data-agent-run-action="refresh"]').forEach((button) => {
        button.addEventListener('click', () => loadNow(messages, scope, { force: true }).catch(reportError));
      });
      root.querySelectorAll('[data-agent-run-action="cancel"]').forEach((button) => {
        button.addEventListener('click', () => {
          const taskId = clean(button.dataset && button.dataset.taskId);
          if (!taskId) return;
          button.disabled = true;
          cancelTask(taskId, messages, scope).catch((error) => {
            button.disabled = false;
            reportError(error);
          });
        });
      });
      root.querySelectorAll('[data-agent-run-action="continue"]').forEach((button) => {
        button.addEventListener('click', () => {
          const taskId = clean(button.dataset && button.dataset.taskId);
          const draft = continuationDrafts.get(taskId);
          if (!draft || typeof draftContinuation !== 'function') return;
          draftContinuation(draft);
        });
      });
    }

    function schedule(messages, scope) {
      clear();
      if (scope !== 'project' || !isActiveDevChannel()) return false;
      const project = activeProject ? activeProject() : null;
      const workspacePath = projectWorkspacePath(project);
      if (!workspacePath) return false;
      const key = cacheKey(project, workspacePath);
      const cached = cache.get(key);
      const delay = shouldPoll(cached) ? 4500 : (cached ? 0 : 80);
      if (cached && delay === 0) return false;
      timer = setTimeout(() => {
        timer = 0;
        return loadNow(messages, scope, { force: !cached }).catch(reportError);
      }, delay || 80);
      return true;
    }

    async function loadNow(messages, scope, options) {
      if (scope !== 'project' || !isActiveDevChannel()) return;
      const project = activeProject ? activeProject() : null;
      const workspacePath = projectWorkspacePath(project);
      if (!workspacePath) return;
      const key = cacheKey(project, workspacePath);
      if (loading.has(key)) return;
      if (!options || !options.force) {
        const cached = cache.get(key);
        if (cached && !shouldPoll(cached)) return;
      }
      loading.add(key);
      try {
        const data = await localNodeApi('/api/project-agent-runs', {
          method: 'POST',
          cache: 'no-store',
          body: JSON.stringify({
            workspace_path: workspacePath,
            limit: 8,
            event_limit: 8
          })
        });
        cache.set(key, normalizeResponse(data));
        failures.delete(key);
      } catch (error) {
        if (!failures.has(key)) reportError(error);
        failures.add(key);
        cache.set(key, {
          error: clean(error && error.message) || '无法读取本机运行记录',
          runs: []
        });
      } finally {
        loading.delete(key);
      }
      if (sameActiveProject(project)) rerender(messages, scope);
    }

    function normalizeResponse(data) {
      return {
        runs: Array.isArray(data && data.runs) ? data.runs : [],
        recoveryEntry: normalizeRecoveryEntry(data && (data.recovery_entry || data.recoveryEntry)),
        activeControls: Array.isArray(data && (data.active_controls || data.activeControls))
          ? (data.active_controls || data.activeControls)
          : [],
        recentTasks: Array.isArray(data && (data.recent_tasks || data.recentTasks))
          ? (data.recent_tasks || data.recentTasks)
          : [],
        logDir: clean(data && (data.log_dir || data.logDir)),
        workspacePath: clean(data && (data.workspace_path || data.workspacePath)),
        loadedAt: Date.now()
      };
    }

    function normalizeRecoveryEntry(entry) {
      if (!entry || typeof entry !== 'object') return null;
      const taskId = recoveryEntryTaskId(entry);
      if (!taskId) return null;
      return entry;
    }

    async function cancelTask(taskId, messages, scope) {
      if (!confirmCancel(`停止本机 Agent 运行 ${shortRunId(taskId)}？`)) return;
      await localNodeApi(`/api/task-journal/${encodeURIComponent(taskId)}/cancel`, {
        method: 'POST',
        cache: 'no-store'
      });
      await loadNow(messages, scope, { force: true });
    }

    function renderControl(control) {
      const taskId = controlTaskId(control);
      const route = clean(control && control.route) || 'local-runtime';
      const cliName = clean(control && (control.cli_name || control.cliName)) || 'agent';
      const permission = clean(control && (control.runtime_permission || control.runtimePermission));
      const pid = clean(control && (control.os_pid || control.osPid));
      const canCancel = taskId && control && control.can_cancel !== false && control.canCancel !== false;
      const meta = [
        cliName,
        route,
        permission,
        pid ? `PID ${pid}` : ''
      ].filter(Boolean).join(' · ');
      return `<article class="agent-run-item running agent-run-control">
        <div class="agent-run-main">
          <span class="agent-run-status">运行中</span>
          <strong>${escapeHtml(shortRunId(taskId || route))}</strong>
          <small>${escapeHtml(meta || '本机控制句柄')}</small>
        </div>
        ${canCancel ? `<button class="agent-run-stop" type="button" data-agent-run-action="cancel" data-task-id="${escapeHtml(taskId)}">停止</button>` : ''}
      </article>`;
    }

    function renderRecoveryEntry(entry, context) {
      const taskId = recoveryEntryTaskId(entry);
      const action = clean(entry && (entry.recommended_action || entry.recommendedAction)).toLowerCase();
      const status = statusMeta(entry && (entry.status || (action === 'wait_or_cancel' ? 'running' : '')));
      const kind = clean(entry && entry.kind);
      const cliName = clean(entry && (entry.cli_name || entry.cliName)) || 'agent';
      const route = clean(entry && entry.route);
      const permission = clean(entry && (entry.runtime_permission || entry.runtimePermission));
      const updated = clean(entry && (entry.updated_at || entry.updatedAt || entry.updated_at_ms || entry.updatedAtMs));
      const reason = clean(entry && entry.reason);
      const canCancel = taskId && (entry.can_cancel === true || entry.canCancel === true);
      const canContinue = taskId && (entry.can_continue === true || entry.canContinue === true || action === 'continue_from_snapshot');
      const draftTask = canContinue ? recoveryTaskForDraft(entry, context) : null;
      const draft = draftTask ? continuationDraft(draftTask, context) : '';
      if (canContinue && taskId && draft) continuationDrafts.set(taskId, draft);
      const approval = toolApprovalRecoveryView(draftTask || taskById(context, taskId));
      const meta = [
        cliName,
        route,
        permission,
        recoveryActionLabel(action, kind),
        updated
      ].filter(Boolean).join(' · ');
      return `<article class="agent-run-item ${escapeHtml(status.tone)} agent-run-control agent-run-recovery">
        <div class="agent-run-main">
          <span class="agent-run-status">推荐恢复</span>
          <strong>${escapeHtml(shortRunId(taskId || kind || cliName))}</strong>
          <small>${escapeHtml(meta || recoveryKindLabel(kind))}</small>
          ${reason ? `<p>${escapeHtml(reason)}</p>` : ''}
          ${approval ? toolApprovalRecoveryHtml(approval) : ''}
        </div>
        ${canContinue ? `<button class="agent-run-continue" type="button" data-agent-run-action="continue" data-task-id="${escapeHtml(taskId)}">继续</button>` : ''}
        ${!canContinue && canCancel ? `<button class="agent-run-stop" type="button" data-agent-run-action="cancel" data-task-id="${escapeHtml(taskId)}">停止</button>` : ''}
      </article>`;
    }

    function renderRecentTask(task, context) {
      const taskId = taskResumeId(task);
      const resume = task && task.resume ? task.resume : null;
      const status = statusMeta(task && (task.status || (resume && resume.status)));
      const cliName = clean(task && (task.cli_name || task.cliName)) || 'agent';
      const route = clean(task && task.route) || clean(resume && resume.continue_mode);
      const updated = clean(task && (task.updated_at || task.updatedAt || task.updated_at_ms || task.updatedAtMs));
      const strategy = clean(resume && resume.strategy && resume.strategy.label);
      const canContinue = taskCanContinue(task);
      const draft = continuationDraft(task, context);
      if (canContinue && taskId && draft) continuationDrafts.set(taskId, draft);
      const approval = toolApprovalRecoveryView(task);
      const meta = [
        cliName,
        route,
        strategy,
        updated
      ].filter(Boolean).join(' · ');
      return `<article class="agent-run-item ${escapeHtml(status.tone)} agent-run-control">
        <div class="agent-run-main">
          <span class="agent-run-status">${escapeHtml(status.label)}</span>
          <strong>${escapeHtml(shortRunId(taskId || cliName))}</strong>
          <small>${escapeHtml(meta || '本机任务快照')}</small>
          ${resumeHint(resume) ? `<p>${escapeHtml(resumeHint(resume))}</p>` : ''}
          ${approval ? toolApprovalRecoveryHtml(approval) : ''}
        </div>
        ${canContinue ? `<button class="agent-run-continue" type="button" data-agent-run-action="continue" data-task-id="${escapeHtml(taskId)}">继续</button>` : ''}
      </article>`;
    }

    function renderRun(run) {
      const status = statusMeta(run && run.status);
      const mode = clean(run && run.mode) || 'runtime';
      const runId = clean(run && (run.run_id || run.runId || run.file_name || run.fileName));
      const tools = Array.isArray(run && run.tool_names)
        ? run.tool_names.map(clean).filter(Boolean).slice(0, 5)
        : [];
      const meta = [
        mode,
        Number(run && (run.turn_count || run.turnCount || 0)) > 0 ? `${Number(run.turn_count || run.turnCount)} 轮` : '',
        Number(run && (run.tool_count || run.toolCount || 0)) > 0 ? `${Number(run.tool_count || run.toolCount)} 个工具` : '',
        clean(run && (run.updated_at || run.updatedAt || run.started_at || run.startedAt))
      ].filter(Boolean).join(' · ');
      const error = clean(run && (run.last_error || run.lastError));
      const toolLine = tools.length ? `<div class="agent-run-tools">${tools.map((tool) => `<span>${escapeHtml(tool)}</span>`).join('')}</div>` : '';
      return `<article class="agent-run-item ${escapeHtml(status.tone)}">
        <div class="agent-run-main">
          <span class="agent-run-status">${escapeHtml(status.label)}</span>
          <strong>${escapeHtml(shortRunId(runId))}</strong>
          <small>${escapeHtml(meta || '本机运行')}</small>
          ${error ? `<p>${escapeHtml(error)}</p>` : ''}
          ${toolLine}
        </div>
      </article>`;
    }

    function statusLine(text, tone) {
      return `<div class="agent-run-empty ${escapeHtml(tone || '')}">${escapeHtml(text)}</div>`;
    }

    function sectionShell(title, body, actions) {
      return `<section class="agent-run-panel">
        <div class="agent-run-panel-head">
          <strong>${escapeHtml(title)}</strong>
          ${actions || ''}
        </div>
        <div class="agent-run-list">${body}</div>
      </section>`;
    }

    function refreshButton() {
      return '<button class="agent-run-refresh" type="button" data-agent-run-action="refresh">刷新</button>';
    }

    function shouldPoll(cached) {
      if (!cached || cached.error) return true;
      if ((cached.activeControls || []).length) return true;
      const recovery = cached.recoveryEntry || null;
      if (recovery && clean(recovery.status).toLowerCase() === 'running') return true;
      if ((cached.recentTasks || []).some((task) => clean(task && task.status).toLowerCase() === 'running')) return true;
      return (cached.runs || []).some((run) => clean(run && run.status).toLowerCase() === 'running');
    }

    function isActiveDevChannel() {
      return state.activeKind === 'project'
        && !!state.activeProjectId
        && !!state.activeChannelId
        && clean(state.activeChannelKind).toLowerCase() === 'ai_development';
    }

    function sameActiveProject(project) {
      return project && sameId(project.id, state.activeProjectId) && isActiveDevChannel();
    }

    function projectWorkspacePath(project) {
      return clean(project && (
        project.workspace_path || project.workspacePath ||
        project.storage_worktree_path || project.storageWorktreePath ||
        project.local_workspace_path || project.localWorkspacePath
      ));
    }

    function cacheKey(project, workspacePath) {
      return `${clean(project && project.id)}:${workspacePath}`;
    }

    function controlTaskId(control) {
      return clean(control && (
        control.task_id || control.taskId ||
        control.req_id || control.reqId ||
        control.run_handle_id || control.runHandleId
      ));
    }

    function taskResumeId(task) {
      return clean(task && (task.task_id || task.taskId || task.req_id || task.reqId));
    }

    function recoveryEntryTaskId(entry) {
      return clean(entry && (entry.task_id || entry.taskId || entry.req_id || entry.reqId));
    }

    function recoveryKindLabel(kind) {
      if (kind === 'active_control') return '本机控制句柄';
      if (kind === 'snapshot_resume') return '本机任务快照';
      return '本机恢复入口';
    }

    function recoveryActionLabel(action, kind) {
      if (action === 'continue_from_snapshot') return '基于快照继续';
      if (action === 'wait_or_cancel') return '等待或停止';
      return recoveryKindLabel(kind);
    }

    function recoveryTaskForDraft(entry, context) {
      const taskId = recoveryEntryTaskId(entry);
      const recent = taskById(context, taskId);
      if (recent) return recent;
      return {
        task_id: taskId,
        cli_name: entry && (entry.cli_name || entry.cliName),
        route: entry && entry.route,
        cwd: entry && entry.cwd,
        runtime_permission: entry && (entry.runtime_permission || entry.runtimePermission),
        status: entry && entry.status,
        updated_at_ms: entry && (entry.updated_at_ms || entry.updatedAtMs),
        attach: { status: 'detached', source: 'local_journal' },
        resume: {
          status: entry && entry.status,
          can_replay_journal_events: true,
          can_stream_live_output: false,
          next_action: entry && (entry.recommended_action || entry.recommendedAction),
          reason: entry && entry.reason,
          strategy: { kind: 'snapshot_continue', label: '基于快照继续' }
        }
      };
    }

    function taskCanContinue(task) {
      const taskId = taskResumeId(task);
      const resume = task && task.resume ? task.resume : null;
      const action = clean(resume && resume.next_action).toLowerCase();
      return !!taskId && action === 'continue_from_snapshot';
    }

    function continuationDraft(task, context) {
      const taskId = taskResumeId(task);
      const resume = task && task.resume ? task.resume : null;
      const strategy = clean(resume && resume.strategy && resume.strategy.label);
      const reason = clean(resume && (resume.reason || (resume.strategy && resume.strategy.reason)));
      const approval = toolApprovalRecoveryView(task);
      const canCodex = resumeFlag(resume, 'can_resume_codex_session', 'canResumeCodexSession');
      const canReplay = resumeFlag(resume, 'can_replay_journal_events', 'canReplayJournalEvents');
      const cannotStream = resumeFlag(resume, 'can_stream_live_output', 'canStreamLiveOutput') === false;
      const cwd = clean(task && (task.cwd || task.workspace_path || task.workspacePath))
        || clean(context && context.workspacePath);
      const logDir = clean(context && context.logDir);
      const route = clean(task && task.route) || clean(resume && resume.continue_mode);
      const permission = clean(task && (task.runtime_permission || task.runtimePermission));
      const status = clean(task && (task.status || (resume && resume.status)));
      const updated = clean(task && (task.updated_at || task.updatedAt || task.updated_at_ms || task.updatedAtMs));
      const attach = task && task.attach ? task.attach : null;
      const attachStatus = clean(attach && attach.status);
      const attachSource = clean(attach && attach.source);
      const contextLines = [
        cwd ? `项目目录：${cwd}` : '',
        logDir ? `本机日志目录：${logDir}` : '',
        route ? `运行路线：${route}` : '',
        permission ? `运行权限：${permission}` : '',
        status ? `任务状态：${status}` : '',
        updated ? `最后更新：${updated}` : '',
        attachStatus || attachSource ? `现场状态：${[attachStatus, attachSource].filter(Boolean).join(' / ')}` : '',
        canReplay ? '本机 journal 事件可回放' : '',
        canCodex ? '本机 Codex session 已记录，节点会优先自动续接；不要让用户手动粘贴 session id。' : '',
        cannotStream ? '原 CLI 终端不可重接' : '',
        approval ? `工具审批恢复：${approval.label}${approval.detail ? `；${approval.detail}` : ''}` : ''
      ].filter(Boolean);
      return [
        '继续处理这个本机 Agent 任务。',
        `本机请求 ID：${taskId}`,
        contextLines.length ? `交接上下文：\n${contextLines.join('\n')}` : '',
        strategy ? `恢复方式：${strategy}` : '恢复方式：基于本机 journal 快照继续',
        reason ? `恢复原因：${reason}` : '',
        '不要假装已经接管原来的 CLI 窗口；请先检查当前项目工作区状态，读取本机日志/快照，再继续完成剩余开发。'
      ].filter(Boolean).join('\n\n');
    }

    function resumeFlag(resume, snakeKey, camelKey) {
      if (!resume) return undefined;
      if (typeof resume[snakeKey] === 'boolean') return resume[snakeKey];
      if (typeof resume[camelKey] === 'boolean') return resume[camelKey];
      return undefined;
    }

    function taskById(context, taskId) {
      const target = clean(taskId);
      if (!target || !Array.isArray(context && context.recentTasks)) return null;
      return context.recentTasks.find((task) => taskResumeId(task) === target) || null;
    }

    function toolApprovalRecoveryView(task) {
      const resume = task && task.resume ? task.resume : null;
      const recovery = resume && (resume.tool_approval_recovery || resume.toolApprovalRecovery);
      if (!recovery || typeof recovery !== 'object') return null;
      const status = clean(recovery.status).toLowerCase();
      const activeIds = Array.isArray(recovery.active_approval_ids || recovery.activeApprovalIds)
        ? (recovery.active_approval_ids || recovery.activeApprovalIds).map(clean).filter(Boolean)
        : [];
      const label = toolApprovalRecoveryLabel(status, activeIds.length);
      const reason = clean(recovery.reason);
      const action = clean(recovery.pending_after_restart_action || recovery.pendingAfterRestartAction);
      return {
        status,
        label,
        detail: reason || action,
        tone: toolApprovalRecoveryTone(status)
      };
    }

    function toolApprovalRecoveryLabel(status, activeCount) {
      if (status === 'active_waiter') return activeCount > 0 ? `审批可继续（${activeCount}）` : '审批可继续';
      if (status === 'no_active_waiter') return '审批仅回放';
      if (status === 'lost_after_restart') return '审批已失效';
      if (status === 'closed_by_terminal_task') return '审批已关闭';
      if (status === 'unavailable') return '审批状态不可用';
      return '审批状态';
    }

    function toolApprovalRecoveryTone(status) {
      if (status === 'active_waiter') return 'active';
      if (status === 'no_active_waiter') return 'warning';
      if (status === 'lost_after_restart' || status === 'closed_by_terminal_task') return 'failed';
      return 'muted';
    }

    function toolApprovalRecoveryHtml(approval) {
      if (!approval) return '';
      return `<div class="agent-run-approval ${escapeHtml(approval.tone)}">
        <span>${escapeHtml(approval.label)}</span>
        ${approval.detail ? `<em>${escapeHtml(approval.detail)}</em>` : ''}
      </div>`;
    }

    function resumeHint(resume) {
      if (!resume) return '';
      const parts = [];
      if (resume.can_replay_journal_events === true || resume.canReplayJournalEvents === true) parts.push('本机事件可回放');
      if (resume.can_resume_codex_session === true || resume.canResumeCodexSession === true) parts.push('Codex 会话可续接');
      if (resume.can_stream_live_output === false || resume.canStreamLiveOutput === false) parts.push('原 CLI 终端不可重接');
      return parts.join('，');
    }

    function statusMeta(status) {
      const value = clean(status).toLowerCase();
      if (value === 'completed' || value === 'done') return { tone: 'done', label: '完成' };
      if (value === 'failed' || value === 'error') return { tone: 'failed', label: '失败' };
      if (value === 'canceled' || value === 'cancelled' || value === 'cancel_requested' || value === 'interrupted' || value === 'stopped') return { tone: 'failed', label: '已停止' };
      if (value === 'running') return { tone: 'running', label: '运行中' };
      return { tone: 'muted', label: value || '未知' };
    }

    function shortRunId(value) {
      const text = clean(value) || 'agent run';
      if (text.length <= 24) return text;
      return `${text.slice(0, 12)}…${text.slice(-8)}`;
    }

    function rerender(messages, scope) {
      if (typeof deps.renderMessages === 'function') deps.renderMessages(messages || [], scope || 'project');
    }

    function reportError(error) {
      if (typeof deps.logError === 'function') deps.logError(error);
      else if (typeof console !== 'undefined' && console.warn) console.warn(error);
    }

    return { clear, renderSection, bindActions, schedule };
  }

  window.ElonPcAgentRuns = { create };
})();
