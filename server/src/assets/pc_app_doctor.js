(function () {
  const markdown = window.ElonPcMarkdown || {};
  const TOOL_SECTIONS = [
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
    state.doctorSessions = Array.isArray(state.doctorSessions) ? state.doctorSessions : [];
    state.doctorActiveSessionId = state.doctorActiveSessionId || '';
    state.doctorSessionsLoaded = !!state.doctorSessionsLoaded;
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
      const sessions = Array.isArray(state.doctorSessions) ? state.doctorSessions : [];
      const sessionList = sessions.length
        ? sessions.map(renderDoctorSessionButton).join('')
        : `<div class="empty-state doctor-session-empty">${state.doctorSessionsLoaded ? '暂无诊断会话' : '正在读取会话…'}</div>`;
      els.channelList.innerHTML = [
        '<div class="channel-section">电脑医生项目</div>',
        `<button class="channel-item doctor-new-session" type="button" data-doctor-new-session="1">
          <span class="glyph">+</span>
          <span class="main"><strong>新诊断</strong><span>开启一次独立排查</span></span>
        </button>`,
        '<div class="channel-section">诊断会话</div>',
        sessionList,
        '<div class="channel-section">工具</div>',
        TOOL_SECTIONS.map((section) => channelButton({
          id: section.id,
          kind: 'doctor-section',
          glyph: section.glyph,
          title: section.title,
          sub: section.sub,
          active: state.doctorSection === section.id
        })).join('')
      ].join('');
      els.channelList.querySelector('[data-doctor-new-session]')?.addEventListener('click', createDoctorSession);
      els.channelList.querySelectorAll('[data-doctor-session-id]').forEach((btn) => {
        btn.addEventListener('click', () => selectDoctorSession(btn.dataset.doctorSessionId));
      });
      els.channelList.querySelectorAll('[data-doctor-section]').forEach((btn) => {
        btn.addEventListener('click', () => selectSection(btn.dataset.doctorSection));
      });
    }

    function renderDoctorSessionButton(session) {
      const id = clean(session && session.id);
      const title = clean(session && session.title) || '未命名诊断';
      const messageCount = Number(session && session.messageCount) || 0;
      const updated = Number(session && session.updatedAtMs) || 0;
      const sub = messageCount ? `${messageCount} 条消息 · ${formatDoctorTime(updated)}` : '空会话';
      return `<button class="channel-item doctor-session-item ${id === state.doctorActiveSessionId ? 'active' : ''}" type="button" data-doctor-session-id="${escapeHtml(id)}">
        <span class="glyph">#</span>
        <span class="main"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(sub)}</span></span>
      </button>`;
    }

    function formatDoctorTime(value) {
      if (!value) return '';
      const date = new Date(value);
      if (Number.isNaN(date.getTime())) return '';
      return date.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' });
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
      loadDoctorSessions(true);
      if (state.doctorMemories === null) loadDoctorMemory(true);
    }

    function selectSection(section) {
      state.doctorSection = section || 'diagnosis';
      deps.renderChannels();
      const target = els.messageList.querySelector(`[data-doctor-panel="${state.doctorSection}"]`);
      if (target) target.scrollIntoView({ block: 'start', behavior: 'smooth' });
    }

    async function createDoctorSession() {
      state.doctorSection = 'diagnosis';
      state.doctorResult = { kind: '', text: '正在创建新的诊断会话…' };
      state.doctorMessages = [];
      state.doctorProblem = '';
      state.doctorAnalysis = '';
      renderDoctorMain();
      try {
        const data = await doctorApi('/api/doctor/sessions', {
          method: 'POST',
          body: JSON.stringify({ title: '新的电脑诊断' })
        });
        if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
        applyDoctorSession(data.session || null);
        state.doctorResult = { kind: 'ok', text: '新的诊断会话已创建。' };
      } catch (error) {
        state.doctorResult = { kind: 'err', text: `创建诊断会话失败：${error.message || error}` };
      }
      deps.renderChannels();
      renderDoctorMain();
    }

    async function selectDoctorSession(sessionId) {
      const id = clean(sessionId);
      if (!id) return;
      state.doctorActiveSessionId = id;
      state.doctorSection = 'diagnosis';
      state.doctorResult = { kind: '', text: '正在读取诊断会话…' };
      deps.renderChannels();
      renderDoctorMain();
      await loadDoctorSession(id);
    }

    async function loadDoctorSessions(quiet) {
      try {
        const data = await doctorApi('/api/doctor/sessions');
        state.doctorSessions = Array.isArray(data.items) ? data.items : [];
        state.doctorSessionsLoaded = true;
        if (!state.doctorActiveSessionId && state.doctorSessions.length) {
          await loadDoctorSession(state.doctorSessions[0].id);
          return;
        }
      } catch (error) {
        state.doctorSessions = [];
        state.doctorSessionsLoaded = true;
        if (!quiet) state.doctorResult = { kind: 'err', text: `读取诊断会话失败：${error.message || error}` };
      }
      if (state.activeKind === 'doctor') {
        deps.renderChannels();
        renderDoctorMain();
      }
    }

    async function loadDoctorSession(sessionId) {
      const id = clean(sessionId);
      if (!id) return;
      try {
        const data = await doctorApi(`/api/doctor/sessions/${encodeURIComponent(id)}`);
        applyDoctorSession(data.session || null);
        state.doctorResult = null;
      } catch (error) {
        state.doctorResult = { kind: 'err', text: `读取诊断会话失败：${error.message || error}` };
      }
      if (state.activeKind === 'doctor') {
        deps.renderChannels();
        renderDoctorMain();
      }
    }

    function applyDoctorSession(session) {
      if (!session || !session.id) return;
      state.doctorActiveSessionId = clean(session.id);
      state.doctorMessages = normalizeDoctorMessages(session.messages || []);
      state.doctorProblem = latestDoctorMessage('user');
      state.doctorAnalysis = latestDoctorMessage('assistant', 'ok');
    }

    function normalizeDoctorMessages(messages) {
      return (Array.isArray(messages) ? messages : []).map((message) => {
        const createdAtMs = Number(message.createdAtMs) || Date.now();
        return {
          id: clean(message.id),
          role: message.role === 'user' ? 'user' : 'assistant',
          content: clean(message.content),
          kind: clean(message.kind),
          createdAtMs,
          time: formatDoctorTime(createdAtMs)
        };
      });
    }

    function latestDoctorMessage(role, kind) {
      for (let i = state.doctorMessages.length - 1; i >= 0; i -= 1) {
        const message = state.doctorMessages[i];
        if (message.role === role && (!kind || message.kind === kind)) return clean(message.content);
      }
      return '';
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
      const createdAtMs = Date.now();
      const message = {
        id: `local-${createdAtMs}-${state.doctorMessages.length + 1}`,
        role,
        content,
        kind: kind || '',
        createdAtMs,
        time: formatDoctorTime(createdAtMs)
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
          body: JSON.stringify({ problem, sessionId: state.doctorActiveSessionId || null })
        });
        state.doctorAnalysis = data.analysis || '';
        state.doctorSnapshot = data.snapshot || state.doctorSnapshot;
        if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
        if (data.session) {
          applyDoctorSession(data.session);
        } else {
          assistantMessage.content = state.doctorAnalysis || '远程 AI 已完成分析。';
          assistantMessage.kind = 'ok';
        }
        state.doctorResult = { kind: 'ok', text: '远程 AI 已完成分析。' };
      } catch (error) {
        const data = error.data || {};
        const errorText = data.error || error.message || String(error);
        if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
        if (data.session) {
          applyDoctorSession(data.session);
        } else {
          assistantMessage.content = `远程 AI 分析失败：${errorText}`;
          assistantMessage.kind = 'err';
        }
        state.doctorResult = { kind: 'err', text: `远程 AI 分析失败：${errorText}` };
      }
      deps.renderChannels();
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
          body: JSON.stringify({
            action,
            confirm: true,
            adapterName: adapterName || null,
            sessionId: state.doctorActiveSessionId || null
          })
        });
        const outcome = data.outcome || {};
        const text = [
          outcome.stdout ? `stdout:\n${outcome.stdout}` : '',
          outcome.stderr ? `stderr:\n${outcome.stderr}` : '',
          outcome.error ? `error:\n${outcome.error}` : ''
        ].filter(Boolean).join('\n\n') || '完成';
        state.doctorResult = { kind: 'ok', text: `${data.title || label} 已执行。\n\n${text}` };
        if (state.doctorActiveSessionId) loadDoctorSession(state.doctorActiveSessionId);
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
