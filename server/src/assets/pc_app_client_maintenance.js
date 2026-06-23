(function () {
  function createClientMaintenanceActions(deps) {
    const clean = deps.clean || ((value) => String(value == null ? '' : value).trim());
    const escapeHtml = deps.escapeHtml || clean;
    const actionCache = [];

    function render(container, actions) {
      if (!container) return;
      actionCache.length = 0;
      if (!Array.isArray(actions) || !actions.length) {
        container.textContent = '刷新本机助手后显示每个操作是否可用。';
        return;
      }
      container.innerHTML = `<div class="settings-client-actions">${actions.map((action, index) => {
        const normalized = normalizeAction(action);
        actionCache[index] = normalized;
        const status = normalized.enabled ? '可用' : disabledReason(normalized);
        const classes = [
          'settings-client-action',
          normalized.enabled ? 'is-enabled' : 'is-disabled',
          normalized.tone === 'danger' ? 'is-danger' : '',
          normalized.tone === 'primary' ? 'is-primary' : ''
        ].filter(Boolean).join(' ');
        const disabled = normalized.enabled ? '' : ' disabled';
        return `<button class="${classes}" type="button" data-maintenance-action-index="${index}" title="${escapeHtml(normalized.description)}"${disabled}>
          <strong>${escapeHtml(normalized.label)} · ${escapeHtml(status)}</strong>
          <span>${escapeHtml(normalized.description)}</span>
        </button>`;
      }).join('')}</div>`;
      container.querySelectorAll('[data-maintenance-action-index]').forEach((button) => {
        button.addEventListener('click', () => {
          const action = actionCache[Number(button.dataset.maintenanceActionIndex)];
          if (action) run(action, button);
        });
      });
    }

    function normalizeAction(action) {
      return {
        id: clean(action && action.id),
        kind: clean(action && action.kind),
        target: clean(action && action.target),
        label: clean(action && action.label) || clean(action && action.id) || '维护操作',
        description: clean(action && action.description) || '本机助手未返回说明。',
        enabled: !!action && action.enabled !== false,
        tone: clean(action && action.tone),
        confirmation: clean(action && action.confirmation)
      };
    }

    function disabledReason(action) {
      if (action.kind === 'repair') return '需要 Windows 本机助手';
      if (action.kind === 'update' || action.kind === 'uninstall') return '需要完整安装';
      return '当前环境不可用';
    }

    async function run(action, button) {
      if (!action.enabled) return;
      if (action.kind === 'uninstall') {
        const ok = window.confirm(action.confirmation || '确认卸载一龙 PC 节点客户端？卸载会退出本机节点并清理安装目录。');
        if (!ok) return;
      }
      await withBusy(button, busyLabel(action), async () => {
        if (action.kind === 'open_target') return openTarget(action);
        if (action.kind === 'export_diagnostics') return exportDiagnostics();
        if (action.kind === 'repair') return runMaintenance('/api/client-maintenance/repair', '已开始后台修复客户端入口。', true);
        if (action.kind === 'update') return runMaintenance('/api/client-maintenance/update', '已开始后台检查更新。', true);
        if (action.kind === 'uninstall') return runMaintenance('/api/client-maintenance/uninstall', '已安排卸载。', false);
        setResult('未知客户端维护动作。', 'error');
      });
    }

    function busyLabel(action) {
      if (action.kind === 'open_target') return '打开中...';
      if (action.kind === 'export_diagnostics') return '生成中...';
      if (action.kind === 'repair') return '修复中...';
      if (action.kind === 'update') return '检查中...';
      if (action.kind === 'uninstall') return '卸载中...';
      return '处理中...';
    }

    async function openTarget(action) {
      try {
        const data = await post('/api/client-maintenance/open', { target: action.target });
        setResult(`已打开：${escapeHtml(data.opened || action.target)}`);
      } catch (error) {
        const protocolUrl = deps.protocolUrlForTarget && deps.protocolUrlForTarget(action.target);
        if (protocolUrl && deps.launchProtocol) {
          deps.launchProtocol(protocolUrl);
          setResult(`本机助手暂时不可达，已请求 Win 端打开：${escapeHtml(action.target)}`, 'note');
          return;
        }
        setResult(escapeHtml(error && (error.message || error)), 'error');
      }
    }

    async function exportDiagnostics() {
      try {
        const data = await post('/api/client-maintenance/diagnostics/export');
        setResult(`已生成诊断信息：${escapeHtml(data.path || '')}`);
      } catch (error) {
        setResult(escapeHtml(error && (error.message || error)), 'error');
      }
    }

    async function runMaintenance(path, fallbackMessage, refreshAfter) {
      try {
        const data = await post(path);
        setResult(escapeHtml(data.message || fallbackMessage));
        if (refreshAfter && deps.refreshMaintenance) {
          window.setTimeout(() => deps.refreshMaintenance(false), 2400);
        }
      } catch (error) {
        if (path.includes('/repair') && deps.launchProtocol && deps.repairProtocolUrl) {
          deps.launchProtocol(deps.repairProtocolUrl);
          setResult(`本机助手暂时不可达，已请求 Win 端修复客户端入口：${escapeHtml(error && (error.message || error))}`, 'note');
          return;
        }
        setResult(escapeHtml(error && (error.message || error)), 'error');
      }
    }

    async function post(path, body) {
      if (!deps.localNodeApi) throw new Error('本机节点 API 不可用');
      const options = { method: 'POST' };
      if (body) options.body = JSON.stringify(body);
      return deps.localNodeApi(path, options);
    }

    async function withBusy(button, label, task) {
      const html = button && button.innerHTML;
      if (button) {
        button.disabled = true;
        button.textContent = label;
      }
      try {
        await task();
      } finally {
        if (button) {
          button.disabled = false;
          button.innerHTML = html;
        }
      }
    }

    function setResult(message, kind) {
      if (deps.setResult) deps.setResult(message, kind);
    }

    return { render };
  }

  window.ElonPcClientMaintenance = { create: createClientMaintenanceActions };
})();
