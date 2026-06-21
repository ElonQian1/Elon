(function () {
  function create(deps) {
    const state = deps.state;
    const els = deps.els;
    const clean = deps.clean;
    const escapeHtml = deps.escapeHtml;
    const openSettings = deps.openSettings;
    const selectNode = deps.selectNode;
    const openModelPicker = deps.openModelPicker;
    const routeStorageKey = 'elon_pc_dev_runtime_route';
    let bar = null;
    let busy = false;
    let routePreference = normalizeRoutePreference(readStoredRoutePreference());

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
          <div class="dev-route-segment" role="group" aria-label="选择运行路线">
            ${info.routeOptions.map((option) => `
              <button
                type="button"
                class="${option.active ? 'active' : ''}"
                data-dev-composer-route="${escapeHtml(option.value)}"
                title="${escapeHtml(option.title)}"
                aria-pressed="${option.active ? 'true' : 'false'}"
                ${option.enabled ? '' : 'disabled'}
              >${escapeHtml(option.label)}</button>
            `).join('')}
          </div>
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

    function selectedRouteForRequest() {
      return routePreference === 'auto' ? '' : routePreference;
    }

    function ensureBar() {
      if (bar || !els.composer || !els.composer.parentElement) return;
      bar = document.createElement('section');
      bar.className = 'dev-composer-bar';
      bar.hidden = true;
      els.composer.parentElement.insertBefore(bar, els.composer);
    }

    function bindActions() {
      bar.querySelectorAll('[data-dev-composer-route]').forEach((button) => {
        button.addEventListener('click', () => {
          if (button.disabled) return;
          routePreference = normalizeRoutePreference(button.dataset.devComposerRoute);
          saveStoredRoutePreference(routePreference);
          render();
        });
      });
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
      const route = routeInfo(node, routePreference);
      return {
        tone: route.ready ? (permission === 'full_access' ? 'full-access' : 'ready') : 'warning',
        routeLabel: route.label,
        routeOptions: route.options,
        permissionLabel: permission === 'full_access' ? '完全访问' : '仅项目内写入',
        workspace: clean(project.workspace_path || project.workspacePath) || '未绑定本机目录',
        settingsLabel: permission === 'full_access' ? '权限' : '设置'
      };
    }

    function routeInfo(node, selectedRoute) {
      const route = normalizeRoutePreference(selectedRoute);
      if (!node) {
        return routeSummary(route, '未绑定可用节点', false, unavailableRouteOptions(route, '未绑定可用节点'));
      }
      if (!node.online) {
        return routeSummary(route, '节点离线', false, unavailableRouteOptions(route, '节点离线'));
      }
      const clis = normalizedClis(node);
      const routeA = ['codex', 'copilot', 'claude', 'gemini'].find((cli) => clis.includes(cli));
      const routeAReady = !!routeA;
      const routeBReady = !!node.api_runtime_ready;
      const routeCReady = !!node.server_runtime_ready;
      const auto = autoRouteLabel(routeA, routeBReady, routeCReady);
      const options = [
        {
          value: 'auto',
          label: '自动',
          enabled: auto.ready,
          title: auto.ready ? `自动选择：${auto.label}` : '自动选择：运行时未就绪'
        },
        {
          value: 'route_a',
          label: 'A',
          enabled: routeAReady,
          title: routeAReady ? `Route A · ${routeA}` : 'Route A 未就绪：未检测到 Codex/Copilot/Claude/Gemini CLI'
        },
        {
          value: 'route_b',
          label: 'B',
          enabled: routeBReady,
          title: routeBReady ? 'Route B · 本机 API Runtime' : 'Route B 未就绪：未配置本机 API key/runtime'
        },
        {
          value: 'route_c',
          label: 'C',
          enabled: routeCReady,
          title: routeCReady ? 'Route C · 一龙服务器模型' : 'Route C 未就绪：Win 客户端未连接服务器模型 runtime'
        }
      ].map((option) => ({ ...option, active: option.value === route }));
      const selected = options.find((option) => option.value === route) || options[0];
      const labelByRoute = {
        auto: auto.label,
        route_a: routeAReady ? `Route A · ${routeA}` : 'Route A 未就绪',
        route_b: routeBReady ? 'Route B · 本机 API runtime' : 'Route B 未就绪',
        route_c: routeCReady ? 'Route C · 服务器模型' : 'Route C 未就绪'
      };
      const ready = route === 'auto' ? auto.ready : !!selected.enabled;
      return routeSummary(route, labelByRoute[route] || auto.label, ready, options);
    }

    function routeSummary(route, label, ready, options) {
      return {
        ready,
        label,
        options: options.map((option) => ({
          ...option,
          active: option.value === route
        }))
      };
    }

    function unavailableRouteOptions(route, reason) {
      return ['auto', 'route_a', 'route_b', 'route_c'].map((value) => ({
        value,
        label: value === 'auto' ? '自动' : value.slice(-1).toUpperCase(),
        enabled: false,
        active: value === route,
        title: reason
      }));
    }

    function autoRouteLabel(routeA, routeBReady, routeCReady) {
      if (routeA) return { ready: true, label: `Route A · ${routeA}` };
      if (routeBReady) return { ready: true, label: 'Route B · 本机 API runtime' };
      if (routeCReady) return { ready: true, label: 'Route C · 服务器模型' };
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

    function normalizeRoutePreference(value) {
      const cleanValue = clean(value).toLowerCase().replace(/-/g, '_');
      if (['route_a', 'route_b', 'route_c'].includes(cleanValue)) return cleanValue;
      return 'auto';
    }

    function readStoredRoutePreference() {
      try {
        return typeof localStorage === 'undefined' ? '' : localStorage.getItem(routeStorageKey);
      } catch (_) {
        return '';
      }
    }

    function saveStoredRoutePreference(value) {
      try {
        if (typeof localStorage !== 'undefined') {
          localStorage.setItem(routeStorageKey, normalizeRoutePreference(value));
        }
      } catch (_) {
        // localStorage can be disabled in privacy mode; the in-memory selection still works.
      }
    }

    return { render, setBusy, selectedRouteForRequest };
  }

  window.ElonPcDevComposer = { create };
})();
