(function () {
  function createProjectReadiness(deps) {
    const {
      state, $, clean, escapeHtml, api, openSettings,
      selectNode, selectProject, selectProjectChannel
    } = deps;

    function renderMemberPanel(project) {
      const info = buildReadiness(project);
      const permission = runtimePermission(project);
      const permissionTone = permission === 'full_access' ? 'warn' : 'ok';
      const devAction = info.devChannel
        ? `<button class="dev-readiness-action primary" type="button" data-dev-readiness-action="development-channel" data-channel-id="${escapeHtml(info.devChannel.id)}">开发频道</button>`
        : '';
      const nodeAction = info.node
        ? `<button class="dev-readiness-action" type="button" data-dev-readiness-action="node" data-node-id="${escapeHtml(info.nodeId)}">节点详情</button>`
        : `<button class="dev-readiness-action" type="button" data-dev-readiness-action="settings">绑定本机</button>`;
      const settingsAction = info.node
        ? '<button class="dev-readiness-action" type="button" data-dev-readiness-action="settings">项目设置</button>'
        : '';

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
            ${readinessRow('本机工具', info.toolContractLabel)}
          </div>
          <div class="dev-readiness-next">
            <span>下一步</span>
            <strong>${escapeHtml(info.nextStep)}</strong>
          </div>
          <div class="dev-readiness-checks">
            ${readinessChecksHtml(info.checks)}
          </div>
          <label class="dev-readiness-permission ${permissionTone}">
            <span>AI 权限</span>
            <select data-dev-readiness-permission="${escapeHtml(project.id || '')}">
              <option value="project_write" ${permission === 'project_write' ? 'selected' : ''}>仅项目内写入</option>
              <option value="full_access" ${permission === 'full_access' ? 'selected' : ''}>完全访问</option>
            </select>
          </label>
          <div class="dev-readiness-actions">
            ${devAction}
            ${nodeAction}
            ${settingsAction}
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
          if (action === 'development-channel') {
            const channelId = clean(button.dataset.channelId);
            if (channelId) selectProjectChannel(channelId);
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
        const ok = window.confirm(`确认给项目「${projectTitle(project)}」开启完全访问？Route A 的 Codex/Copilot 可能读取或修改项目目录外的本机文件和系统设置；Route B/C 仍保留项目路径和命令白名单，但 build/test 会执行项目代码。`);
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
      const workspaceRaw = clean(project.workspace_path || project.workspacePath);
      const workspace = workspaceRaw || '未绑定本机目录';
      const workspaceReady = !!workspaceRaw;
      const devChannel = findDevelopmentChannel();
      const permission = runtimePermission(project);
      if (!nodeId) {
        const checks = readinessChecks({ workspaceReady, nodeBound: false, nodeOnline: false, route: null, devChannel, permission });
        return {
          tone: 'missing',
          title: '未绑定本机',
          badge: '未就绪',
          nodeId,
          node: null,
          devChannel,
          checks,
          workspace,
          nodeLabel: '未选择 PC 节点',
          routeLabel: '先绑定本地项目',
          cliLabel: '未检查',
          toolContractLabel: '未上报',
          nextStep: nextStepForReadiness(checks)
        };
      }
      if (!node) {
        const checks = readinessChecks({ workspaceReady, nodeBound: true, nodeOnline: false, route: null, devChannel, permission });
        return {
          tone: 'warning',
          title: '节点不可见',
          badge: '需检查',
          nodeId,
          node: null,
          devChannel,
          checks,
          workspace,
          nodeLabel: shortNodeId(nodeId),
          routeLabel: '等待节点上线或授权',
          cliLabel: '未上报',
          toolContractLabel: '未上报',
          nextStep: nextStepForReadiness(checks)
        };
      }
      const route = routeInfo(node);
      const checks = readinessChecks({ workspaceReady, nodeBound: true, nodeOnline: !!node.online, route, devChannel, permission });
      const ready = checks.every((check) => check.ok || check.optional);
      return {
        tone: ready ? 'ready' : 'warning',
        title: ready ? '可以开发' : '运行时未就绪',
        badge: ready ? 'Ready' : 'Check',
        nodeId,
        node,
        devChannel,
        checks,
        workspace,
        nodeLabel: nodeLabel(node),
        routeLabel: route.label,
        cliLabel: cliLabel(node),
        toolContractLabel: localToolContractLabel(node),
        nextStep: nextStepForReadiness(checks)
      };
    }

    function routeInfo(node) {
      const clis = normalizedClis(node);
      const routeA = ['codex', 'copilot', 'claude', 'gemini'].find((cli) => clis.includes(cli));
      if (routeAProbeReady(node, routeA)) return { ready: true, label: `Route A · ${routeA}` };
      if (routeFlagReady(node, 'api_runtime_ready', 'apiRuntimeReady')) return { ready: true, label: 'Route B · 本机 API runtime' };
      if (routeFlagReady(node, 'server_runtime_ready', 'serverRuntimeReady')) return { ready: true, label: 'Route C · 服务器模型' };
      if (routeA) return { ready: false, label: `${routeA} CLI 探测未通过` };
      return { ready: false, label: '无可用运行时' };
    }

    function readinessChecks(input) {
      const route = input.route || { ready: false, label: '无可用运行时' };
      return [
        {
          key: 'workspace',
          ok: input.workspaceReady,
          label: '项目目录',
          detail: input.workspaceReady ? '已绑定本机目录' : '未绑定本机项目目录',
          action: '在项目设置里选择项目文件夹'
        },
        {
          key: 'node',
          ok: input.nodeBound,
          label: 'PC 节点',
          detail: input.nodeBound ? '已绑定执行节点' : '未绑定当前 PC 节点',
          action: '先绑定本机节点'
        },
        {
          key: 'online',
          ok: input.nodeOnline,
          label: '节点在线',
          detail: input.nodeOnline ? '节点正在连接云端' : 'Win 端未在线或账号未登录',
          action: '启动 Win 端并确认已登录'
        },
        {
          key: 'route',
          ok: route.ready,
          label: '运行路线',
          detail: route.ready ? route.label : 'Route A/B/C 都未就绪',
          action: '安装本机 CLI 或启用服务器模型兜底'
        },
        {
          key: 'channel',
          ok: !!input.devChannel,
          label: '开发频道',
          detail: input.devChannel ? 'AI 开发频道可用' : '未找到 AI 开发频道',
          action: '刷新项目空间或重新创建项目频道'
        },
        {
          key: 'permission',
          ok: true,
          optional: true,
          tone: input.permission === 'full_access' ? 'warn' : 'ok',
          label: '执行权限',
          detail: input.permission === 'full_access' ? 'Route A 全权限；B/C 保留白名单' : '限制在项目目录内写入',
          action: input.permission === 'full_access' ? 'Route B/C 仍保留本机白名单；build/test 会执行项目代码' : '需要跨目录操作时再开启完全访问'
        }
      ];
    }

    function nextStepForReadiness(checks) {
      const blocked = checks.find((check) => !check.ok && !check.optional);
      if (blocked) return blocked.action;
      return '打开开发频道，直接描述要修改、检查或运行的任务。';
    }

    function findDevelopmentChannel() {
      const channels = (state.projectSpace && state.projectSpace.channels) || [];
      return channels.find((channel) => {
        const kind = clean(channel.kind || channel.channel_kind).toLowerCase();
        return kind === 'ai_development';
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

    function routeAProbeReady(node, routeA) {
      if (!routeA) return false;
      if (hasRouteFlag(node, 'route_a_ready', 'routeAReady')) {
        return routeFlagReady(node, 'route_a_ready', 'routeAReady');
      }
      return true;
    }

    function routeFlagReady(node, snakeName, camelName) {
      if (!node) return false;
      if (Object.prototype.hasOwnProperty.call(node, snakeName)) return node[snakeName] === true;
      if (Object.prototype.hasOwnProperty.call(node, camelName)) return node[camelName] === true;
      return false;
    }

    function hasRouteFlag(node, snakeName, camelName) {
      return !!node && (
        Object.prototype.hasOwnProperty.call(node, snakeName)
        || Object.prototype.hasOwnProperty.call(node, camelName)
      );
    }

    function cliLabel(node) {
      const clis = normalizedClis(node);
      return clis.length ? clis.join(' / ') : '未连接本机 CLI';
    }

    function localToolContract(node) {
      const runtime = (node && (node.dev_runtime || node.devRuntime)) || {};
      return (runtime.local_tool_contract || runtime.localToolContract) || {};
    }

    function localToolContractLabel(node) {
      const contract = localToolContract(node);
      const supported = arrayStrings(contract.supported_tools || contract.supportedTools);
      if (!supported.length) return '未上报';
      const core = ['read_file_range', 'apply_patch', 'run_command']
        .filter((tool) => supported.includes(tool));
      const approvals = arrayStrings(contract.approval_required_tools || contract.approvalRequiredTools);
      const approvalCore = approvals.filter((tool) => ['write_file', 'apply_patch', 'run_command'].includes(tool));
      const approvalText = approvalCore.length
        ? `${approvalCore.join('/')} 需确认`
        : '审批策略未上报';
      return `${(core.length ? core : supported.slice(0, 3)).join(' / ')} · ${approvalText}`;
    }

    function arrayStrings(value) {
      return Array.isArray(value)
        ? value.map((item) => clean(item)).filter(Boolean)
        : [];
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

    function readinessChecksHtml(checks) {
      return (checks || []).map((check) => {
        const tone = check.tone || (check.ok ? 'ok' : 'bad');
        const mark = check.ok ? '✓' : '!';
        return `<div class="dev-readiness-check ${escapeHtml(tone)}">
          <span>${escapeHtml(mark)}</span>
          <div>
            <strong>${escapeHtml(check.label)}</strong>
            <em>${escapeHtml(check.detail)}</em>
          </div>
        </div>`;
      }).join('');
    }

    return { renderMemberPanel, bindMemberPanel };
  }

  window.ElonPcProjectReadiness = { create: createProjectReadiness };
})();
