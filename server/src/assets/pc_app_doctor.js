(function () {
  const markdown = window.ElonPcMarkdown || {};
  const TOOL_SECTIONS = [
    { id: 'snapshot', glyph: '查', title: '体检快照', sub: '把系统快照写入会话' },
    { id: 'repair', glyph: '修', title: '修复动作', sub: '清 DNS、代理、重启网卡' },
    { id: 'memory', glyph: '记', title: '问题记忆', sub: '保存可复用结论' }
  ];

  function createDoctorController(deps) {
    const { state, els, $, clean, escapeHtml, setHeader, setComposer, setRails, setDoctorMode } = deps;
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

    function currentSessionSummary() {
      return (state.doctorSessions || []).find((session) => clean(session.id) === state.doctorActiveSessionId) || null;
    }

    function currentSessionTitle() {
      const summary = currentSessionSummary();
      return clean(summary && summary.title) || clean(state.doctorProblem) || '新的电脑诊断';
    }

    async function ensureDoctorSession(title) {
      if (state.doctorActiveSessionId) return state.doctorActiveSessionId;
      const data = await doctorApi('/api/doctor/sessions', {
        method: 'POST',
        body: JSON.stringify({ title: clean(title) || '新的电脑诊断' })
      });
      if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
      applyDoctorSession(data.session || null);
      deps.renderChannels();
      return state.doctorActiveSessionId;
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
      loadDoctorSessions(true);
      if (state.doctorMemories === null) loadDoctorMemory(true);
    }

    function selectSection(section) {
      state.doctorSection = section || 'diagnosis';
      deps.renderChannels();
      renderDoctorMain();
      if (state.doctorSection === 'snapshot') loadDoctorSnapshot();
      if (state.doctorSection === 'memory') loadDoctorMemory(false);
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

    function renderDoctorSidebar() {
      const memoryCount = Array.isArray(state.doctorMemories) ? state.doctorMemories.length : 0;
      const messageCount = state.doctorMessages.length;
      const snapshotText = state.doctorSnapshot ? '已采集' : '未体检';
      els.memberPanelTitle.textContent = '电脑医生';
      els.memberList.innerHTML = `<div class="doctor-side">
        <div class="doctor-side-block">
          <span>当前会话</span>
          <strong>${escapeHtml(currentSessionTitle())}</strong>
          <small>${escapeHtml(messageCount ? `${messageCount} 条消息` : '还没有消息')}</small>
        </div>
        <div class="doctor-side-grid">
          <div><span>只读快照</span><strong>${escapeHtml(snapshotText)}</strong></div>
          <div><span>问题记忆</span><strong>${escapeHtml(`${memoryCount} 条`)}</strong></div>
        </div>
        <div class="doctor-side-block">
          <span>会话动作</span>
          <button class="text-button" id="doctorSideSnapshotBtn" type="button">只读体检</button>
          <button class="text-button" id="doctorSideAnalyzeBtn" type="button" ${state.doctorProblem ? '' : 'disabled'}>分析最近问题</button>
          <button class="text-button" id="doctorSideMemoryBtn" type="button" ${state.doctorAnalysis ? '' : 'disabled'}>保存为问题记忆</button>
        </div>
        <div class="doctor-side-block ${state.doctorSection === 'repair' ? 'active' : ''}">
          <span>白名单修复</span>
          <input id="doctorAdapterName" placeholder="网卡名称，如 Wi-Fi" />
          <button class="text-button" data-doctor-repair="flush_dns" type="button">清 DNS 缓存</button>
          <button class="text-button" data-doctor-repair="reset_winhttp_proxy" type="button">重置 WinHTTP 代理</button>
          <button class="text-button" data-doctor-repair="clear_user_proxy" type="button">关闭当前用户代理</button>
          <button class="text-button" data-doctor-repair="restart_adapter" type="button">重启指定网卡</button>
        </div>
        <div class="doctor-side-note">
          <strong>安全边界</strong>
          <span>默认只读；涉及网络、代理、DNS、网卡的修改动作都需要二次确认。</span>
        </div>
      </div>`;

      $('doctorSideSnapshotBtn')?.addEventListener('click', loadDoctorSnapshot);
      $('doctorSideAnalyzeBtn')?.addEventListener('click', () => doctorAnalyze());
      $('doctorSideMemoryBtn')?.addEventListener('click', saveDoctorMemory);
      els.memberList.querySelectorAll('[data-doctor-repair]').forEach((btn) => {
        btn.addEventListener('click', () => doctorRepair(btn.dataset.doctorRepair));
      });
    }

    function renderDoctorMain() {
      setDoctorMode(true);
      const subtitle = state.doctorActiveSessionId
        ? '同一会话会保留上下文；继续追问时会带上最近消息。'
        : '像 Codex 一样新建一个诊断会话，然后直接从底部输入问题。';
      els.messageList.innerHTML = `<div class="doctor-page doctor-chat-page">
        <section class="doctor-chat-header">
          <div>
            <div class="doctor-kicker">Windows PC Doctor Session</div>
            <h2>${escapeHtml(currentSessionTitle())}</h2>
            <p>${escapeHtml(subtitle)}</p>
          </div>
          <div class="doctor-chat-actions">
            <button class="text-button" type="button" id="doctorSnapshotBtn">只读体检</button>
            <button class="text-button" id="doctorAnalyzeBtn" type="button" ${state.doctorProblem ? '' : 'disabled'}>分析最近问题</button>
            <button class="text-button" id="doctorMemorySaveBtn" type="button" ${state.doctorAnalysis ? '' : 'disabled'}>保存记忆</button>
            <button class="text-button" type="button" id="openDoctorLocalBtn">本机后台</button>
          </div>
        </section>

        <section class="doctor-chat-feed" data-doctor-panel="diagnosis">
          ${doctorConversationHtml()}
          ${doctorStatusHtml()}
        </section>
      </div>`;

      $('openDoctorLocalBtn')?.addEventListener('click', () => window.open(`${state.nodeAdminUrl}#doctor`, '_blank'));
      $('doctorSnapshotBtn')?.addEventListener('click', loadDoctorSnapshot);
      $('doctorAnalyzeBtn')?.addEventListener('click', () => doctorAnalyze());
      $('doctorMemorySaveBtn')?.addEventListener('click', saveDoctorMemory);
      if (markdown.bindCopyButtons) markdown.bindCopyButtons(els.messageList);
      renderDoctorSidebar();
    }

    async function loadDoctorSnapshot() {
      state.doctorResult = { kind: '', text: '正在采集只读系统快照…' };
      renderDoctorMain();
      try {
        const sessionId = await ensureDoctorSession('只读系统体检');
        const data = await doctorApi(`/api/doctor/snapshot?sessionId=${encodeURIComponent(sessionId)}`);
        state.doctorSnapshot = data.snapshot || null;
        if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
        if (data.session) applyDoctorSession(data.session);
        const count = Array.isArray(data.snapshot && data.snapshot.commands) ? data.snapshot.commands.length : 0;
        state.doctorResult = { kind: 'ok', text: `只读体检完成，已写入当前会话，采集 ${count} 组系统状态。` };
      } catch (error) {
        state.doctorResult = { kind: 'err', text: `只读体检失败：${error.message || error}` };
      }
      deps.renderChannels();
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
        const sessionId = await ensureDoctorSession(`${label}${suffix}`);
        const data = await doctorApi('/api/doctor/repair', {
          method: 'POST',
          body: JSON.stringify({
            action,
            confirm: true,
            adapterName: adapterName || null,
            sessionId
          })
        });
        const outcome = data.outcome || {};
        const text = [
          outcome.stdout ? `stdout:\n${outcome.stdout}` : '',
          outcome.stderr ? `stderr:\n${outcome.stderr}` : '',
          outcome.error ? `error:\n${outcome.error}` : ''
        ].filter(Boolean).join('\n\n') || '完成';
        state.doctorResult = { kind: 'ok', text: `${data.title || label} 已执行。\n\n${text}` };
        if (state.doctorActiveSessionId) await loadDoctorSession(state.doctorActiveSessionId);
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
        const sessionId = await ensureDoctorSession(problem);
        const data = await doctorApi('/api/doctor/memory', {
          method: 'POST',
          body: JSON.stringify({ problem, summary, sessionId })
        });
        if (Array.isArray(data.sessions)) state.doctorSessions = data.sessions;
        if (data.session) applyDoctorSession(data.session);
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
