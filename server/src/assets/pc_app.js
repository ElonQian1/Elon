(function () {
  const kit = window.ElonPcKit || {};
  const { readToken, safeNodeAdminUrl, escapeHtml, clean, firstChar, formatTime } = kit;
  const markdown = window.ElonPcMarkdown || {};
  const TOKEN_KEYS = kit.TOKEN_KEYS || ['lodex_token', 'elon_token'];
  const $ = (id) => document.getElementById(id);
  const state = {
    token: readToken(), user: null, projects: [], friends: [], groups: [], nodes: [],
    activeKind: 'friends', activeProjectId: '', activeChannelId: '', activeChannelKind: '',
    activeVoiceChannel: 'studio',
    activePeer: null, projectSpace: null,
    nodeAdminUrl: safeNodeAdminUrl()
  };
  const PROJECT_SHARE_MARKER = '【一龙项目卡片】';

  const els = {
    friendsRail: $('friendsRail'), doctorRail: $('doctorRail'), nodeRail: $('nodeRail'), voiceRail: $('voiceRail'), projectRailList: $('projectRailList'),
    channelList: $('channelList'), memberList: $('memberList'), messageList: $('messageList'),
    workspaceName: $('workspaceName'), workspaceMeta: $('workspaceMeta'), channelGlyph: $('channelGlyph'),
    channelTitle: $('channelTitle'), channelSubtitle: $('channelSubtitle'), userName: $('userName'),
    userMeta: $('userMeta'), userDot: $('userDot'), friendBadge: $('friendBadge'), nodeBadge: $('nodeBadge'),
    sidebarSearch: $('sidebarSearch'), composer: $('composer'), input: $('messageInput'),
    sendBtn: $('sendBtn'), aiTaskBtn: $('aiTaskBtn'), memberPanelTitle: $('memberPanelTitle'),
    railTooltip: $('railTooltip'), userSettingsBtn: $('userSettingsBtn'), settingsBackdrop: $('settingsBackdrop'),
    settingsCloseBtn: $('settingsCloseBtn'), chooseProjectFolderBtn: $('chooseProjectFolderBtn'),
    inspectProjectFolderBtn: $('inspectProjectFolderBtn'), registerProjectBtn: $('registerProjectBtn'),
    settingsProjectPath: $('settingsProjectPath'), settingsProjectName: $('settingsProjectName'),
    settingsProjectDesc: $('settingsProjectDesc'), settingsProjectRepo: $('settingsProjectRepo'),
    settingsProjectBranch: $('settingsProjectBranch'), settingsProjectMeta: $('settingsProjectMeta'),
    settingsProjectResult: $('settingsProjectResult')
  };

  const doctor = window.ElonPcDoctor.create({
    state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer,
    setRails, renderChannels, setDoctorMode
  });
  const node = window.ElonPcNode.create({
    state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer,
    setRails, renderChannels, setNodeMode
  });

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

  function nodeAdminEndpoint(path) {
    const cleanPath = String(path || '').replace(/^\/+/, '');
    return new URL(cleanPath, state.nodeAdminUrl).toString();
  }

  async function localNodeApi(path, options) {
    const request = Object.assign({}, options || {});
    if (request.body && !request.headers) {
      request.headers = { 'Content-Type': 'application/json' };
    }
    let resp;
    try {
      resp = await fetch(nodeAdminEndpoint(path), request);
    } catch (error) {
      throw new Error(`无法连接本机 PC 节点 ${state.nodeAdminUrl}，请确认一龙 PC 节点正在运行并已更新。`);
    }
    const text = await resp.text();
    const data = text ? JSON.parse(text) : {};
    if (!resp.ok || data.ok === false) {
      throw new Error(data.error || data.message || resp.statusText);
    }
    return data;
  }

  function titleOf(project) {
    return clean(project.display_name || project.displayName || project.alias || project.name || project.title) || '未命名项目';
  }

  function iconUrlOf(project) {
    return clean(project.icon_data_url || project.iconDataUrl || project.icon_url || project.iconUrl || project.logo || project.avatar);
  }

  function avatarUrlOf(entity) {
    if (!entity) return '';
    return clean(entity.avatar_data_url || entity.avatarDataUrl ||
      entity.sender_avatar_data_url || entity.senderAvatarDataUrl ||
      entity.avatar_url || entity.avatarUrl ||
      entity.icon_data_url || entity.iconDataUrl ||
      entity.image_url || entity.imageUrl ||
      entity.avatar);
  }

  function avatarContents(url, label, fallback) {
    const source = clean(url);
    const initial = firstChar(label, fallback || '员');
    const image = source
      ? `<img src="${escapeHtml(source)}" alt="" onerror="this.remove(); this.parentElement.classList.add('fallback')" />`
      : '';
    return `${image}<span>${escapeHtml(initial)}</span>`;
  }

  function avatarElement(tag, className, url, label, fallback) {
    const source = clean(url);
    return `<${tag} class="${className}${source ? '' : ' fallback'}">${avatarContents(source, label, fallback)}</${tag}>`;
  }

  function sameId(left, right) {
    return String(left || '') === String(right || '');
  }

  function projectById(id) {
    return state.projects.find((project) => sameId(project && project.id, id)) || null;
  }

  function projectHue(project) {
    const seed = `${project.id || ''}:${titleOf(project)}`;
    let hash = 0;
    for (let i = 0; i < seed.length; i += 1) {
      hash = ((hash << 5) - hash + seed.charCodeAt(i)) | 0;
    }
    return 18 + (Math.abs(hash) % 318);
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
    els.doctorRail.classList.toggle('active', kind === 'doctor');
    els.nodeRail.classList.toggle('active', kind === 'node');
    els.voiceRail.classList.toggle('active', kind === 'voice');
    Array.from(els.projectRailList.children).forEach((btn) => {
      btn.classList.toggle('active', kind === 'project' && btn.dataset.projectId === state.activeProjectId);
    });
  }

  function renderUser() {
    const name = userName(state.user);
    const avatar = avatarUrlOf(state.user);
    els.userName.textContent = name;
    els.userMeta.textContent = state.token ? '在线' : '需要登录';
    els.userDot.classList.toggle('fallback', !avatar);
    els.userDot.innerHTML = avatarContents(avatar, name, '龙');
  }

  function renderProjectRail() {
    els.projectRailList.innerHTML = state.projects.map((project) => {
      const title = titleOf(project);
      const icon = iconUrlOf(project);
      const hue = projectHue(project);
      const iconMarkup = icon ? `<img src="${escapeHtml(icon)}" alt="" onerror="this.parentElement.classList.add('fallback'); this.remove()" />` : '';
      return `<button class="rail-avatar project" type="button" data-project-id="${escapeHtml(project.id)}" data-label="${escapeHtml(title)}" aria-label="${escapeHtml(title)}" style="--project-hue:${hue}">
        <span class="rail-icon ${icon ? '' : 'fallback'}" aria-hidden="true">${iconMarkup}</span>
      </button>`;
    }).join('');
    els.projectRailList.querySelectorAll('[data-project-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.projectId));
      attachRailTooltip(btn);
    });
  }

  function filterText() {
    return clean(els.sidebarSearch.value).toLowerCase();
  }

  function renderChannels() {
    const query = filterText();
    if (state.activeKind === 'friends') return renderFriendChannels(query);
    if (state.activeKind === 'doctor') return doctor.renderChannels(channelButton);
    if (state.activeKind === 'node') return node.renderChannels(channelButton);
    if (state.activeKind === 'voice') return window.ElonVoiceProject.renderChannels(voiceContext());
    return renderProjectChannels(query);
  }

  function renderFriendChannels(query) {
    const friends = state.friends.filter((f) => userName(f).toLowerCase().includes(query));
    const groups = state.groups.filter((g) => clean(g.name || g.title || g.id).toLowerCase().includes(query));
    els.channelList.innerHTML = [
      '<div class="channel-section">好友</div>',
      friends.map((friend) => channelButton({
        id: friend.id, kind: 'friend', avatar: avatarUrlOf(friend), avatarFallback: userName(friend), title: userName(friend),
        sub: friend.is_online ? '在线' : '离线', online: !!friend.is_online,
        active: state.activePeer && state.activePeer.kind === 'friend' && state.activePeer.id === friend.id
      })).join('') || '<div class="empty-state">暂无好友</div>',
      '<div class="channel-section">群聊</div>',
      groups.map((group) => channelButton({
        id: group.id, kind: 'group', avatar: avatarUrlOf(group), avatarFallback: clean(group.name || group.title || '群聊'), glyph: '群', title: clean(group.name || group.title || '未命名群聊'),
        sub: `${Number(group.member_count || group.members_count || 0)} 位成员`,
        active: state.activePeer && state.activePeer.kind === 'group' && state.activePeer.id === group.id
      })).join('') || '<div class="empty-state">暂无群聊</div>'
    ].join('');
    els.channelList.querySelectorAll('[data-peer-kind]').forEach((btn) => {
      btn.addEventListener('click', () => selectPeer(btn.dataset.peerKind, btn.dataset.itemId));
    });
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
      : (item.kind === 'doctor-section'
        ? `data-doctor-section="${escapeHtml(item.id)}"`
        : `data-peer-kind="${escapeHtml(item.kind)}" data-item-id="${escapeHtml(item.id)}"`);
    const glyph = item.avatar || item.avatarFallback
      ? avatarElement('span', 'glyph channel-avatar', item.avatar, item.avatarFallback || item.title || item.glyph || '#', item.glyph || '#')
      : `<span class="glyph">${escapeHtml(item.glyph || '#')}</span>`;
    return `<button class="channel-item ${item.active ? 'active' : ''}" type="button" ${attrs}>
      ${glyph}
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

  function clearSurfaceModes() {
    els.messageList.classList.remove('node-mode', 'doctor-mode');
  }

  function setNodeMode(enabled) {
    clearSurfaceModes();
    if (enabled) els.messageList.classList.add('node-mode');
  }

  function setDoctorMode(enabled) {
    clearSurfaceModes();
    if (enabled) els.messageList.classList.add('doctor-mode');
  }

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
    els.doctorRail.addEventListener('click', doctor.selectDoctor);
    els.nodeRail.addEventListener('click', node.selectNode);
    els.voiceRail.addEventListener('click', () => selectVoiceProject());
    [els.friendsRail, els.doctorRail, els.nodeRail, els.voiceRail, $('openWebBtn')].forEach(attachRailTooltip);
    $('refreshBtn').addEventListener('click', refreshActive);
    $('openWebBtn').addEventListener('click', () => window.open('/web', '_blank'));
    $('openLegacyWebBtn').addEventListener('click', () => window.open('/web', '_blank'));
    $('openLocalNodeBtn').addEventListener('click', node.openNodeWindow);
    els.userSettingsBtn.addEventListener('click', openSettings);
    els.settingsCloseBtn.addEventListener('click', closeSettings);
    els.settingsBackdrop.addEventListener('click', (event) => {
      if (event.target === els.settingsBackdrop) closeSettings();
    });
    els.chooseProjectFolderBtn.addEventListener('click', chooseLocalProjectFolder);
    els.inspectProjectFolderBtn.addEventListener('click', inspectLocalProjectFolder);
    els.registerProjectBtn.addEventListener('click', registerLocalProject);
    $('logoutBtn').addEventListener('click', logout);
    els.sidebarSearch.addEventListener('input', renderChannels);
    els.composer.addEventListener('submit', (event) => {
      event.preventDefault();
      sendCurrentMessage(false);
    });
    els.aiTaskBtn.addEventListener('click', () => sendCurrentMessage(true));
    els.input.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter' || event.shiftKey || event.isComposing || event.keyCode === 229) return;
      if (els.sendBtn.disabled) return;
      event.preventDefault();
      sendCurrentMessage(false);
    });
    els.input.addEventListener('input', () => {
      els.input.style.height = '46px';
      els.input.style.height = Math.min(120, els.input.scrollHeight) + 'px';
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && !els.settingsBackdrop.hidden) closeSettings();
    });
  }

  function attachRailTooltip(button) {
    if (!button || button.dataset.tooltipBound) return;
    button.dataset.tooltipBound = '1';
    button.addEventListener('mouseenter', showRailTooltip);
    button.addEventListener('focus', showRailTooltip);
    button.addEventListener('mouseleave', hideRailTooltip);
    button.addEventListener('blur', hideRailTooltip);
  }

  function showRailTooltip(event) {
    if (!els.railTooltip) return;
    const button = event.currentTarget;
    const label = clean(button.dataset.label || button.getAttribute('aria-label'));
    if (!label) return;
    const rect = button.getBoundingClientRect();
    els.railTooltip.textContent = label;
    els.railTooltip.style.left = `${Math.round(rect.right + 12)}px`;
    els.railTooltip.style.top = `${Math.round(rect.top + rect.height / 2)}px`;
    els.railTooltip.classList.add('show');
  }

  function hideRailTooltip() {
    if (!els.railTooltip) return;
    els.railTooltip.classList.remove('show');
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
      <strong>登录后一处使用好友、电脑医生、项目和 PC 节点</strong>
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
    renderMembers('好友在线', state.friends.map((f) => Object.assign({}, f, { name: userName(f), sub: f.is_online ? '在线' : '离线' })));
    if (state.activePeer) selectPeer(state.activePeer.kind, state.activePeer.id);
    else {
      setHeader('友', '好友列表', '选择左侧好友或群聊开始对话');
      setComposer(false, '选择好友或群聊后开始输入', false);
      setNodeMode(false);
      els.messageList.innerHTML = '<div class="empty-state"><strong>好友和群聊</strong><p>左侧第一枚图标固定打开好友列表；电脑医生和 PC 节点是独立入口，项目图标排在固定入口下方。</p></div>';
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

  function voiceContext() {
    return {
      state, els, api, setHeader, setComposer, setNodeMode, renderMembers,
      escapeHtml, clean, selectVoiceChannel
    };
  }

  function selectVoiceProject(channelId) {
    state.activeKind = 'voice';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.activeVoiceChannel = channelId || state.activeVoiceChannel || 'studio';
    setRails('voice');
    els.workspaceName.textContent = 'ai声音';
    els.workspaceMeta.textContent = '情绪女声 TTS';
    renderChannels();
    return selectVoiceChannel(state.activeVoiceChannel);
  }

  function selectVoiceChannel(channelId) {
    state.activeVoiceChannel = channelId || 'studio';
    renderChannels();
    return window.ElonVoiceProject.renderMain(voiceContext());
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
      renderMembers('项目成员', members.map((m) => Object.assign({}, m, {
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
      return `<div class="member-row">${avatarElement('div', 'member-avatar', avatarUrlOf(member), name, '员')}<div><strong>${escapeHtml(name)}</strong><span>${escapeHtml(sub)}</span></div></div>`;
    }).join('') || '<div class="empty-state">暂无成员</div>';
  }

  function senderIdOf(message) {
    return clean(message.sender_user_id || message.senderUserId || message.sender_id || message.senderId || message.user_id || message.userId);
  }

  function isOwnMessage(message) {
    const senderId = senderIdOf(message);
    return !!message.outgoing || !!(state.user && senderId && sameId(senderId, state.user.id));
  }

  function avatarForMessage(message, scope) {
    const direct = avatarUrlOf(message);
    if (direct) return direct;
    const senderId = senderIdOf(message);
    if (isOwnMessage(message)) return avatarUrlOf(state.user);
    if (scope === 'friend') {
      const friend = state.friends.find((item) => senderId ? sameId(item.id, senderId) : (state.activePeer && sameId(item.id, state.activePeer.id)));
      return avatarUrlOf(friend);
    }
    if (scope === 'group') {
      const group = state.groups.find((item) => state.activePeer && sameId(item.id, state.activePeer.id));
      const member = group && Array.isArray(group.members)
        ? group.members.find((item) => sameId(item.id || item.user_id || item.userId, senderId))
        : null;
      return avatarUrlOf(member);
    }
    if (scope === 'project') {
      const member = ((state.projectSpace && state.projectSpace.members) || [])
        .find((item) => sameId(item.user_id || item.userId || item.id, senderId));
      return avatarUrlOf(member);
    }
    return '';
  }

  function normalizeProjectJoinMode(value) {
    const mode = clean(value).toLowerCase();
    return ['open', 'approval', 'invite', 'readonly'].includes(mode) ? mode : 'open';
  }

  function parseProjectShareMessage(content) {
    const text = clean(content);
    if (!text.startsWith(PROJECT_SHARE_MARKER)) return null;
    const jsonText = text.slice(PROJECT_SHARE_MARKER.length).trim();
    if (!jsonText) return null;
    try {
      const data = JSON.parse(jsonText);
      const id = clean(data.id || data.project_id || data.projectId);
      const name = clean(data.name || data.display_name || data.displayName || data.title);
      if (!id || !name) return null;
      return {
        id,
        name,
        description: clean(data.description || data.project_description || data.projectDescription),
        ownerAccount: clean(data.owner_account || data.ownerAccount || data.created_by_account || data.owner),
        memberCount: Math.max(1, Number(data.member_count || data.memberCount || data.members || 1) || 1),
        joinMode: normalizeProjectJoinMode(data.join_mode || data.joinMode),
        latestLog: clean(data.latest_log || data.latestLog || data.last_task_status || data.status),
        icon: iconUrlOf(data),
        source: clean(data.source || 'store') || 'store'
      };
    } catch (_) {
      return null;
    }
  }

  function projectShareModeLabel(mode) {
    if (mode === 'approval') return '审批加入';
    if (mode === 'invite') return '邀请协作';
    if (mode === 'readonly') return '只读体验';
    return '开放加入';
  }

  function projectShareActionLabel(share, message) {
    if (isOwnMessage(message) || projectById(share.id)) return '打开项目';
    if (share.source === 'local') return '查看项目';
    if (share.joinMode === 'approval') return '申请加入';
    if (share.joinMode === 'invite') return '接受邀请';
    if (share.joinMode === 'readonly') return '进入体验';
    return '加入项目';
  }

  function renderProjectShareCard(share, message) {
    const hue = projectHue({ id: share.id, name: share.name });
    const desc = share.description || share.latestLog || '暂无简介';
    const owner = share.ownerAccount ? `<span>创建者：${escapeHtml(share.ownerAccount)}</span>` : '';
    const icon = share.icon ? `<img src="${escapeHtml(share.icon)}" alt="" onerror="this.remove(); this.parentElement.classList.add('fallback')" />` : '';
    return `<div class="message-content project-share-wrap">
      <div class="project-share-card" style="--project-hue:${hue}">
        <div class="project-share-banner">
          <span class="project-share-icon ${share.icon ? '' : 'fallback'}" aria-hidden="true">${icon}<span>${escapeHtml(firstChar(share.name, 'P'))}</span></span>
          <span class="project-share-pill">${escapeHtml(projectShareModeLabel(share.joinMode))}</span>
        </div>
        <div class="project-share-body">
          <strong class="project-share-title">${escapeHtml(share.name)}</strong>
          <div class="project-share-meta"><span>● ${escapeHtml(share.memberCount)} 位成员</span>${owner}</div>
          <div class="project-share-desc">${escapeHtml(desc)}</div>
          <button class="project-share-action" type="button"
            data-project-share-id="${escapeHtml(share.id)}"
            data-project-share-name="${escapeHtml(share.name)}"
            data-project-share-join-mode="${escapeHtml(share.joinMode)}"
            data-project-share-source="${escapeHtml(share.source)}">${escapeHtml(projectShareActionLabel(share, message))}</button>
        </div>
      </div>
    </div>`;
  }

  function renderMessageContent(message, options) {
    const opts = options || {};
    const raw = message.content || message.text || message.message || '';
    const share = parseProjectShareMessage(raw);
    if (share) return renderProjectShareCard(share, message);
    if (markdown.renderMessage && opts.markdown) {
      return markdown.renderMessage(raw, {
        className: opts.className || '',
        copy: !!opts.copy
      });
    }
    const className = clean(opts.className);
    return `<div class="message-content ${escapeHtml(className)}">${escapeHtml(raw)}</div>`;
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
      const contentHtml = renderMessageContent(message, {
        className: tone,
        markdown: !!tone,
        copy: !!tone
      });
      return `<article class="message-row">
        ${avatarElement('div', 'message-avatar', avatarForMessage(message, scope), name, '员')}
        <div class="message-body">
          <div class="message-meta"><strong>${escapeHtml(name)}</strong><span>${escapeHtml(formatTime(message.created_at || message.createdAt))}</span></div>
          ${contentHtml}
        </div>
      </article>`;
    }).join('');
    els.messageList.querySelectorAll('.project-share-action').forEach((button) => {
      button.addEventListener('click', () => handleProjectShareAction(button));
    });
    if (markdown.bindCopyButtons) markdown.bindCopyButtons(els.messageList);
    els.messageList.scrollTop = els.messageList.scrollHeight;
  }

  async function handleProjectShareAction(button) {
    const share = {
      id: clean(button.dataset.projectShareId),
      name: clean(button.dataset.projectShareName),
      joinMode: normalizeProjectJoinMode(button.dataset.projectShareJoinMode),
      source: clean(button.dataset.projectShareSource || 'store') || 'store'
    };
    if (!share.id) return;
    const existing = projectById(share.id);
    if (existing) {
      await selectProject(share.id);
      return;
    }
    if (share.source === 'local') {
      window.alert('这个卡片来自手机本地项目，请在手机端加入，或让对方重新发送协作项目卡片。');
      return;
    }
    const originalText = button.textContent;
    button.disabled = true;
    button.textContent = share.joinMode === 'approval' ? '提交申请中…' : '加入中…';
    try {
      if (share.joinMode === 'approval') {
        const request = await api(`/api/projects/${encodeURIComponent(share.id)}/request-join`, {
          method: 'POST',
          body: JSON.stringify({ message: '' })
        });
        if (request.ok === false) throw new Error(request.message || '申请失败');
        window.alert(request.message || '申请已提交，等待审核');
        return;
      }
      const joined = await api(`/api/projects/${encodeURIComponent(share.id)}/join`, { method: 'POST' });
      if (joined.ok === false) throw new Error(joined.message || '加入失败');
      await loadBaseData();
      const project = projectById(share.id);
      if (project) await selectProject(share.id);
      else window.alert(joined.message || `已加入「${share.name || '项目'}」`);
    } catch (error) {
      window.alert(error.message || '加入失败');
    } finally {
      button.disabled = false;
      button.textContent = originalText;
    }
  }

  async function sendCurrentMessage(useAiTask) {
    const content = clean(els.input.value);
    if (!content) return;
    els.sendBtn.disabled = true;
    try {
      if (state.activeKind === 'doctor') {
        await doctor.sendComposerMessage(content);
        els.input.value = '';
        els.input.style.height = '46px';
      } else if (state.activeKind === 'friends' && state.activePeer) {
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

  function openSettings() {
    els.settingsBackdrop.hidden = false;
    setSettingsResult('');
    setTimeout(() => els.settingsProjectPath.focus(), 0);
  }

  function closeSettings() {
    els.settingsBackdrop.hidden = true;
  }

  function setSettingsResult(message, kind) {
    els.settingsProjectResult.innerHTML = message
      ? `<div class="settings-result ${kind || 'ok'}">${message}</div>`
      : '';
  }

  function setSettingsBusy(button, busy, label) {
    if (!button) return;
    if (busy) {
      button.dataset.label = button.textContent;
      button.disabled = true;
      button.textContent = label || '处理中…';
    } else {
      button.disabled = false;
      button.textContent = button.dataset.label || button.textContent;
    }
  }

  function applyLocalProjectInfo(payload) {
    const project = (payload && payload.project) || {};
    const inspect = (payload && payload.inspect) || {};
    const path = clean(project.workspace_path || inspect.workspace_path);
    const name = clean(project.name);
    const repo = clean(project.repo_url || inspect.git_remote_origin);
    const branch = clean(project.branch || inspect.git_branch);
    const desc = clean(project.description);
    if (path) els.settingsProjectPath.value = path;
    if (name) els.settingsProjectName.value = name;
    els.settingsProjectRepo.value = repo;
    els.settingsProjectBranch.value = branch;
    if (desc && !clean(els.settingsProjectDesc.value)) els.settingsProjectDesc.value = desc;
    const git = inspect.is_git_worktree || project.is_git_worktree
      ? [branch || 'HEAD', clean(project.git_head || inspect.git_head), (project.has_uncommitted_changes || inspect.has_uncommitted_changes) ? '有未提交改动' : '干净']
        .filter(Boolean).join(' · ')
      : '未检测到 Git 工作区';
    els.settingsProjectMeta.textContent = path ? `${path} · ${git}` : '尚未选择项目目录';
  }

  async function chooseLocalProjectFolder() {
    setSettingsResult('正在打开本机文件夹选择器…');
    setSettingsBusy(els.chooseProjectFolderBtn, true, '选择中…');
    try {
      const data = await localNodeApi('/api/project-folder/pick', { method: 'POST' });
      if (data.cancelled) {
        setSettingsResult('已取消选择。');
        return;
      }
      applyLocalProjectInfo(data);
      setSettingsResult('已读取项目目录、Git 远端和当前分支。');
    } catch (error) {
      setSettingsResult(escapeHtml(error.message || error), 'error');
    } finally {
      setSettingsBusy(els.chooseProjectFolderBtn, false);
    }
  }

  async function inspectLocalProjectFolder() {
    const path = clean(els.settingsProjectPath.value);
    if (!path) {
      setSettingsResult('请先选择或填写项目目录。', 'error');
      return;
    }
    setSettingsBusy(els.inspectProjectFolderBtn, true, '读取中…');
    try {
      const data = await localNodeApi('/api/project-folder/inspect', {
        method: 'POST',
        body: JSON.stringify({ workspace_path: path })
      });
      applyLocalProjectInfo(data);
      setSettingsResult('已读取项目目录、Git 远端和当前分支。');
    } catch (error) {
      setSettingsResult(escapeHtml(error.message || error), 'error');
    } finally {
      setSettingsBusy(els.inspectProjectFolderBtn, false);
    }
  }

  async function ensureLocalNodeLogin() {
    if (!state.token) throw new Error('请先登录一龙账号');
    const status = await localNodeApi('/api/status');
    if (status.logged_in && status.user_token_configured) return status;
    return localNodeApi('/api/login', {
      method: 'POST',
      body: JSON.stringify({ token: state.token })
    });
  }

  async function registerLocalProject() {
    const name = clean(els.settingsProjectName.value);
    const path = clean(els.settingsProjectPath.value);
    if (!name || !path) {
      setSettingsResult('请选择项目目录，确认项目名称已自动填写。', 'error');
      return;
    }
    setSettingsBusy(els.registerProjectBtn, true, '注册中…');
    try {
      await ensureLocalNodeLogin();
      const data = await localNodeApi('/api/register-project', {
        method: 'POST',
        body: JSON.stringify({
          name,
          workspace_path: path,
          description: clean(els.settingsProjectDesc.value) || null,
          repo_url: clean(els.settingsProjectRepo.value) || null,
          branch: clean(els.settingsProjectBranch.value) || null
        })
      });
      const project = (data.cloud && data.cloud.project) || {};
      const reused = data.cloud && data.cloud.reused_existing;
      setSettingsResult(`${reused ? '已复用现有项目' : '注册成功'}：${escapeHtml(project.name || name)}${project.id ? ` · ${escapeHtml(project.id)}` : ''}`);
      await loadBaseData();
      if (project.id) {
        closeSettings();
        await selectProject(project.id);
      } else {
        await refreshActive();
      }
    } catch (error) {
      setSettingsResult(escapeHtml(error.message || error), 'error');
    } finally {
      setSettingsBusy(els.registerProjectBtn, false);
    }
  }

  async function refreshActive() {
    if (!state.token) return showLoginState();
    await loadBaseData();
    if (state.activeKind === 'doctor') return doctor.selectDoctor();
    if (state.activeKind === 'node') return node.selectNode();
    if (state.activeKind === 'voice') return selectVoiceProject(state.activeVoiceChannel);
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
