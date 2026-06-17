(function () {
  const kit = window.ElonPcKit || {};
  const { readToken, safeNodeAdminUrl, escapeHtml, clean, firstChar, formatTime } = kit;
  const TOKEN_KEYS = kit.TOKEN_KEYS || ['lodex_token', 'elon_token'];
  const $ = (id) => document.getElementById(id);
  const state = {
    token: readToken(), user: null, projects: [], friends: [], groups: [], nodes: [],
    activeKind: 'friends', activeProjectId: '', activeChannelId: '', activeChannelKind: '',
    activePeer: null, projectSpace: null,
    nodeAdminUrl: safeNodeAdminUrl()
  };

  const els = {
    friendsRail: $('friendsRail'), nodeRail: $('nodeRail'), projectRailList: $('projectRailList'),
    channelList: $('channelList'), memberList: $('memberList'), messageList: $('messageList'),
    workspaceName: $('workspaceName'), workspaceMeta: $('workspaceMeta'), channelGlyph: $('channelGlyph'),
    channelTitle: $('channelTitle'), channelSubtitle: $('channelSubtitle'), userName: $('userName'),
    userMeta: $('userMeta'), userDot: $('userDot'), friendBadge: $('friendBadge'), nodeBadge: $('nodeBadge'),
    sidebarSearch: $('sidebarSearch'), composer: $('composer'), input: $('messageInput'),
    sendBtn: $('sendBtn'), aiTaskBtn: $('aiTaskBtn'), memberPanelTitle: $('memberPanelTitle')
  };

  function saveToken(token) {
    state.token = token || '';
    if (state.token) {
      localStorage.setItem('lodex_token', state.token);
      localStorage.setItem('elon_token', state.token);
    }
  }

  function authHeaders(extra) {
    const headers = Object.assign({ 'Content-Type': 'application/json' }, extra || {});
    if (state.token) headers.Authorization = 'Bearer ' + state.token;
    return headers;
  }

  async function api(path, options) {
    const resp = await fetch(path, Object.assign({}, options || {}, {
      headers: authHeaders((options && options.headers) || {})
    }));
    if (resp.status === 401) {
      TOKEN_KEYS.forEach((key) => localStorage.removeItem(key));
      state.token = '';
      throw new Error('请先登录一龙账号');
    }
    const text = await resp.text();
    const data = text ? JSON.parse(text) : {};
    if (!resp.ok) throw new Error(data.error || data.message || resp.statusText);
    return data;
  }

  function titleOf(project) {
    return clean(project.display_name || project.displayName || project.alias || project.name || project.title) || '未命名项目';
  }

  function iconUrlOf(project) {
    return clean(project.icon_data_url || project.iconDataUrl || project.icon_url || project.iconUrl || project.logo || project.avatar);
  }

  function userName(user) {
    return clean(user && (user.nickname || user.phone || user.email || user.account || user.id)) || '未登录';
  }

  function setBadge(el, value) {
    if (!el) return;
    const n = Number(value || 0);
    el.textContent = n > 99 ? '99+' : String(n);
    el.classList.toggle('show', n > 0);
  }

  function setRails(kind) {
    els.friendsRail.classList.toggle('active', kind === 'friends');
    els.nodeRail.classList.toggle('active', kind === 'node');
    Array.from(els.projectRailList.children).forEach((btn) => {
      btn.classList.toggle('active', kind === 'project' && btn.dataset.projectId === state.activeProjectId);
    });
  }

  function renderUser() {
    const name = userName(state.user);
    els.userName.textContent = name;
    els.userMeta.textContent = state.token ? '在线' : '需要登录';
    els.userDot.textContent = firstChar(name, '龙');
  }

  function renderProjectRail() {
    els.projectRailList.innerHTML = state.projects.map((project) => {
      const title = titleOf(project);
      const icon = iconUrlOf(project);
      return `<button class="rail-avatar" type="button" data-project-id="${escapeHtml(project.id)}" title="${escapeHtml(title)}">
        <span>${escapeHtml(firstChar(title, '项'))}${icon ? `<img src="${escapeHtml(icon)}" alt="" onerror="this.remove()" />` : ''}</span>
      </button>`;
    }).join('');
    els.projectRailList.querySelectorAll('[data-project-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.projectId));
    });
  }

  function filterText() {
    return clean(els.sidebarSearch.value).toLowerCase();
  }

  function renderChannels() {
    const query = filterText();
    if (state.activeKind === 'friends') return renderFriendChannels(query);
    if (state.activeKind === 'node') return renderNodeChannels(query);
    return renderProjectChannels(query);
  }

  function renderFriendChannels(query) {
    const friends = state.friends.filter((f) => userName(f).toLowerCase().includes(query));
    const groups = state.groups.filter((g) => clean(g.name || g.title || g.id).toLowerCase().includes(query));
    els.channelList.innerHTML = [
      '<div class="channel-section">好友</div>',
      friends.map((friend) => channelButton({
        id: friend.id, kind: 'friend', glyph: '●', title: userName(friend),
        sub: friend.is_online ? '在线' : '离线', online: !!friend.is_online,
        active: state.activePeer && state.activePeer.kind === 'friend' && state.activePeer.id === friend.id
      })).join('') || '<div class="empty-state">暂无好友</div>',
      '<div class="channel-section">群聊</div>',
      groups.map((group) => channelButton({
        id: group.id, kind: 'group', glyph: '群', title: clean(group.name || group.title || '未命名群聊'),
        sub: `${Number(group.member_count || group.members_count || 0)} 位成员`,
        active: state.activePeer && state.activePeer.kind === 'group' && state.activePeer.id === group.id
      })).join('') || '<div class="empty-state">暂无群聊</div>'
    ].join('');
    els.channelList.querySelectorAll('[data-peer-kind]').forEach((btn) => {
      btn.addEventListener('click', () => selectPeer(btn.dataset.peerKind, btn.dataset.itemId));
    });
  }

  function renderNodeChannels() {
    const onlineCount = state.nodes.filter((node) => node.online).length;
    els.channelList.innerHTML = `
      <div class="channel-section">本机</div>
      ${channelButton({ id: 'local-node', kind: 'node', glyph: 'PC', title: '节点注册与电脑维护', sub: '融合本机管理页', active: true })}
      <div class="channel-section">我的节点</div>
      ${state.nodes.map((node) => channelButton({
        id: node.node_id || node.agent_id || '',
        kind: 'node-list',
        glyph: node.online ? '●' : '○',
        title: clean(node.display_name || node.device_name || node.short_id || node.node_id || 'PC 节点'),
        sub: node.online ? '在线' : '离线',
        online: !!node.online
      })).join('') || '<div class="empty-state">暂无节点</div>'}
      <div class="channel-section">状态</div>
      <div class="empty-state">${onlineCount}/${state.nodes.length} 台在线</div>`;
  }

  function renderProjectChannels(query) {
    const channels = ((state.projectSpace && state.projectSpace.channels) || [])
      .filter((channel) => channelName(channel).toLowerCase().includes(query));
    els.channelList.innerHTML = [
      '<div class="channel-section">频道</div>',
      channels.map((channel) => channelButton({
        id: channel.id,
        kind: 'project-channel',
        glyph: channelGlyph(channel),
        title: channelName(channel),
        sub: channel.kind || channel.channel_kind || '频道',
        active: channel.id === state.activeChannelId
      })).join('') || '<div class="empty-state">暂无频道</div>'
    ].join('');
    els.channelList.querySelectorAll('[data-channel-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProjectChannel(btn.dataset.channelId));
    });
  }

  function channelButton(item) {
    const attrs = item.kind === 'project-channel'
      ? `data-channel-id="${escapeHtml(item.id)}"`
      : `data-peer-kind="${escapeHtml(item.kind)}" data-item-id="${escapeHtml(item.id)}"`;
    return `<button class="channel-item ${item.active ? 'active' : ''}" type="button" ${attrs}>
      <span class="glyph">${escapeHtml(item.glyph || '#')}</span>
      <span class="main"><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.sub || '')}</span></span>
      ${typeof item.online === 'boolean' ? `<i class="presence-dot ${item.online ? 'online' : ''}"></i>` : ''}
    </button>`;
  }

  function channelName(channel) {
    return clean(channel.name || channel.title || channel.display_name || channel.id) || '频道';
  }

  function channelGlyph(channel) {
    const kind = clean(channel.kind || channel.channel_kind).toLowerCase();
    if (kind === 'ai_development') return 'AI';
    if (kind === 'announcements') return '!';
    if (kind === 'docs') return '文';
    return '#';
  }

  function setHeader(glyph, title, subtitle) {
    els.channelGlyph.textContent = glyph;
    els.channelTitle.textContent = title;
    els.channelSubtitle.textContent = subtitle || '';
  }

  function setComposer(enabled, placeholder, aiEnabled) {
    els.input.disabled = !enabled;
    els.sendBtn.disabled = !enabled;
    els.aiTaskBtn.disabled = !aiEnabled;
    els.aiTaskBtn.classList.toggle('enabled', !!aiEnabled);
    els.input.placeholder = placeholder || '输入消息';
  }

  function setNodeMode(enabled) { els.messageList.classList.toggle('node-mode', !!enabled); }

  async function init() {
    bindEvents();
    if (!state.token) {
      showLoginState();
      return;
    }
    await loadBaseData();
    selectFriends();
  }

  function bindEvents() {
    els.friendsRail.addEventListener('click', selectFriends);
    els.nodeRail.addEventListener('click', selectNode);
    $('refreshBtn').addEventListener('click', refreshActive);
    $('openWebBtn').addEventListener('click', () => window.open('/web', '_blank'));
    $('openLegacyWebBtn').addEventListener('click', () => window.open('/web', '_blank'));
    $('openLocalNodeBtn').addEventListener('click', () => window.open(state.nodeAdminUrl, '_blank'));
    $('logoutBtn').addEventListener('click', logout);
    els.sidebarSearch.addEventListener('input', renderChannels);
    els.composer.addEventListener('submit', (event) => {
      event.preventDefault();
      sendCurrentMessage(false);
    });
    els.aiTaskBtn.addEventListener('click', () => sendCurrentMessage(true));
    els.input.addEventListener('input', () => {
      els.input.style.height = '46px';
      els.input.style.height = Math.min(120, els.input.scrollHeight) + 'px';
    });
  }

  async function loadBaseData() {
    const [me, projects, friends, groups, nodes] = await Promise.allSettled([
      api('/api/me'),
      api('/api/me/projects?include_system=true'),
      api('/api/me/friends'),
      api('/api/me/groups'),
      api('/api/me/nodes')
    ]);
    state.user = valueOf(me).user || valueOf(me);
    state.projects = valueOf(projects).projects || [];
    state.friends = valueOf(friends).friends || [];
    state.groups = valueOf(groups).groups || [];
    state.nodes = valueOf(nodes).nodes || [];
    renderUser();
    renderProjectRail();
    setBadge(els.friendBadge, state.friends.filter((f) => f.is_online).length);
    setBadge(els.nodeBadge, state.nodes.filter((n) => n.online).length);
  }

  function valueOf(result) {
    if (result.status === 'fulfilled') return result.value || {};
    return {};
  }

  function showLoginState() {
    renderUser();
    setRails('friends');
    els.workspaceName.textContent = '一龙 PC 工作台';
    els.workspaceMeta.textContent = '未登录';
    setHeader('友', '需要登录', '先登录网页版，再回到 PC 工作台');
    setComposer(false, '登录后可输入消息', false);
    els.channelList.innerHTML = '<div class="empty-state">请先打开网页版登录账号</div>';
    els.memberList.innerHTML = '';
    setNodeMode(false);
    els.messageList.innerHTML = `<div class="empty-state">
      <strong>登录后一处使用好友、项目和 PC 节点</strong>
      <p>PC 工作台读取网页版登录态。点击下方按钮登录后，刷新本页即可进入 Discord 风格工作区。</p>
      <button class="text-button" type="button" id="loginWeb">打开网页版登录</button>
    </div>`;
    $('loginWeb').addEventListener('click', () => window.open('/web', '_blank'));
  }

  function selectFriends() {
    state.activeKind = 'friends';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = state.activePeer || null;
    setRails('friends');
    els.workspaceName.textContent = '好友';
    els.workspaceMeta.textContent = `${state.friends.length} 位好友 · ${state.groups.length} 个群聊`;
    renderChannels();
    renderMembers('好友在线', state.friends.map((f) => ({ name: userName(f), sub: f.is_online ? '在线' : '离线' })));
    if (state.activePeer) selectPeer(state.activePeer.kind, state.activePeer.id);
    else {
      setHeader('友', '好友列表', '选择左侧好友或群聊开始对话');
      setComposer(false, '选择好友或群聊后开始输入', false);
      setNodeMode(false);
      els.messageList.innerHTML = '<div class="empty-state"><strong>好友和群聊</strong><p>左侧第一枚图标固定打开好友列表。项目图标会排在 PC 节点下方。</p></div>';
    }
  }

  async function selectPeer(kind, id) {
    const list = kind === 'group' ? state.groups : state.friends;
    const item = list.find((entry) => String(entry.id) === String(id));
    if (!item) return;
    state.activeKind = 'friends';
    state.activePeer = { kind, id };
    renderChannels();
    const title = kind === 'group' ? clean(item.name || item.title || '群聊') : userName(item);
    setHeader(kind === 'group' ? '群' : '@', title, kind === 'group' ? '群聊频道' : (item.is_online ? '在线好友' : '离线好友'));
    setComposer(true, `发送给 ${title}`, false);
    setNodeMode(false);
    els.messageList.innerHTML = '<div class="empty-state">加载消息中…</div>';
    try {
      const path = kind === 'group'
        ? `/api/me/groups/${encodeURIComponent(id)}/messages?limit=100`
        : `/api/me/friends/${encodeURIComponent(id)}/messages?limit=100`;
      const data = await api(path);
      renderMessages(data.messages || [], kind);
    } catch (error) {
      showError(error);
    }
  }

  async function selectNode() {
    state.activeKind = 'node';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    setRails('node');
    els.workspaceName.textContent = 'PC 节点';
    els.workspaceMeta.textContent = '注册、电脑维护、节点状态';
    setHeader('PC', '本机节点注册页面', '节点管理页面已融合在 PC 工作台内');
    setComposer(false, '节点管理页中操作', false);
    renderChannels();
    renderNodeMain();
    renderMembers('我的节点', state.nodes.map((node) => ({
      name: clean(node.display_name || node.device_name || node.short_id || node.node_id || 'PC 节点'),
      sub: node.online ? '在线' : '离线'
    })));
  }

  function renderNodeMain() {
    setNodeMode(true);
    els.messageList.innerHTML = `<div class="node-toolbar">
      <div>
        <strong>本机节点管理</strong>
        <div class="node-status-line">来源：${escapeHtml(state.nodeAdminUrl)}。这里直接嵌入本机 agent 页面，不再让用户面对两个独立入口。</div>
      </div>
      <button class="text-button" type="button" id="openNodeFrame">新窗口打开</button>
    </div>
    <iframe class="node-frame" src="${escapeHtml(state.nodeAdminUrl)}" title="一龙 PC 节点本地管理"></iframe>`;
    $('openNodeFrame').addEventListener('click', () => window.open(state.nodeAdminUrl, '_blank'));
  }

  async function selectProject(projectId) {
    const project = state.projects.find((p) => String(p.id) === String(projectId));
    if (!project) return;
    state.activeKind = 'project';
    state.activeProjectId = String(projectId);
    state.activePeer = null;
    setRails('project');
    els.workspaceName.textContent = titleOf(project);
    els.workspaceMeta.textContent = project.role || '项目';
    setHeader('#', titleOf(project), '加载项目空间中…');
    setComposer(false, '加载项目频道中', false);
    setNodeMode(false);
    els.messageList.innerHTML = '<div class="empty-state">加载项目空间中…</div>';
    try {
      state.projectSpace = await api(`/api/projects/${encodeURIComponent(projectId)}/space`);
      const members = state.projectSpace.members || [];
      renderMembers('项目成员', members.map((m) => ({
        name: userName(m),
        sub: m.role || m.member_role || 'member'
      })));
      renderChannels();
      const first = (state.projectSpace.channels || [])[0];
      if (first) selectProjectChannel(first.id);
      else {
        setHeader('#', titleOf(project), '暂无频道');
        setNodeMode(false);
        els.messageList.innerHTML = '<div class="empty-state"><strong>暂无频道</strong><p>项目空间还没有可显示的频道。</p></div>';
      }
    } catch (error) {
      showError(error);
    }
  }

  async function selectProjectChannel(channelId) {
    const channel = ((state.projectSpace && state.projectSpace.channels) || [])
      .find((item) => String(item.id) === String(channelId));
    if (!channel) return;
    state.activeChannelId = String(channelId);
    state.activeChannelKind = clean(channel.kind || channel.channel_kind).toLowerCase();
    renderChannels();
    setHeader(channelGlyph(channel), channelName(channel), channel.kind || channel.channel_kind || '项目频道');
    const canWrite = state.activeChannelKind !== 'docs';
    setComposer(canWrite, canWrite ? `在 #${channelName(channel)} 发送消息` : '文档频道只读', state.activeChannelKind === 'ai_development');
    setNodeMode(false);
    els.messageList.innerHTML = '<div class="empty-state">加载频道消息中…</div>';
    try {
      const data = await api(`/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=120`);
      renderMessages(data.messages || [], 'project');
    } catch (error) {
      showError(error);
    }
  }

  function renderMembers(title, members) {
    els.memberPanelTitle.textContent = title;
    els.memberList.innerHTML = (members || []).map((member) => {
      const name = clean(member.name || member.nickname || member.account || member.user_account || member.phone || member.email) || '成员';
      const sub = clean(member.sub || member.role || member.status || member.id) || '';
      return `<div class="member-row"><div class="member-avatar">${escapeHtml(firstChar(name, '员'))}</div><div><strong>${escapeHtml(name)}</strong><span>${escapeHtml(sub)}</span></div></div>`;
    }).join('') || '<div class="empty-state">暂无成员</div>';
  }

  function renderMessages(messages, scope) {
    setNodeMode(false);
    if (!messages.length) {
      els.messageList.innerHTML = '<div class="empty-state"><strong>还没有消息</strong><p>从下方输入框发送第一条消息。</p></div>';
      return;
    }
    els.messageList.innerHTML = messages.map((message) => {
      const name = clean(message.sender_name || message.user_account || message.sender || message.author_name || message.from_name) ||
        (message.outgoing ? userName(state.user) : (scope === 'project' ? '项目成员' : '好友'));
      const role = clean(message.role || message.kind || message.message_kind);
      const tone = role.includes('assistant') || role.includes('ai') ? 'ai' : (role.includes('task') ? 'task' : '');
      return `<article class="message-row">
        <div class="message-avatar">${escapeHtml(firstChar(name, '员'))}</div>
        <div class="message-body">
          <div class="message-meta"><strong>${escapeHtml(name)}</strong><span>${escapeHtml(formatTime(message.created_at || message.createdAt))}</span></div>
          <div class="message-content ${tone}">${escapeHtml(message.content || message.text || message.message || '')}</div>
        </div>
      </article>`;
    }).join('');
    els.messageList.scrollTop = els.messageList.scrollHeight;
  }

  async function sendCurrentMessage(useAiTask) {
    const content = clean(els.input.value);
    if (!content) return;
    els.sendBtn.disabled = true;
    try {
      if (state.activeKind === 'friends' && state.activePeer) {
        const path = state.activePeer.kind === 'group'
          ? `/api/me/groups/${encodeURIComponent(state.activePeer.id)}/messages`
          : `/api/me/friends/${encodeURIComponent(state.activePeer.id)}/messages`;
        await api(path, { method: 'POST', body: JSON.stringify({ content }) });
        els.input.value = '';
        await selectPeer(state.activePeer.kind, state.activePeer.id);
      } else if (state.activeKind === 'project' && state.activeProjectId && state.activeChannelId) {
        const path = useAiTask
          ? `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/ai-tasks`
          : `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/messages`;
        await api(path, { method: 'POST', body: JSON.stringify({ content }) });
        els.input.value = '';
        await selectProjectChannel(state.activeChannelId);
      }
    } catch (error) {
      showError(error);
    } finally {
      els.sendBtn.disabled = false;
    }
  }

  async function refreshActive() {
    if (!state.token) return showLoginState();
    await loadBaseData();
    if (state.activeKind === 'node') return selectNode();
    if (state.activeKind === 'project' && state.activeProjectId) return selectProject(state.activeProjectId);
    return selectFriends();
  }

  function logout() {
    TOKEN_KEYS.forEach((key) => localStorage.removeItem(key));
    saveToken('');
    state.user = null;
    state.projects = [];
    state.friends = [];
    state.groups = [];
    state.nodes = [];
    showLoginState();
  }

  function showError(error) {
    setNodeMode(false);
    els.messageList.innerHTML = `<div class="empty-state"><strong>加载失败</strong><p>${escapeHtml(error.message || error)}</p></div>`;
  }

  window.addEventListener('storage', () => {
    const latest = readToken();
    if (latest && latest !== state.token) {
      saveToken(latest);
      refreshActive();
    }
  });

  init().catch(showError);
})();
