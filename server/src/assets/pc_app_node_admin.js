(function () {
  function createNativeNodeAdmin(deps) {
    const { state, $, clean, escapeHtml, localNodeApi, ensureLocalNodeLogin } = deps;
    let root = null;

    function render(surface, status) {
      root = surface;
      root.innerHTML = template(status || {});
      bind();
      loadStorageConfig(true);
      loadEnvCheck(true);
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
              <p class="node-admin-copy">从这里打开 PC 工作台的原生注册弹窗，选择本机文件夹后自动读取 Git 远端和分支。</p>
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
              <div class="node-admin-actions"><button class="node-action primary" type="button" id="nodeEnvInstall">一键安装 / 修复</button></div>
              <label class="node-form-field"><span>OPENAI_API_KEY</span><input id="nodeOpenAiKey" type="password" placeholder="sk-..." autocomplete="off" /></label>
              <div class="node-admin-actions"><button class="node-action" type="button" id="nodeSaveOpenAiKey">保存 API Key</button></div>
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
        if (typeof deps.openSettings === 'function') deps.openSettings();
      });
      $('#nodeStorageRefresh')?.addEventListener('click', () => loadStorageConfig(false));
      $('#nodeStorageSave')?.addEventListener('click', saveStorageConfig);
      $('#nodeEnvRefresh')?.addEventListener('click', () => loadEnvCheck(false));
      $('#nodeEnvInstall')?.addEventListener('click', installEnv);
      $('#nodeSaveOpenAiKey')?.addEventListener('click', saveOpenAiKey);
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
      if (!key) return setResult('nodeEnvResult', '请输入 API Key。', 'error');
      await withBusy('nodeSaveOpenAiKey', '保存中...', async () => {
        try {
          await api('/api/save-openai-key', { method: 'POST', body: JSON.stringify({ api_key: key }) });
          $('#nodeOpenAiKey').value = '';
          setResult('nodeEnvResult', 'API Key 已保存，Codex CLI 可立即使用。');
          await loadEnvCheck(true);
        } catch (error) {
          setResult('nodeEnvResult', error.message || error, 'error');
        }
      });
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
      const items = [
        ['Git', env.git, '代码操作'],
        ['JDK 17', env.java, 'Android 编译'],
        ['Node.js', env.node, '工具运行时'],
        ['npm', env.npm, '包管理'],
        ['Codex CLI', env.codex, 'AI 编码'],
        ['OPENAI_API_KEY', env.openai_key, 'Codex 鉴权'],
        ['Android SDK', env.android_sdk, 'APK 构建'],
        ['Ollama', env.ollama, '本地模型'],
        ['Claude CLI', env.claude, '可选'],
        ['Gemini CLI', env.gemini, '可选']
      ];
      $('#nodeEnvGrid').innerHTML = items.map(([name, ok, note]) => `
        <div class="${ok ? 'ok' : 'miss'}"><strong>${escapeHtml(name)}</strong><span>${ok ? '可用' : '未就绪'} · ${escapeHtml(note)}</span></div>
      `).join('');
    }

    function modelsTable(models) {
      if (!models.length) return '<div class="node-empty-panel">未发现本地模型，检查 Ollama / LM Studio 是否在运行。</div>';
      return `<div class="node-model-list">${models.map((model) => `
        <div><strong>${escapeHtml(clean(model.model_id || model.display_name) || '模型')}</strong><span>${escapeHtml(clean(model.provider) || 'local')} · ${escapeHtml(clean(model.price_per_1k_credits || model.price_per_1k) || '0')} credits/1k</span></div>
      `).join('')}</div>`;
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
