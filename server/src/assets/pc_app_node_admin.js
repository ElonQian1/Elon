(function () {
  function createNativeNodeAdmin(deps) {
    const { state, $, clean, escapeHtml, localNodeApi, ensureLocalNodeLogin } = deps;
    let root = null;
    let clientMaintenanceData = null;
    let latestClientPackage = null;

    function render(surface, status) {
      root = surface;
      root.innerHTML = template(status || {});
      bind();
      loadStorageConfig(true);
      loadEnvCheck(true);
      loadLatestClientPackageVersion(true);
      loadClientMaintenance(true);
      loadTts(true);
    }

    function template(status) {
      const logged = !!status.logged_in;
      const storage = status.storage || {};
      return `
        <div class="node-native-admin">
          <section class="node-admin-grid top">
            <div class="node-admin-panel identity">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">本机节点</span>
                  <h3>${escapeHtml(clean(status.device_name) || '这台电脑')}</h3>
                </div>
                <span class="node-status-chip ${status.connected ? 'online' : 'checking'}">${status.connected ? '云端在线' : '等待云端'}</span>
              </div>
              <div class="node-kv-list dense">
                ${row('登录', logged ? '已登录' : '未登录')}
                ${row('节点 ID', status.agent_id || '登录后自动生成')}
                ${row('版本', status.version || '未知')}
                ${row('硬件', hardwareLine(status.hardware))}
                ${row('硬盘服务', storageLine(storage))}
                ${row('云端', status.cloud_url || '未配置')}
              </div>
              <div class="node-admin-actions">
                <button class="node-action primary" type="button" id="nodeNativeLogin">${logged ? '重新绑定当前账号' : '用当前账号注册节点'}</button>
                <button class="node-action" type="button" id="nodeNativeRefresh">刷新状态</button>
                <button class="node-action" type="button" id="nodeNativeLogout"${logged ? '' : ' disabled'}>登出本机节点</button>
                <button class="node-action" type="button" id="nodeNativeAdvanced">高级本机页</button>
              </div>
              <div class="node-inline-result" id="nodeNativeResult"></div>
            </div>
            <div class="node-admin-panel project">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">项目</span>
                  <h3>注册本地项目</h3>
                </div>
              </div>
              <p class="node-admin-copy">从这里选择本机项目文件夹，工作台会自动读取 Git 远端、分支和项目命令，并在信息完整时直接注册。</p>
              <div class="node-admin-actions">
                <button class="node-action primary" type="button" id="nodeOpenProjectSettings">选择文件夹并注册</button>
              </div>
            </div>
          </section>

          <section class="node-admin-grid">
            <div class="node-admin-panel">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">项目硬盘</span>
                  <h3>让本机保存项目母仓</h3>
                </div>
                <button class="node-mini-button" type="button" id="nodeStorageRefresh">刷新</button>
              </div>
              <label class="node-check-field"><input id="nodeStorageEnabled" type="checkbox" /> 启用本机作为项目硬盘节点</label>
              <label class="node-form-field"><span>存储根目录</span><input id="nodeStorageRoot" placeholder="留空使用默认目录" /></label>
              <label class="node-form-field"><span>外部 Git 服务基础地址</span><input id="nodeStorageGitBase" placeholder="可选，高级直连" /></label>
              <div class="node-admin-actions"><button class="node-action primary" type="button" id="nodeStorageSave">保存硬盘服务配置</button></div>
              <div class="node-inline-result" id="nodeStorageResult"></div>
            </div>

            <div class="node-admin-panel">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">AI 编码工具</span>
                  <h3>本机开发环境</h3>
                </div>
                <button class="node-mini-button" type="button" id="nodeEnvRefresh">重新检查</button>
              </div>
              <div class="node-env-grid" id="nodeEnvGrid"><span>检查中...</span></div>
              <div class="node-runtime-contract" id="nodeApiRuntimeContract"></div>
              <div class="node-admin-actions"><button class="node-action primary" type="button" id="nodeEnvInstall">一键安装 / 修复</button></div>
              <label class="node-form-field"><span>API Key</span><input id="nodeOpenAiKey" type="password" placeholder="sk-..." autocomplete="off" /></label>
              <label class="node-form-field"><span>Route B 模型</span><input id="nodeApiRuntimeModel" placeholder="如 gpt-5.4-mini" autocomplete="off" /></label>
              <label class="node-form-field"><span>API Base URL</span><input id="nodeApiRuntimeBase" placeholder="默认 https://api.openai.com/v1" autocomplete="off" /></label>
              <div class="node-admin-actions"><button class="node-action" type="button" id="nodeSaveOpenAiKey">保存 Route B 配置</button></div>
              <div class="node-inline-result" id="nodeEnvResult"></div>
            </div>
          </section>

          <section class="node-admin-grid bottom">
            <div class="node-admin-panel">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">模型</span>
                  <h3>本地模型能力</h3>
                </div>
              </div>
              ${modelsTable(status.models || [])}
            </div>
            <div class="node-admin-panel">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">客户端</span>
                  <h3>更新、日志和卸载</h3>
                </div>
                <button class="node-mini-button" type="button" id="nodeClientRefresh">刷新</button>
              </div>
              <div class="node-kv-list dense" id="nodeClientMaintenance">${row('状态', '读取中')}</div>
              <div class="node-admin-actions" id="nodeClientActions"></div>
              <div class="node-inline-result" id="nodeClientResult"></div>
            </div>
            <div class="node-admin-panel">
              <div class="node-panel-head">
                <div>
                  <span class="node-admin-eyebrow">TTS</span>
                  <h3>本机语音 Worker</h3>
                </div>
                <span class="node-status-chip checking" id="nodeTtsChip">检查中</span>
              </div>
              <div class="node-kv-list dense" id="nodeTtsStatus">${row('端口', '检查中')}</div>
              <label class="node-form-field"><span>TTS Worker URL</span><input id="nodeTtsWorkerUrl" placeholder="如 http://127.0.0.1:5011" /></label>
              <div class="node-admin-actions">
                <button class="node-action primary" type="button" id="nodeTtsSave">保存 TTS 配置</button>
                <button class="node-action" type="button" id="nodeTtsRefresh">刷新</button>
              </div>
              <div class="node-inline-result" id="nodeTtsResult"></div>
            </div>
          </section>
        </div>`;
    }

    function bind() {
      $('#nodeNativeRefresh')?.addEventListener('click', () => refreshStatus());
      $('#nodeNativeLogin')?.addEventListener('click', loginWithCurrentUser);
      $('#nodeNativeLogout')?.addEventListener('click', logoutLocalNode);
      $('#nodeNativeAdvanced')?.addEventListener('click', () => window.open(state.nodeAdminUrl, '_blank', 'noopener'));
      $('#nodeOpenProjectSettings')?.addEventListener('click', () => {
        if (typeof deps.openSettings === 'function') deps.openSettings('workbench', { autoPickAndRegister: true });
      });
      $('#nodeStorageRefresh')?.addEventListener('click', () => loadStorageConfig(false));
      $('#nodeStorageSave')?.addEventListener('click', saveStorageConfig);
      $('#nodeEnvRefresh')?.addEventListener('click', () => loadEnvCheck(false));
      $('#nodeEnvInstall')?.addEventListener('click', installEnv);
      $('#nodeSaveOpenAiKey')?.addEventListener('click', saveOpenAiKey);
      $('#nodeClientRefresh')?.addEventListener('click', () => loadClientMaintenance(false));
      $('#nodeClientActions')?.addEventListener('click', handleClientActionClick);
      $('#nodeTtsRefresh')?.addEventListener('click', () => loadTts(false));
      $('#nodeTtsSave')?.addEventListener('click', saveTtsConfig);
    }

    async function refreshStatus() {
      if (typeof deps.probeLocalNode === 'function') {
        await deps.probeLocalNode(true);
        return;
      }
      const status = await api('/api/status');
      if (root) render(root, status);
    }

    async function loginWithCurrentUser() {
      await withBusy('nodeNativeLogin', '绑定中...', async () => {
        try {
          if (ensureLocalNodeLogin) await ensureLocalNodeLogin();
          else await api('/api/login', { method: 'POST', body: JSON.stringify({ token: state.token }) });
          await afterCloudChange();
          await refreshStatus();
          setResult('nodeNativeResult', '本机节点已绑定当前账号，正在连接云端。');
        } catch (error) {
          setResult('nodeNativeResult', error.message || error, 'error');
        }
      });
    }

    async function logoutLocalNode() {
      await withBusy('nodeNativeLogout', '登出中...', async () => {
        try {
          await api('/api/logout', { method: 'POST' });
          await afterCloudChange();
          await refreshStatus();
          setResult('nodeNativeResult', '本机节点已登出。');
        } catch (error) {
          setResult('nodeNativeResult', error.message || error, 'error');
        }
      });
    }

    async function loadStorageConfig(quiet) {
      try {
        const data = await api('/api/storage-config');
        $('#nodeStorageEnabled').checked = !!data.enabled;
        $('#nodeStorageRoot').value = clean(data.root_path);
        $('#nodeStorageGitBase').value = clean(data.git_base_url);
        if (!quiet) setResult('nodeStorageResult', storageLine(data.profile || data));
      } catch (error) {
        setResult('nodeStorageResult', error.message || error, 'error');
      }
    }

    async function saveStorageConfig() {
      await withBusy('nodeStorageSave', '保存中...', async () => {
        try {
          const data = await api('/api/storage-config', {
            method: 'POST',
            body: JSON.stringify({
              enabled: !!$('#nodeStorageEnabled')?.checked,
              root_path: clean($('#nodeStorageRoot')?.value) || null,
              git_base_url: clean($('#nodeStorageGitBase')?.value) || null
            })
          });
          await refreshStatus();
          setResult('nodeStorageResult', data.enabled ? `硬盘服务已启用：${data.root_path || '默认目录'}` : '硬盘服务已关闭。');
        } catch (error) {
          setResult('nodeStorageResult', error.message || error, 'error');
        }
      });
    }

    async function loadEnvCheck(quiet) {
      try {
        const env = await api('/api/env-check');
        renderEnvGrid(env);
        if (!quiet) setResult('nodeEnvResult', '检查完成。');
      } catch (error) {
        setResult('nodeEnvResult', error.message || error, 'error');
      }
    }

    async function installEnv() {
      await withBusy('nodeEnvInstall', '启动中...', async () => {
        try {
          const data = await api('/api/install-env', { method: 'POST' });
          setResult('nodeEnvResult', data.msg || '安装向导已启动。');
          window.setTimeout(() => loadEnvCheck(true), 8000);
        } catch (error) {
          setResult('nodeEnvResult', error.message || error, 'error');
        }
      });
    }

    async function saveOpenAiKey() {
      const key = clean($('#nodeOpenAiKey')?.value);
      const model = clean($('#nodeApiRuntimeModel')?.value);
      const apiBase = clean($('#nodeApiRuntimeBase')?.value);
      if (!key) return setResult('nodeEnvResult', '请输入 API Key。', 'error');
      if (!model) return setResult('nodeEnvResult', '请输入 Route B 模型。', 'error');
      await withBusy('nodeSaveOpenAiKey', '保存中...', async () => {
        try {
          const result = await api('/api/save-openai-key', {
            method: 'POST',
            body: JSON.stringify({ api_key: key, model, api_base: apiBase || null })
          });
          $('#nodeOpenAiKey').value = '';
          setResult('nodeEnvResult', result.msg || 'Route B 配置已保存。');
          await loadEnvCheck(true);
        } catch (error) {
          setResult('nodeEnvResult', error.message || error, 'error');
        }
      });
    }

    async function loadClientMaintenance(quiet) {
      try {
        const data = await api('/api/client-maintenance');
        clientMaintenanceData = data;
        renderClientMaintenance(data);
        loadLatestClientPackageVersion(true);
        if (!quiet) setResult('nodeClientResult', '客户端维护状态已刷新。');
      } catch (error) {
        const panel = $('#nodeClientMaintenance');
        if (panel) panel.innerHTML = row('状态', '无法读取本机维护状态');
        setResult('nodeClientResult', error.message || error, 'error');
      }
    }

    async function loadLatestClientPackageVersion(quiet) {
      try {
        const resp = await fetch('/api/node-agent/version', { cache: 'no-store' });
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        latestClientPackage = await resp.json();
      } catch (error) {
        latestClientPackage = { error: clean(error && (error.message || error)) || 'version_unavailable' };
      }
      if (clientMaintenanceData) renderClientMaintenance(clientMaintenanceData);
      if (!quiet && latestClientPackage && latestClientPackage.error) {
        setResult('nodeClientResult', '无法读取线上客户端版本，仍可使用本地更新按钮。', 'error');
      }
    }

    async function openClientTarget(target, buttonId) {
      await withBusy(buttonId, '打开中...', async () => {
        try {
          const data = await api('/api/client-maintenance/open', {
            method: 'POST',
            body: JSON.stringify({ target })
          });
          setResult('nodeClientResult', `已打开：${data.opened || target}`);
        } catch (error) {
          setResult('nodeClientResult', error.message || error, 'error');
        }
      });
    }

    async function exportClientDiagnostics(buttonId) {
      await withBusy(buttonId || 'nodeExportDiagnostics', '生成中...', async () => {
        try {
          const data = await api('/api/client-maintenance/diagnostics/export', { method: 'POST' });
          setResult('nodeClientResult', `诊断信息已生成：${data.path || ''}`);
        } catch (error) {
          setResult('nodeClientResult', error.message || error, 'error');
        }
      });
    }

    async function updateClient(buttonId) {
      await withBusy(buttonId || 'nodeClientUpdate', '检查中...', async () => {
        try {
          const data = await api('/api/client-maintenance/update', { method: 'POST' });
          setResult('nodeClientResult', data.message || '已开始后台检查更新。');
          window.setTimeout(() => loadClientMaintenance(true), 2400);
        } catch (error) {
          setResult('nodeClientResult', error.message || error, 'error');
        }
      });
    }

    async function repairClient(buttonId) {
      await withBusy(buttonId || 'nodeClientRepair', '修复中...', async () => {
        try {
          const data = await api('/api/client-maintenance/repair', { method: 'POST' });
          setResult('nodeClientResult', data.message || '已开始后台修复客户端入口。');
          window.setTimeout(() => loadClientMaintenance(true), 2400);
        } catch (error) {
          setResult('nodeClientResult', error.message || error, 'error');
        }
      });
    }

    async function uninstallClient(buttonId, confirmation) {
      if (!window.confirm(confirmation || '确认卸载一龙 PC 节点客户端？卸载会退出本机节点并清理安装目录。')) return;
      await withBusy(buttonId || 'nodeClientUninstall', '卸载中...', async () => {
        try {
          const data = await api('/api/client-maintenance/uninstall', { method: 'POST' });
          setResult('nodeClientResult', data.message || '已安排卸载。');
        } catch (error) {
          setResult('nodeClientResult', error.message || error, 'error');
        }
      });
    }

    function handleClientActionClick(event) {
      const button = event.target && event.target.closest && event.target.closest('[data-client-action]');
      if (!button || button.disabled) return;
      const kind = clean(button.dataset.clientAction);
      const target = clean(button.dataset.clientTarget);
      const confirmation = clean(button.dataset.clientConfirmation);
      const buttonId = button.id;
      if (kind === 'open_target') return openClientTarget(target, buttonId);
      if (kind === 'export_diagnostics') return exportClientDiagnostics(buttonId);
      if (kind === 'repair') return repairClient(buttonId);
      if (kind === 'update') return updateClient(buttonId);
      if (kind === 'uninstall') return uninstallClient(buttonId, confirmation);
      setResult('nodeClientResult', '未知客户端维护动作。', 'error');
    }

    function renderClientMaintenance(data) {
      const panel = $('#nodeClientMaintenance');
      if (!panel) return;
      if (!data) {
        panel.innerHTML = row('状态', '未读取');
        renderClientActions(null);
        return;
      }
      const install = data.install || {};
      const product = data.product_status || install.product_status || {};
      const startMenu = data.start_menu || install.start_menu || {};
      const installedSha = clean(data.installed_git_sha || install.installed_git_sha);
      const packageVersion = clean(data.installed_package_version || install.installed_package_version);
      const installState = data.supported === false
        ? '当前平台不支持维护'
        : data.installed
          ? '已安装'
          : '未检测到完整安装';
      const running = data.running_from_install_dir ? '安装目录运行中' : '外部启动或未确认';
      const packageLine = installedSha
        ? `${shortHash(installedSha)}${packageVersion ? ` / ${packageVersion}` : ''}`
        : '未读取';
      const entryLine = clean(product.primary_entry_name) || '一龙PC节点.exe';
      const startMenuLine = clientStartMenuLine(product, startMenu);
      const recommendedLine = clientRecommendedActionsLine(product);
      const updateLine = clientUpdateLine(data, latestClientPackage);
      panel.innerHTML = [
        row('健康', clean(product.summary) || '未读取'),
        row('建议', recommendedLine || '无需处理'),
        row('入口', entryLine),
        row('开始菜单', startMenuLine || '未上报'),
        row('运行版本', clean(data.version) || '未知'),
        row('安装状态', `${installState} · ${running}`),
        row('安装包', packageLine),
        row('更新', updateLine),
        row('目录结构', clientLayoutLabel(data.layout_status || install.layout_status)),
        row('运行日志', clean(data.logs_dir) || '未上报'),
        row('启动器日志', clean(data.launcher_logs_dir) || '未上报'),
        row('任务记录', clean(data.task_journal_dir) || '未上报'),
        row('诊断目录', clean(data.diagnostics_dir) || '未上报')
      ].join('');
      renderClientActions(data);
    }

    function renderClientActions(data) {
      const actionsEl = $('#nodeClientActions');
      if (!actionsEl) return;
      const actions = clientActions(data);
      actionsEl.innerHTML = actions.map((action) => {
        const id = clientActionButtonId(action.id || action.kind || action.label);
        const disabled = action.enabled === false ? ' disabled' : '';
        const tone = clean(action.tone);
        const className = ['node-action', tone === 'primary' ? 'primary' : '', tone === 'danger' ? 'danger' : '']
          .filter(Boolean)
          .join(' ');
        return `<button class="${className}" type="button" id="${escapeHtml(id)}" data-client-action="${escapeHtml(clean(action.kind))}" data-client-target="${escapeHtml(clean(action.target))}" data-client-confirmation="${escapeHtml(clean(action.confirmation))}" title="${escapeHtml(clean(action.description))}"${disabled}>${escapeHtml(clean(action.label) || '操作')}</button>`;
      }).join('');
    }

    function clientActions(data) {
      const actions = data && Array.isArray(data.maintenance_actions) ? data.maintenance_actions : [];
      if (actions.length) return actions;
      return [
        { id: 'open_install_dir', kind: 'open_target', target: 'install_dir', label: '安装目录', enabled: true },
        { id: 'open_client_logs', kind: 'open_target', target: 'logs', label: '运行日志', enabled: true },
        { id: 'open_launcher_logs', kind: 'open_target', target: 'launcher_logs', label: '启动器日志', enabled: true },
        { id: 'open_task_journal', kind: 'open_target', target: 'task_journal', label: '任务日志', enabled: true },
        { id: 'open_diagnostics_dir', kind: 'open_target', target: 'diagnostics_dir', label: '诊断目录', enabled: true },
        { id: 'export_diagnostics', kind: 'export_diagnostics', label: '导出诊断', enabled: true },
        { id: 'repair_client', kind: 'repair', label: '修复客户端', tone: 'primary', enabled: true },
        { id: 'check_update', kind: 'update', label: '检查更新', tone: 'primary', enabled: true },
        { id: 'uninstall_client', kind: 'uninstall', label: '卸载', tone: 'danger', enabled: true }
      ];
    }

    function clientActionButtonId(value) {
      if (clean(value) === 'open_diagnostics_dir') return 'nodeOpenDiagnosticsDir';
      return `nodeClientAction_${clean(value).replace(/[^a-zA-Z0-9_-]/g, '_') || 'item'}`;
    }

    async function loadTts(quiet) {
      try {
        const [status, config] = await Promise.all([api('/api/tts-status'), api('/api/tts-relay-config')]);
        $('#nodeTtsChip').textContent = status.running ? '运行中' : '未运行';
        $('#nodeTtsChip').className = `node-status-chip ${status.running ? 'online' : 'offline'}`;
        $('#nodeTtsStatus').innerHTML = [
          row('启用', status.enabled_in_env ? '已启用' : '未启用'),
          row('端口', status.port || '5011'),
          row('健康', status.running ? '可访问' : '未检测到 Worker')
        ].join('');
        $('#nodeTtsWorkerUrl').value = clean(config.ttsWorkerUrl);
        if (!quiet) setResult('nodeTtsResult', 'TTS 状态已刷新。');
      } catch (error) {
        setResult('nodeTtsResult', error.message || error, 'error');
      }
    }

    async function saveTtsConfig() {
      await withBusy('nodeTtsSave', '保存中...', async () => {
        try {
          const url = clean($('#nodeTtsWorkerUrl')?.value);
          await api('/api/tts-relay-config', { method: 'POST', body: JSON.stringify({ tts_worker_url: url || null }) });
          setResult('nodeTtsResult', 'TTS 配置已保存。');
          await loadTts(true);
        } catch (error) {
          setResult('nodeTtsResult', error.message || error, 'error');
        }
      });
    }

    async function afterCloudChange() {
      if (typeof deps.loadBaseData === 'function') await deps.loadBaseData().catch(() => {});
      if (typeof deps.renderChannels === 'function') deps.renderChannels();
    }

    async function api(path, options) {
      if (!localNodeApi) throw new Error('本机节点 API 不可用');
      return localNodeApi(path, options || {});
    }

    async function withBusy(id, label, task) {
      const button = $(id);
      const text = button && button.textContent;
      if (button) {
        button.disabled = true;
        button.textContent = label;
      }
      try {
        await task();
      } finally {
        if (button) {
          button.disabled = false;
          button.textContent = text;
        }
      }
    }

    function renderEnvGrid(env) {
      const modelInput = $('#nodeApiRuntimeModel');
      const baseInput = $('#nodeApiRuntimeBase');
      if (modelInput && !modelInput.value && env.api_runtime_model) modelInput.value = env.api_runtime_model;
      if (baseInput && !baseInput.value && env.api_runtime_base) baseInput.value = env.api_runtime_base;
      const items = [
        ['Git', env.git, '代码操作'],
        ['JDK 17', env.java, 'Android 编译'],
        ['Node.js', env.node, '工具运行时'],
        ['npm', env.npm, '包管理'],
        ['Codex CLI', env.codex, 'AI 编码'],
        ['API Key', env.api_runtime_key || env.openai_key, 'Route A/B 鉴权'],
        ['Route B 模型', env.api_runtime_model_configured, env.api_runtime_model || '未配置'],
        ['Route B Runtime', env.api_runtime_ready, env.api_runtime_ready ? '可用' : '需要 Key + 模型'],
        ['Android SDK', env.android_sdk, 'APK 构建'],
        ['Ollama', env.ollama, '本地模型'],
        ['Claude CLI', env.claude, '可选'],
        ['Gemini CLI', env.gemini, '可选']
      ];
      const grid = $('#nodeEnvGrid');
      if (grid) grid.innerHTML = items.map(([name, ok, note]) => `
        <div class="${ok ? 'ok' : 'miss'}"><strong>${escapeHtml(name)}</strong><span>${ok ? '可用' : '未就绪'} · ${escapeHtml(note)}</span></div>
      `).join('');
      renderApiRuntimeContract(env.api_runtime_contract || env.apiRuntimeContract);
    }

    function renderApiRuntimeContract(contract) {
      const target = $('#nodeApiRuntimeContract');
      if (!target) return;
      const supported = arrayStrings(contract && (contract.supported_tools || contract.supportedTools));
      if (!supported.length) {
        target.innerHTML = '<div class="node-contract-empty">Route B 工具契约未上报，刷新节点或更新客户端。</div>';
        return;
      }
      const approvals = arrayStrings(contract.approval_required_tools || contract.approvalRequiredTools);
      const readOnly = arrayStrings(contract.read_only_tools || contract.readOnlyTools);
      const limitations = arrayStrings(contract.limitations);
      const core = ['search_files', 'file_info', 'read_file_range', 'apply_patch', 'run_command']
        .filter((tool) => supported.includes(tool));
      target.innerHTML = `
        <div class="node-contract-head">
          <strong>${escapeHtml(clean(contract.label) || 'Route B 本机 API runtime')}</strong>
          <span>${escapeHtml(clean(contract.route) || 'route_b_api_runtime')}</span>
        </div>
        <div class="node-contract-tags">
          ${supported.map((tool) => `<span>${escapeHtml(tool)}</span>`).join('')}
        </div>
        <div class="node-contract-lines">
          ${contractLine('核心能力', (core.length ? core : supported.slice(0, 4)).join(' / '))}
          ${contractLine('只读工具', readOnly.join(' / ') || '未上报')}
          ${contractLine('需要确认', approvals.join(' / ') || '审批策略未上报')}
          ${contractLine('命令策略', clean(contract.command_policy || contract.commandPolicy) || '未上报')}
          ${contractLine('恢复限制', clean(contract.recovery_policy || contract.recoveryPolicy) || '未上报')}
        </div>
        ${limitations.length ? `<ul class="node-contract-notes">${limitations.map((note) => `<li>${escapeHtml(note)}</li>`).join('')}</ul>` : ''}
      `;
    }

    function contractLine(label, value) {
      return `<div><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`;
    }

    function modelsTable(models) {
      if (!models.length) return '<div class="node-empty-panel">未发现本地模型，检查 Ollama / LM Studio 是否在运行。</div>';
      return `<div class="node-model-list">${models.map((model) => `
        <div><strong>${escapeHtml(clean(model.model_id || model.display_name) || '模型')}</strong><span>${escapeHtml(clean(model.provider) || 'local')} · ${escapeHtml(clean(model.price_per_1k_credits || model.price_per_1k) || '0')} credits/1k</span></div>
      `).join('')}</div>`;
    }

    function shortHash(value) {
      const text = clean(value);
      return text.length > 12 ? text.slice(0, 8) : text;
    }

    function clientUpdateLine(local, latest) {
      const installedSha = clean(local && (local.installed_git_sha || local.install && local.install.installed_git_sha));
      const remoteSha = clean(latest && (latest.gitSha || latest.git_sha));
      const remoteVersion = clean(latest && latest.version);
      const remoteSize = formatBytes(latest && (latest.windowsClientFileSize || latest.fileSize));
      const latestLabel = [
        remoteVersion ? `v${remoteVersion}` : '',
        remoteSize,
        remoteSha ? shortHash(remoteSha) : ''
      ].filter(Boolean).join(' · ');
      if (latest && latest.error) return '无法读取线上版本';
      if (!remoteSha) return '正在读取线上版本';
      if (!installedSha) return `可检查更新 · 最新 ${latestLabel}`;
      if (installedSha === remoteSha) return `客户端已是最新 · ${latestLabel}`;
      return `可更新 · 当前 ${shortHash(installedSha)} · 最新 ${latestLabel}`;
    }

    function clientLayoutLabel(status) {
      const value = clean(status).toLowerCase();
      if (value === 'clean') return '目录清爽';
      if (value === 'legacy_files_present') return '发现旧脚本';
      if (value === 'unexpected_entries') return '存在额外文件';
      if (value === 'incomplete') return '安装不完整';
      if (value === 'unsupported') return '非 Win 维护环境';
      return '目录状态未知';
    }

    function clientStartMenuLine(product, startMenu) {
      const folder = clean(startMenu && (startMenu.folder || startMenu.folder_name)) || clean(product && product.start_menu_folder_name);
      const entriesSource = Array.isArray(startMenu && startMenu.entry_names)
        ? startMenu.entry_names
        : (Array.isArray(product && product.start_menu_entries) ? product.start_menu_entries : []);
      const entries = entriesSource.map(clean).filter(Boolean).slice(0, 5).join(' / ');
      const status = clientStartMenuStatusLabel(clean((startMenu && startMenu.status) || (product && product.start_menu_status)));
      const missing = Number((product && product.missing_start_menu_entry_count)
        || (Array.isArray(startMenu && startMenu.missing_entries) ? startMenu.missing_entries.length : 0)
        || 0);
      const parts = [];
      if (entries) parts.push(entries);
      if (status) parts.push(status);
      if (missing > 0) parts.push(`缺 ${missing} 个入口`);
      if (folder) return `${folder}${parts.length ? ' · ' + parts.join(' · ') : ''}`;
      return parts.join(' · ');
    }

    function clientRecommendedActionsLine(product) {
      const actions = Array.isArray(product && product.recommended_actions)
        ? product.recommended_actions.map(clean).filter(Boolean)
        : [];
      return actions.slice(0, 3).join(' / ');
    }

    function clientStartMenuStatusLabel(status) {
      const value = clean(status).toLowerCase();
      if (!value || value === 'unknown') return '';
      if (value === 'clean') return '入口完整';
      if (value === 'missing') return '开始菜单文件夹缺失';
      if (value === 'incomplete') return '维护入口不完整';
      return value;
    }

    function hardwareLine(hardware) {
      if (!hardware) return '未探测';
      return [
        clean(hardware.cpu_brand),
        hardware.cpu_cores ? `${hardware.cpu_cores} 核` : '',
        formatBytes(hardware.memory_total_bytes) ? `内存 ${formatBytes(hardware.memory_total_bytes)}` : '',
        (hardware.gpu_names || []).length ? `GPU ${(hardware.gpu_names || []).join(' / ')}` : ''
      ].filter(Boolean).join(' · ') || '未探测';
    }

    function storageLine(storage) {
      if (!storage || !storage.enabled) return '未启用';
      const rootPath = clean(storage.root_path) || '默认目录';
      const free = formatBytes(storage.disk_free_bytes);
      return `已启用 · ${rootPath}${free ? ` · 剩余 ${free}` : ''}`;
    }

    function row(label, value) {
      return `<div><span>${escapeHtml(label)}</span><strong>${escapeHtml(clean(value) || '未上报')}</strong></div>`;
    }

    function arrayStrings(value) {
      return Array.isArray(value)
        ? value.map((item) => clean(item)).filter(Boolean)
        : [];
    }

    function setResult(id, message, kind) {
      const el = $(id);
      if (!el) return;
      el.innerHTML = message ? `<div class="${kind === 'error' ? 'error' : 'ok'}">${escapeHtml(message)}</div>` : '';
    }

    function formatBytes(value) {
      if (deps.formatBytes) return deps.formatBytes(value);
      const bytes = Number(value || 0);
      if (!bytes) return '';
      if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
      if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
      if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
      return `${bytes} B`;
    }

    return { render };
  }

  window.ElonPcNodeAdmin = { create: createNativeNodeAdmin };
})();
