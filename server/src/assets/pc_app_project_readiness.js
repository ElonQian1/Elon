(function () {
  function createProjectReadiness(deps) {
    const {
      state, $, clean, escapeHtml, api, openSettings,
      selectNode, selectProject, selectProjectChannel
    } = deps;

    function renderMemberPanel(project) {
      if (!canUseDeveloperReadiness(project)) return '';
      const info = buildReadiness(project);
      const permission = runtimePermission(project);
      const permissionTone = permission === 'project_write' ? 'ok' : 'warn';
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
        <details class="dev-readiness ${escapeHtml(info.tone)}" data-dev-readiness-project="${escapeHtml(project.id || '')}">
          <summary class="dev-readiness-summary">
            <div>
              <span>开发者设置</span>
              <strong>${escapeHtml(info.title)}</strong>
            </div>
            <em>${escapeHtml(info.badge)}</em>
          </summary>
          <div class="dev-readiness-body">
            <div class="dev-readiness-grid">
              ${readinessRow('项目目录', info.workspace)}
              ${readinessRow('执行节点', info.nodeLabel)}
              ${readinessRow('运行路线', info.routeLabel)}
              ${info.routeCProtectionLabel ? readinessRow('Route C 保护', info.routeCProtectionLabel) : ''}
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
                <option value="danger_full_access" ${permission === 'danger_full_access' ? 'selected' : ''}>完整本机命令行</option>
              </select>
            </label>
            <div class="dev-readiness-actions">
              ${devAction}
              ${nodeAction}
              ${settingsAction}
              <button class="dev-readiness-action" type="button" data-dev-readiness-action="refresh" data-project-id="${escapeHtml(project.id || '')}">刷新</button>
            </div>
          </div>
        </details>`;
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
      if (next === 'full_access' || next === 'danger_full_access') {
        const dangerText = next === 'danger_full_access'
          ? 'Route A/B/C 都可能运行任意 cmd/powershell 命令，并读写项目目录外的本机文件和系统设置。'
          : 'Route A 的 Codex/Copilot 可能读取或修改项目目录外的本机文件和系统设置；Route B/C 仍保留项目路径和命令白名单，但 build/test 会执行项目代码。';
        const ok = window.confirm(`确认给项目「${projectTitle(project)}」开启${permissionLabel(next)}？${dangerText}`);
        if (!ok) {
          select.value = previous;
          return;
        }
      }
      select.disabled = true;
      try {
        const data = await api(`/api/projects/${encodeURIComponent(projectId)}/runtime-permission`, {
          method: 'PATCH',
          body: JSON.stringify({
            mode: next,
            confirmFullAccess: next === 'full_access' || next === 'danger_full_access',
            confirmDangerFullAccess: next === 'danger_full_access'
          })
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
          routeCProtectionLabel: '',
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
          routeCProtectionLabel: '',
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
        routeCProtectionLabel: routeCProtectionLabel(node),
        cliLabel: cliLabel(node),
        toolContractLabel: localToolContractLabel(node),
        nextStep: nextStepForReadiness(checks)
      };
    }

    function canUseDeveloperReadiness(project) {
      const role = clean(project && (project.role || project.member_role || project.memberRole)).toLowerCase();
      return ['owner', 'admin', 'editor', 'developer', 'maintainer'].includes(role);
    }

    function routeInfo(node) {
      const clis = normalizedClis(node);
      const routeA = ['codex', 'copilot', 'claude', 'gemini'].find((cli) => clis.includes(cli));
      if (routeAProbeReady(node, routeA)) return { ready: true, label: `Route A · ${routeA}` };
      if (routeFlagReady(node, 'api_runtime_ready', 'apiRuntimeReady')) return { ready: true, label: 'Route B · 本机 API runtime' };
      if (routeFlagReady(node, 'server_runtime_ready', 'serverRuntimeReady')) return { ready: true, label: routeCReadyLabel(node) };
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
          tone: input.permission === 'project_write' ? 'ok' : 'warn',
          label: '执行权限',
          detail: permissionDetail(input.permission),
          action: permissionAction(input.permission)
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

    function routeCStatus(node) {
      const runtime = (node && (node.dev_runtime || node.devRuntime)) || {};
      return runtime.server_runtime_status || runtime.serverRuntimeStatus || null;
    }

    function routeCReadyLabel(node) {
      const status = routeCStatus(node) || {};
      const agent = status.agent || {};
      const model = clean(agent.model);
      return model ? `Route C · 服务器模型 ${model}` : 'Route C · 服务器模型';
    }

    function routeCProtectionLabel(node) {
      const status = routeCStatus(node);
      if (!status) return '';
      const stateText = clean(status.status);
      const policy = status.policy || {};
      if (policy.enabled === false || stateText === 'disabled') return '已关闭 · 运维开关保护';
      if (stateText === 'missing_token') return '未登录 · 不会调用服务器模型';
      if (stateText === 'unsupported_agent_usage_mode') return 'agent 模式不允许 · 只允许 server_api_key · 不会启用';
      if (stateText === 'http_error') return `云端返回 ${clean(status.httpStatus) || '错误'} · 不会启用`;
      if (stateText === 'unavailable') return `${clean(status.reason) || '云端不可用'} · 不会启用`;
      const budget = status.budget || {};
      const budgetStatus = clean(budget.status);
      if (stateText === 'user_budget_exhausted' || budgetStatus === 'user_exhausted') {
        const retryAfter = numberField(budget, 'resetAfterSecs', 'reset_after_secs');
        return retryAfter
          ? `已保护 · 今日个人额度已用完 · ${retryAfter} 秒后重试`
          : '已保护 · 今日个人额度已用完';
      }
      if (stateText === 'budget_exhausted' || budgetStatus === 'exhausted') {
        const retryAfter = numberField(budget, 'resetAfterSecs', 'reset_after_secs');
        return retryAfter
          ? `已保护 · 今日平台预算已用完 · ${retryAfter} 秒后重试`
          : '已保护 · 今日平台预算已用完';
      }
      const availability = status.admissionAvailability || status.admission_availability || {};
      if (stateText === 'limited' || availability.ready === false) {
        const retryAfter = numberField(availability, 'retryAfterSecs', 'retry_after_secs');
        const message = clean(availability.publicMessage || availability.public_message)
          || routeCLimitedReasonText(clean(availability.reason))
          || '当前容量已满';
        return retryAfter ? `已保护 · ${message} · ${retryAfter} 秒后重试` : `已保护 · ${message}`;
      }

      const limits = status.limits || {};
      const admission = status.admission || {};
      const protection = status.protection || {};
      const agentSelection = clean(protection.agentSelection || protection.agent_selection);
      const agentPolicy = routeCAgentPolicyLabel(status);
      const rpm = numberField(limits, 'maxRequestsPerMinute', 'max_requests_per_minute')
        || numberField(admission, 'maxRequestsPerMinute', 'max_requests_per_minute');
      const perUser = numberField(limits, 'maxConcurrentPerUser', 'max_concurrent_per_user')
        || numberField(admission, 'maxConcurrentPerUser', 'max_concurrent_per_user');
      const global = numberField(limits, 'maxConcurrentGlobal', 'max_concurrent_global')
        || numberField(admission, 'maxConcurrentGlobal', 'max_concurrent_global');
      const duplicateWindow = numberField(limits, 'duplicateRequestWindowSecs', 'duplicate_request_window_secs')
        || numberField(admission, 'duplicateRequestWindowSecs', 'duplicate_request_window_secs');
      const remaining = numberField(admission, 'remainingRequestsPerMinute', 'remaining_requests_per_minute');
      const dailyLimit = numberField(budget, 'dailyCallLimit', 'daily_call_limit');
      const remainingBudget = numberField(budget, 'remainingCallsToday', 'remaining_calls_today');
      const userDailyLimit = numberField(budget, 'perUserDailyCallLimit', 'per_user_daily_call_limit');
      const remainingUserBudget = numberField(budget, 'remainingCallsTodayForUser', 'remaining_calls_today_for_user');
      const parts = [];
      if (agentPolicy) parts.push(agentPolicy);
      else if (agentSelection) parts.push('agent 受控');
      if (rpm) parts.push(`${rpm}/分钟`);
      if (perUser || global) parts.push(`并发 ${perUser || '?'} / ${global || '?'}`);
      if (duplicateWindow) parts.push(`重复防抖 ${duplicateWindow}秒`);
      if (Number.isFinite(remaining)) parts.push(`剩余 ${remaining}`);
      if (Number.isFinite(dailyLimit) && Number.isFinite(remainingBudget)) parts.push(`今日剩余 ${remainingBudget}/${dailyLimit}`);
      if (Number.isFinite(userDailyLimit) && Number.isFinite(remainingUserBudget)) parts.push(`个人今日剩余 ${remainingUserBudget}/${userDailyLimit}`);
      return parts.length ? `已保护 · ${parts.join(' · ')}` : '已保护 · 限额策略已上报';
    }

    function routeCAgentPolicyLabel(status) {
      const policy = (status && (status.agentPolicy || status.agent_policy)) || {};
      const mode = clean(policy.mode);
      if (!mode) return '';
      if (mode === 'default_agent_only') return 'agent策略 默认';
      if (mode === 'allowlist') return 'agent策略 白名单';
      if (mode === 'any') return 'agent策略 开放';
      return `agent策略 ${mode}`;
    }

    function routeCLimitedReasonText(reason) {
      if (reason === 'global_concurrency_limited') return '平台并发已满';
      if (reason === 'user_concurrency_limited') return '当前用户并发已满';
      if (reason === 'rate_limited') return '请求频率已达上限';
      return '';
    }

    function localToolContractLabel(node) {
      const contract = localToolContract(node);
      const supported = arrayStrings(contract.supported_tools || contract.supportedTools);
      if (!supported.length) return '未上报';
      const readTools = [
        'search_files', 'list_dir', 'file_info', 'read_file', 'read_file_range',
        'git_status', 'git_diff', 'git_log', 'git_show'
      ]
        .filter((tool) => supported.includes(tool));
      const approvals = arrayStrings(contract.approval_required_tools || contract.approvalRequiredTools);
      const writeTools = ['write_file', 'apply_patch', 'run_command']
        .filter((tool) => supported.includes(tool));
      const approvalCore = writeTools.filter((tool) => approvals.includes(tool));
      const parts = [];
      if (readTools.length) parts.push(`只读 ${readTools.join('/')}`);
      if (approvalCore.length) parts.push(`需确认 ${approvalCore.join('/')}`);
      if (writeTools.length && approvalCore.length < writeTools.length) {
        const unreported = writeTools.filter((tool) => !approvalCore.includes(tool));
        parts.push(`写入 ${unreported.join('/')}`);
      }
      const shown = new Set([...readTools, ...writeTools]);
      const others = supported.filter((tool) => !shown.has(tool));
      if (others.length) parts.push(`其他 ${others.join('/')}`);
      return parts.length ? parts.join(' · ') : supported.join(' / ');
    }

    function arrayStrings(value) {
      return Array.isArray(value)
        ? value.map((item) => clean(item)).filter(Boolean)
        : [];
    }

    function numberField(object, camelName, snakeName) {
      if (!object || (!Object.prototype.hasOwnProperty.call(object, camelName) && !Object.prototype.hasOwnProperty.call(object, snakeName))) return null;
      const value = object[camelName] ?? object[snakeName];
      const number = Number(value);
      return Number.isFinite(number) ? number : null;
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
      const mode = clean(value);
      if (mode === 'danger_full_access') return 'danger_full_access';
      return mode === 'full_access' ? 'full_access' : 'project_write';
    }

    function permissionLabel(permission) {
      if (permission === 'danger_full_access') return '完整本机命令行';
      return permission === 'full_access' ? '完全访问' : '仅项目内写入';
    }

    function permissionDetail(permission) {
      if (permission === 'danger_full_access') return 'Route A/B/C 完整本机命令行';
      return permission === 'full_access' ? 'Route A 全权限；B/C 保留白名单' : '限制在项目目录内写入';
    }

    function permissionAction(permission) {
      if (permission === 'danger_full_access') return 'AI 可运行 cmd/powershell 并读写绝对路径';
      return permission === 'full_access' ? 'Route B/C 仍保留本机白名单；build/test 会执行项目代码' : '需要跨目录操作时再开启完全访问';
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
