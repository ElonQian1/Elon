(function () {
  const markdown = window.ElonPcMarkdown || {};
  const SECTIONS = [
    { id: 'diagnosis', glyph: '医', title: '诊断对话', sub: '描述问题并让远程 AI 分析' },
    { id: 'snapshot', glyph: '查', title: '只读体检', sub: '网络、代理、DNS、服务状态' },
    { id: 'repair', glyph: '修', title: '白名单修复', sub: '清 DNS、代理、重启网卡' },
    { id: 'memory', glyph: '记', title: '问题记忆', sub: '常见电脑问题复用' }
  ];

  function createDoctorController(deps) {
    const { state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer, setRails, setDoctorMode } = deps;
    state.doctorSection = state.doctorSection || 'diagnosis';
    state.doctorProblem = state.doctorProblem || '';
    state.doctorSnapshot = state.doctorSnapshot || null;
    state.doctorAnalysis = state.doctorAnalysis || '';
    state.doctorMemories = state.doctorMemories || null;
    state.doctorResult = state.doctorResult || null;
    state.doctorMessages = Array.isArray(state.doctorMessages) ? state.doctorMessages : [];

    function localNodeApiUrl(path) {
      const base = state.nodeAdminUrl.endsWith('/') ? state.nodeAdminUrl : `${state.nodeAdminUrl}/`;
      return new URL(String(path || '').replace(/^\//, ''), base).toString();
    }

    async function doctorApi(path, options) {
      const opts = Object.assign({}, options || {});
      if (opts.body && !opts.headers) opts.headers = { 'Content-Type': 'application/json' };
      const resp = await fetch(localNodeApiUrl(path), opts);
      const text = await resp.text();
      const data = text ? JSON.parse(text) : {};
      if (!resp.ok || data.ok === false) {
        const error = new Error(data.error || data.message || `HTTP ${resp.status}`);
        error.data = data;
        throw error;
      }
      return data;
    }

    function renderChannels(channelButton) {
      els.channelList.innerHTML = [
        '<div class="channel-section">电脑医生项目</div>',
        SECTIONS.map((section) => channelButton({
          id: section.id,
          kind: 'doctor-section',
          glyph: section.glyph,
          title: section.title,
          sub: section.sub,
          active: state.doctorSection === section.id
        })).join('')
      ].join('');
      els.channelList.querySelectorAll('[data-doctor-section]').forEach((btn) => {
        btn.addEventListener('click', () => selectSection(btn.dataset.doctorSection));
      });
    }

    function selectDoctor() {
      state.activeKind = 'doctor';
      state.activeProjectId = '';
      state.activeChannelId = '';
      state.activePeer = null;
      setRails('doctor');
      els.workspaceName.textContent = '电脑医生';
      els.workspaceMeta.textContent = '独立本机维护项目';
      setHeader('医', '电脑医生', '诊断 Windows 网络、代理、DNS 和常见系统设置');
      setComposer(true, '描述电脑问题，按 Enter 发送给电脑医生', false);
      deps.renderChannels();
      renderDoctorMain();
      renderMembers('电脑医生权限', [
        { name: '只读体检', sub: '默认读取系统状态' },
        { name: '远程 AI 分析', sub: '使用登录态调用云端模型' },
        { name: '白名单修复', sub: '执行前二次确认' },
        { name: '问题记忆', sub: '保存在本机节点' }
      ]);
      if (state.doctorMemories === null) loadDoctorMemory(true);
    }

    function selectSection(section) {
      state.doctorSection = section || 'diagnosis';
      deps.renderChannels();
      const target = els.messageList.querySelector(`[data-doctor-panel="${state.doctorSection}"]`);
      if (target) target.scrollIntoView({ block: 'start', behavior: 'smooth' });
    }

    function syncDoctorProblem() {
      return clean(state.doctorProblem);
    }

    function doctorStatusHtml() {
      if (!state.doctorResult) {
        return '<div class="doctor-output">直接使用底部消息发送框描述问题；电脑医生会先采集只读快照，再让远程 AI 分析。</div>';
      }
      return `<div class="doctor-output ${escapeHtml(state.doctorResult.kind || '')}">${escapeHtml(state.doctorResult.text || '')}</div>`;
    }

    function doctorSnapshotHtml() {
      if (!state.doctorSnapshot) return '尚未体检';
      return escapeHtml(JSON.stringify(state.doctorSnapshot, null, 2));
    }

    function doctorMemoryHtml() {
      if (state.doctorMemories === null) return '<div class="doctor-memory">加载中…</div>';
      const items = Array.isArray(state.doctorMemories) ? state.doctorMemories.slice(0, 6) : [];
      if (!items.length) return '<div class="doctor-memory">暂无电脑问题记忆</div>';
      return `<div class="doctor-memory">${items.map((item) => {
        const time = item.createdAtMs ? new Date(Number(item.createdAtMs)).toLocaleString('zh-CN') : '';
        return `<div class="doctor-memory-item">
          <time>${escapeHtml(time)}</time>
          <strong>${escapeHtml(item.problem || '')}</strong>
          <span>${escapeHtml((item.summary || '').slice(0, 180))}</span>
        </div>`;
      }).join('')}</div>`;
    }

    function doctorActorName(message) {
      if (message.role === 'user') {
        return clean(state.user && (state.user.nickname || state.user.account || state.user.phone || state.user.email)) || '你';
      }
      return '电脑医生';
    }

    function doctorMessageGlyph(message) {
      return message.role === 'user' ? '你' : '医';
    }

    function doctorConversationHtml() {
      if (!state.doctorMessages.length) {
        return `<div class="doctor-conversation-empty">
          用底部的消息发送框描述电脑问题，例如“网页打不开但微信能用”或“代理关不掉”。
        </div>`;
      }
      return `<div class="doctor-conversation">${state.doctorMessages.map((message) => {
        const name = doctorActorName(message);
        const tone = message.role === 'assistant' ? ` ai ${message.kind || ''}` : '';
        const contentHtml = message.role === 'assistant' && markdown.renderMessage
          ? markdown.renderMessage(message.content || '', { className: tone, copy: true })
          : `<div class="message-content${tone}">${escapeHtml(message.content || '')}</div>`;
        return `<article class="message-row doctor-message-row">
          <div class="message-avatar fallback"><span>${escapeHtml(doctorMessageGlyph(message))}</span></div>
          <div class="message-body">
            <div class="message-meta"><strong>${escapeHtml(name)}</strong><span>${escapeHtml(message.time || '')}</span></div>
            ${contentHtml}
          </div>
        </article>`;
      }).join('')}</div>`;
    }

    function appendDoctorMessage(role, content, kind) {
      const message = {
        role,
        content,
        kind: kind || '',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
      };
      state.doctorMessages.push(message);
      return message;
    }

    function renderDoctorMain() {
      setDoctorMode(true);
      els.messageList.innerHTML = `<div class="doctor-page">
        <section class="doctor-hero">
          <div>
            <div class="doctor-kicker">Windows PC Project</div>
            <h2>电脑医生</h2>
            <p>它现在是 PC 工作台左侧的独立项目入口，不再藏在节点注册管理页里。默认只读诊断，修复动作只走白名单并要求确认。</p>
          </div>
          <button class="text-button" type="button" id="openDoctorLocalBtn">本机后台</button>
        </section>

        <section class="doctor-panel" data-doctor-panel="diagnosis">
          <h3>诊断对话</h3>
          <div class="doctor-actions">
            <button class="text-button" id="doctorSnapshotBtn" type="button">只读体检</button>
            <button class="text-button" id="doctorAnalyzeBtn" type="button">分析最近问题</button>
            <button class="text-button" id="doctorMemorySaveBtn" type="button">保存为问题记忆</button>
          </div>
          ${doctorConversationHtml()}
          ${doctorStatusHtml()}
        </section>

        <div class="doctor-grid">
          <section class="doctor-panel" data-doctor-panel="snapshot">
            <h3>只读系统快照</h3>
            <p>读取网络、代理、DNS、Windows 服务等系统状态，不执行修改。</p>
            <pre class="doctor-snapshot">${doctorSnapshotHtml()}</pre>
          </section>

          <section class="doctor-panel" data-doctor-panel="repair">
            <h3>白名单修复</h3>
            <p>这些动作会修改本机网络状态，执行前会再次确认；重启网卡可能需要管理员权限。</p>
            <div class="doctor-field">
              <label for="doctorAdapterName">网卡名称</label>
              <input id="doctorAdapterName" placeholder="如：Wi-Fi 或 以太网" />
            </div>
            <div class="doctor-repair-list">
              <button class="text-button" data-doctor-repair="flush_dns" type="button">清 DNS 缓存</button>
              <button class="text-button" data-doctor-repair="reset_winhttp_proxy" type="button">重置 WinHTTP 代理</button>
              <button class="text-button" data-doctor-repair="clear_user_proxy" type="button">关闭当前用户代理</button>
              <button class="text-button" data-doctor-repair="restart_adapter" type="button">重启指定网卡</button>
            </div>
          </section>
        </div>

        <section class="doctor-panel" data-doctor-panel="memory">
          <div class="doctor-actions">
            <h3 style="margin-right:auto">常见问题记忆</h3>
            <button class="text-button" id="doctorMemoryRefreshBtn" type="button">刷新记忆</button>
          </div>
          ${doctorMemoryHtml()}
        </section>
      </div>`;

      $('openDoctorLocalBtn')?.addEventListener('click', () => window.open(`${state.nodeAdminUrl}#doctor`, '_blank'));
      $('doctorSnapshotBtn')?.addEventListener('click', loadDoctorSnapshot);
      $('doctorAnalyzeBtn')?.addEventListener('click', () => doctorAnalyze());
      $('doctorMemorySaveBtn')?.addEventListener('click', saveDoctorMemory);
      $('doctorMemoryRefreshBtn')?.addEventListener('click', () => loadDoctorMemory(false));
      els.messageList.querySelectorAll('[data-doctor-repair]').forEach((btn) => {
        btn.addEventListener('click', () => doctorRepair(btn.dataset.doctorRepair));
      });
      if (markdown.bindCopyButtons) markdown.bindCopyButtons(els.messageList);
    }

    async function loadDoctorSnapshot() {
      state.doctorResult = { kind: '', text: '正在采集只读系统快照…' };
      renderDoctorMain();
      try {
        const data = await doctorApi('/api/doctor/snapshot');
        state.doctorSnapshot = data.snapshot || null;
        const count = Array.isArray(data.snapshot && data.snapshot.commands) ? data.snapshot.commands.length : 0;
        state.doctorResult = { kind: 'ok', text: `只读体检完成，已采集 ${count} 组系统状态。` };
      } catch (error) {
        state.doctorResult = { kind: 'err', text: `只读体检失败：${error.message || error}` };
      }
      renderDoctorMain();
    }

    async function doctorAnalyze(problemFromComposer) {
      const problem = clean(typeof problemFromComposer === 'string' ? problemFromComposer : '') || syncDoctorProblem();
      if (!problem) {
        state.doctorResult = { kind: 'err', text: '请先描述电脑问题。' };
        renderDoctorMain();
        return;
      }
      state.doctorSection = 'diagnosis';
      state.doctorProblem = problem;
      appendDoctorMessage('user', problem);
      const assistantMessage = appendDoctorMessage('assistant', '正在采集只读快照，并请求远程 AI 分析…');
      state.doctorResult = { kind: '', text: '正在采集只读快照，并请求远程 AI 分析…' };
      renderDoctorMain();
      try {
        const data = await doctorApi('/api/doctor/analyze', {
          method: 'POST',
          body: JSON.stringify({ problem })
        });
        state.doctorAnalysis = data.analysis || '';
        state.doctorSnapshot = data.snapshot || state.doctorSnapshot;
        assistantMessage.content = state.doctorAnalysis || '远程 AI 已完成分析。';
        assistantMessage.kind = 'ok';
        state.doctorResult = { kind: 'ok', text: '远程 AI 已完成分析。' };
      } catch (error) {
        assistantMessage.content = `远程 AI 分析失败：${error.message || error}`;
        assistantMessage.kind = 'err';
        state.doctorResult = { kind: 'err', text: assistantMessage.content };
      }
      renderDoctorMain();
    }

    async function sendComposerMessage(content) {
      await doctorAnalyze(content);
    }

    async function doctorRepair(action) {
      const labels = {
        flush_dns: '清 DNS 缓存',
        reset_winhttp_proxy: '重置 WinHTTP 代理',
        clear_user_proxy: '关闭当前用户代理',
        restart_adapter: '重启指定网卡'
      };
      const adapterName = clean(($('doctorAdapterName') && $('doctorAdapterName').value) || '');
      if (action === 'restart_adapter' && !adapterName) {
        state.doctorSection = 'repair';
        state.doctorResult = { kind: 'err', text: '请先填写网卡名称。' };
        renderDoctorMain();
        selectSection('repair');
        return;
      }
      const label = labels[action] || action;
      const suffix = action === 'restart_adapter' ? `：${adapterName}` : '';
      if (!window.confirm(`确认执行「${label}${suffix}」？该动作会修改本机网络状态。`)) return;
      state.doctorResult = { kind: '', text: `正在执行：${label}…` };
      renderDoctorMain();
      try {
        const data = await doctorApi('/api/doctor/repair', {
          method: 'POST',
          body: JSON.stringify({ action, confirm: true, adapterName: adapterName || null })
        });
        const outcome = data.outcome || {};
        const text = [
          outcome.stdout ? `stdout:\n${outcome.stdout}` : '',
          outcome.stderr ? `stderr:\n${outcome.stderr}` : '',
          outcome.error ? `error:\n${outcome.error}` : ''
        ].filter(Boolean).join('\n\n') || '完成';
        state.doctorResult = { kind: 'ok', text: `${data.title || label} 已执行。\n\n${text}` };
        setTimeout(loadDoctorSnapshot, 800);
      } catch (error) {
        const outcome = error.data && error.data.outcome ? error.data.outcome : {};
        const detail = [
          outcome.stdout ? `stdout:\n${outcome.stdout}` : '',
          outcome.stderr ? `stderr:\n${outcome.stderr}` : '',
          outcome.error ? `error:\n${outcome.error}` : ''
        ].filter(Boolean).join('\n\n');
        state.doctorResult = { kind: 'err', text: `修复失败：${error.message || error}${detail ? `\n\n${detail}` : ''}` };
      }
      renderDoctorMain();
    }

    async function loadDoctorMemory(quiet) {
      try {
        const data = await doctorApi('/api/doctor/memory');
        state.doctorMemories = Array.isArray(data.items) ? data.items : [];
      } catch (error) {
        state.doctorMemories = [];
        if (!quiet) state.doctorResult = { kind: 'err', text: `读取问题记忆失败：${error.message || error}` };
      }
      if (state.activeKind === 'doctor') renderDoctorMain();
    }

    async function saveDoctorMemory() {
      const problem = syncDoctorProblem();
      const summary = clean(state.doctorAnalysis);
      if (!problem || !summary) {
        state.doctorResult = { kind: 'err', text: '需要先填写问题并完成一次远程 AI 分析，才能保存为问题记忆。' };
        renderDoctorMain();
        return;
      }
      try {
        await doctorApi('/api/doctor/memory', {
          method: 'POST',
          body: JSON.stringify({ problem, summary })
        });
        state.doctorResult = { kind: 'ok', text: '已保存为电脑问题记忆。' };
        await loadDoctorMemory(true);
      } catch (error) {
        state.doctorResult = { kind: 'err', text: `保存问题记忆失败：${error.message || error}` };
        renderDoctorMain();
      }
    }

    return {
      renderChannels,
      selectDoctor,
      selectSection,
      sendComposerMessage
    };
  }

  window.ElonPcDoctor = { create: createDoctorController };
})();
