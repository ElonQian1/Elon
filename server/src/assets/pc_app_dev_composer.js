(function () {
  function create(deps) {
    const state = deps.state;
    const els = deps.els;
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const openSettings = deps.openSettings;
    const selectNode = deps.selectNode;
    const openModelPicker = deps.openModelPicker;
    let bar = null;
    let busy = false;

    function render() {
      ensureBar();
      if (!bar) return;
      const project = activeProject();
      const visible = state.activeKind === 'project'
        && state.activeChannelKind === 'ai_development'
        && !!project;
      bar.hidden = !visible;
      if (!visible) return;

      const info = buildInfo(project);
      bar.className = `dev-composer-bar ${escapeHtml(info.tone)}`;
      bar.innerHTML = `
        <div class="dev-composer-main">
          <span class="dev-composer-kicker">${busy ? '任务提交中' : 'AI 开发任务'}</span>
          <strong>${escapeHtml(info.routeLabel)}</strong>
          <span>${escapeHtml(info.permissionLabel)}</span>
          <span title="${escapeHtml(info.workspace)}">${escapeHtml(compactPath(info.workspace))}</span>
        </div>
        <div class="dev-composer-actions">
          <button type="button" data-dev-composer-action="model">模型</button>
          <button type="button" data-dev-composer-action="node">节点</button>
          <button type="button" data-dev-composer-action="settings">${escapeHtml(info.settingsLabel)}</button>
        </div>
      `;
      bindActions();
    }

    function setBusy(nextBusy) {
      busy = !!nextBusy;
      render();
    }

    function ensureBar() {
      if (bar || !els.composer || !els.composer.parentElement) return;
      bar = document.createElement('section');
      bar.className = 'dev-composer-bar';
      bar.hidden = true;
      els.composer.parentElement.insertBefore(bar, els.composer);
    }

    function bindActions() {
      bar.querySelectorAll('[data-dev-composer-action]').forEach((button) => {
        button.addEventListener('click', () => {
          const action = button.dataset.devComposerAction;
          if (action === 'model') {
            openModelPicker();
            return;
          }
          if (action === 'node') {
            selectNode();
            return;
          }
          openSettings('workbench');
        });
      });
    }

    function buildInfo(project) {
      const permission = runtimePermission(project);
      const nodeId = clean(project.node_id || project.nodeId || project.agent_id || project.agentId);
      const node = findNode(nodeId);
      const route = routeInfo(node);
      return {
        tone: route.ready ? (permission === 'full_access' ? 'full-access' : 'ready') : 'warning',
        routeLabel: route.label,
        permissionLabel: permission === 'full_access' ? '完全访问' : '仅项目内写入',
        workspace: clean(project.workspace_path || project.workspacePath) || '未绑定本机目录',
        settingsLabel: permission === 'full_access' ? '权限' : '设置'
      };
    }

    function routeInfo(node) {
      if (!node) return { ready: false, label: '未绑定可用节点' };
      if (!node.online) return { ready: false, label: '节点离线' };
      const clis = normalizedClis(node);
      const routeA = ['codex', 'copilot', 'claude', 'gemini'].find((cli) => clis.includes(cli));
      if (node.route_a_ready && routeA) return { ready: true, label: `Route A · ${routeA}` };
      if (node.api_runtime_ready) return { ready: true, label: 'Route B · 本机 API runtime' };
      if (node.server_runtime_ready) return { ready: true, label: 'Route C · 服务器模型' };
      if (routeA) return { ready: true, label: `本机 CLI · ${routeA}` };
      return { ready: false, label: '运行时未就绪' };
    }

    function activeProject() {
      return (state.projects || []).find((project) => {
        return String(project.id || '') === String(state.activeProjectId || '');
      }) || null;
    }

    function findNode(nodeId) {
      if (!nodeId) return null;
      return (state.nodes || []).find((node) => {
        return sameId(node.node_id, nodeId) || sameId(node.agent_id, nodeId);
      }) || null;
    }

    function normalizedClis(node) {
      return (node && node.allowed_clis || []).map((item) => clean(item).toLowerCase()).filter(Boolean);
    }

    function runtimePermission(project) {
      return clean(project.runtime_permission || project.runtimePermission) === 'full_access'
        ? 'full_access'
        : 'project_write';
    }

    function compactPath(path) {
      const value = clean(path);
      if (!value || value === '未绑定本机目录') return value || '未绑定本机目录';
      const normalized = value.replace(/\//g, '\\');
      const parts = normalized.split('\\').filter(Boolean);
      if (parts.length <= 3) return normalized;
      const drive = normalized.match(/^[A-Za-z]:/) ? normalized.slice(0, 2) : parts[0];
      return `${drive}\\...\\${parts.slice(-2).join('\\')}`;
    }

    function sameId(left, right) {
      return String(left || '') === String(right || '');
    }

    return { render, setBusy };
  }

  window.ElonPcDevComposer = { create };
})();
