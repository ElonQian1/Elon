(function () {
  function createProjectReadiness(deps) {
    const { state, $, clean, escapeHtml, api, openSettings, selectNode, selectProject } = deps;

    function renderMemberPanel(project) {
      const info = buildReadiness(project);
      const permission = runtimePermission(project);
      const permissionTone = permission === 'full_access' ? 'warn' : 'ok';
      const nodeAction = info.node
        ? `<button class="dev-readiness-action" type="button" data-dev-readiness-action="node" data-node-id="${escapeHtml(info.nodeId)}">节点详情</button>`
        : `<button class="dev-readiness-action" type="button" data-dev-readiness-action="settings">绑定本机</button>`;

      return `
        <section class="dev-readiness ${escapeHtml(info.tone)}" data-dev-readiness-project="${escapeHtml(project.id || '')}">
          <div class="dev-readiness-head">
            <div>
              <span>开发就绪</span>
              <strong>${escapeHtml(info.title)}</strong>
            </div>
            <em>${escapeHtml(info.badge)}</em>
          </div>
          <div class="dev-readiness-grid">
            ${readinessRow('项目目录', info.workspace)}
            ${readinessRow('执行节点', info.nodeLabel)}
            ${readinessRow('运行路线', info.routeLabel)}
            ${readinessRow('可用 CLI', info.cliLabel)}
          </div>
          <label class="dev-readiness-permission ${permissionTone}">
            <span>AI 权限</span>
            <select data-dev-readiness-permission="${escapeHtml(project.id || '')}">
              <option value="project_write" ${permission === 'project_write' ? 'selected' : ''}>仅项目内写入</option>
              <option value="full_access" ${permission === 'full_access' ? 'selected' : ''}>完全访问</option>
            </select>
          </label>
          <div class="dev-readiness-actions">
            ${nodeAction}
            <button class="dev-readiness-action" type="button" data-dev-readiness-action="settings">项目设置</button>
            <button class="dev-readiness-action" type="button" data-dev-readiness-action="refresh" data-project-id="${escapeHtml(project.id || '')}">刷新</button>
          </div>
        </section>`;
    }

    function bindMemberPanel(project) {
      const panel = document.querySelector('[data-dev-readiness-project]');
      if (!panel) return;
      panel.querySelectorAll('[data-dev-readiness-action]').forEach((button) => {
        button.addEventListener('click', () => {
          const action = button.dataset.devReadinessAction;
          if (action === 'node') {
            const nodeId = clean(button.dataset.nodeId);
            if (nodeId) state.activeNodeId = nodeId;
            selectNode();
            return;
          }
          if (action === 'refresh') {
            const projectId = clean(button.dataset.projectId || project.id);
            if (projectId) selectProject(projectId);
            return;
          }
          openSettings('workbench');
        });
      });
      panel.querySelectorAll('[data-dev-readiness-permission]').forEach((select) => {
        select.addEventListener('change', () => updatePermission(project, select));
      });
    }

    async function updatePermission(project, select) {
      const projectId = clean(select.dataset.devReadinessPermission || project.id);
      const previous = runtimePermission(project);
      const next = normalizePermission(select.value);
      if (!projectId || next === previous) return;
      if (next === 'full_access') {
        const ok = window.confirm(`确认给项目「${projectTitle(project)}」开启完全访问？AI CLI 可能读取或修改项目目录外的本机文件和系统设置。`);
        if (!ok) {
          select.value = previous;
          return;
        }
      }
      select.disabled = true;
      try {
        const data = await api(`/api/projects/${encodeURIComponent(projectId)}/runtime-permission`, {
          method: 'PATCH',
          body: JSON.stringify({ mode: next, confirmFullAccess: next === 'full_access' })
        });
        project.runtime_permission = data.mode || next;
        await selectProject(projectId);
      } catch (error) {
        select.value = previous;
        window.alert(error.message || error);
      } finally {
        select.disabled = false;
      }
    }

    function buildReadiness(project) {
      const nodeId = clean(project.node_id || project.nodeId || project.agent_id || project.agentId);
      const node = findNode(nodeId);
      const workspace = clean(project.workspace_path || project.workspacePath) || '未绑定本机目录';
      if (!nodeId) {
        return {
          tone: 'missing',
          title: '未绑定本机',
          badge: '未就绪',
          nodeId,
          node: null,
          workspace,
          nodeLabel: '未选择 PC 节点',
          routeLabel: '先绑定本地项目',
          cliLabel: '未检查'
        };
      }
      if (!node) {
        return {
          tone: 'warning',
          title: '节点不可见',
          badge: '需检查',
          nodeId,
          node: null,
          workspace,
          nodeLabel: shortNodeId(nodeId),
          routeLabel: '等待节点上线或授权',
          cliLabel: '未上报'
        };
      }
      const route = routeInfo(node);
      const ready = !!node.online && route.ready;
      return {
        tone: ready ? 'ready' : 'warning',
        title: ready ? '可以开发' : '运行时未就绪',
        badge: ready ? 'Ready' : 'Check',
        nodeId,
        node,
        workspace,
        nodeLabel: nodeLabel(node),
        routeLabel: route.label,
        cliLabel: cliLabel(node)
      };
    }

    function routeInfo(node) {
      const clis = normalizedClis(node);
      const routeA = ['codex', 'copilot', 'claude', 'gemini'].find((cli) => clis.includes(cli));
      if (node.route_a_ready && routeA) return { ready: true, label: `Route A · ${routeA}` };
      if (node.api_runtime_ready) return { ready: true, label: 'Route B · 远程 API Key' };
      if (node.server_runtime_ready) return { ready: true, label: 'Route C · 服务器模型' };
      if (routeA) return { ready: !!node.online, label: `本机 CLI · ${routeA}` };
      return { ready: false, label: '无可用运行时' };
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

    function cliLabel(node) {
      const clis = normalizedClis(node);
      return clis.length ? clis.join(' / ') : '未连接本机 CLI';
    }

    function nodeLabel(node) {
      if (!node) return '未连接';
      return clean(node.display_name || node.label || node.device_name || node.short_id || node.node_id || node.agent_id) || 'PC 节点';
    }

    function shortNodeId(nodeId) {
      return nodeId ? `${nodeId.slice(0, 8)}...` : '未知节点';
    }

    function runtimePermission(project) {
      return normalizePermission(project.runtime_permission || project.runtimePermission);
    }

    function normalizePermission(value) {
      return clean(value) === 'full_access' ? 'full_access' : 'project_write';
    }

    function projectTitle(project) {
      return clean(project.display_name || project.displayName || project.alias || project.name || project.title) || '未命名项目';
    }

    function sameId(left, right) {
      return String(left || '') === String(right || '');
    }

    function readinessRow(label, value) {
      return `<div><span>${escapeHtml(label)}</span><strong title="${escapeHtml(value)}">${escapeHtml(value)}</strong></div>`;
    }

    return { renderMemberPanel, bindMemberPanel };
  }

  window.ElonPcProjectReadiness = { create: createProjectReadiness };
})();
