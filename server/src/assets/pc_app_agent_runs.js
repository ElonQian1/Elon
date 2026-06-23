// server/src/assets/pc_app_agent_runs.js
(function () {
  function create(deps) {
    const state = deps.state;
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const localNodeApi = deps.localNodeApi;
    const activeProject = deps.activeProject;
    const sameId = deps.sameId || ((left, right) => String(left || '') === String(right || ''));
    const cache = new Map();
    const loading = new Set();
    const failures = new Set();
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
      const runs = Array.isArray(cached.runs) ? cached.runs : [];
      if (!runs.length) {
        return sectionShell('本机 Agent 运行', statusLine('暂无本机运行记录', 'muted'), refreshButton());
      }
      const items = runs.slice(0, 4).map(renderRun).join('');
      return sectionShell('本机 Agent 运行', items, refreshButton());
    }

    function bindActions(root, messages, scope) {
      if (!root) return;
      root.querySelectorAll('[data-agent-run-action="refresh"]').forEach((button) => {
        button.addEventListener('click', () => loadNow(messages, scope, { force: true }).catch(reportError));
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
        logDir: clean(data && (data.log_dir || data.logDir)),
        workspacePath: clean(data && (data.workspace_path || data.workspacePath)),
        loadedAt: Date.now()
      };
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

    function statusMeta(status) {
      const value = clean(status).toLowerCase();
      if (value === 'completed' || value === 'done') return { tone: 'done', label: '完成' };
      if (value === 'failed' || value === 'error') return { tone: 'failed', label: '失败' };
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
