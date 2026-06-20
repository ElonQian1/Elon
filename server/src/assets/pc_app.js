(function () {
  const kit = window.ElonPcKit || {};
  const { readToken, safeNodeAdminUrl, escapeHtml, clean, firstChar, formatTime } = kit;
  const markdown = window.ElonPcMarkdown || {};
  const TOKEN_KEYS = kit.TOKEN_KEYS || ['lodex_token', 'elon_token'];
  const SOCIAL_AI_USER_ID = 'usr_elon_ai';
  const $ = (id) => document.getElementById(id);
  const state = {
    token: readToken(), user: null, projects: [], friends: [], groups: [], nodes: [],
    activeKind: 'ai', activeProjectId: '', activeChannelId: '', activeChannelKind: '',
    activeVoiceChannel: 'studio',
    activePeer: null, projectSpace: null,
    plaza: { loaded: false, loading: false, projects: [], query: '', filterKey: 'all', busyId: '', error: '' },
    nodeAdminUrl: safeNodeAdminUrl()
  };
  const PROJECT_SHARE_MARKER = '【一龙项目卡片】';
  const PLAZA_FILTERS = [
    { key: 'all', label: '全部' },
    { key: 'installable', label: '可安装', hasApk: true },
    { key: 'no_approval', label: '无审批', noApprovalOnly: true },
    { key: 'joined', label: '已加入', joinedOnly: true },
    { key: 'popular', label: '最热门', sort: 'members' }
  ];
  let pcAuthMode = 'login';

  const els = {
    pcShell: $('pcShell'), authClaimBanner: $('authClaimBanner'), authClaimBtn: $('authClaimBtn'),
    pcAuthBackdrop: $('pcAuthBackdrop'), pcAuthForm: $('pcAuthForm'), pcAuthCloseBtn: $('pcAuthCloseBtn'),
    pcAuthAccountInput: $('pcAuthAccountInput'), pcAuthNicknameField: $('pcAuthNicknameField'),
    pcAuthNicknameInput: $('pcAuthNicknameInput'), pcAuthPasswordInput: $('pcAuthPasswordInput'),
    pcAuthError: $('pcAuthError'), pcAuthSubmitBtn: $('pcAuthSubmitBtn'),
    aiRail: $('aiRail'), friendsRail: $('friendsRail'), projectsRail: $('projectsRail'), projectPlazaRail: $('projectPlazaRail'),
    doctorRail: $('doctorRail'), nodeRail: $('nodeRail'), voiceRail: $('voiceRail'), apkRail: $('openWebBtn'), projectRailList: $('projectRailList'),
    channelList: $('channelList'), memberList: $('memberList'), messageList: $('messageList'),
    workspaceName: $('workspaceName'), workspaceMeta: $('workspaceMeta'), channelGlyph: $('channelGlyph'),
    channelTitle: $('channelTitle'), channelSubtitle: $('channelSubtitle'), userName: $('userName'),
    userMeta: $('userMeta'), userDot: $('userDot'), aiBadge: $('aiBadge'), friendBadge: $('friendBadge'), nodeBadge: $('nodeBadge'),
    sidebarSearch: $('sidebarSearch'), composer: $('composer'), input: $('messageInput'),
    sendBtn: $('sendBtn'), aiTaskBtn: $('aiTaskBtn'), memberPanelTitle: $('memberPanelTitle'),
    railTooltip: $('railTooltip'), userProfileBtn: $('userProfileBtn'), userSettingsBtn: $('userSettingsBtn'),
    accountMenu: $('accountMenu'), accountMenuAvatar: $('accountMenuAvatar'), accountMenuName: $('accountMenuName'),
    accountMenuMeta: $('accountMenuMeta'), profileCenterBtn: $('profileCenterBtn'), pcSettingsMenuBtn: $('pcSettingsMenuBtn'),
    logoutMenuBtn: $('logoutMenuBtn'), settingsBackdrop: $('settingsBackdrop'),
    settingsCloseBtn: $('settingsCloseBtn'), settingsAccountTab: $('settingsAccountTab'), settingsWorkbenchTab: $('settingsWorkbenchTab'),
    settingsNotificationsTab: $('settingsNotificationsTab'), settingsAccountPanel: $('settingsAccountPanel'),
    settingsWorkbenchPanel: $('settingsWorkbenchPanel'), settingsNotificationsPanel: $('settingsNotificationsPanel'),
    settingsPlaceholderPanel: $('settingsPlaceholderPanel'), settingsPlaceholderTitle: $('settingsPlaceholderTitle'),
    settingsPlaceholderText: $('settingsPlaceholderText'), settingsSubtitle: $('settingsSubtitle'),
    settingsUserAvatar: $('settingsUserAvatar'), settingsUserName: $('settingsUserName'), settingsUserMeta: $('settingsUserMeta'),
    settingsDisplayName: $('settingsDisplayName'), settingsAccountValue: $('settingsAccountValue'), settingsUserId: $('settingsUserId'),
    settingsVerifyBtn: $('settingsVerifyBtn'), settingsEditProfileBtn: $('settingsEditProfileBtn'), settingsLoginBtn: $('settingsLoginBtn'),
    settingsSecurityBtn: $('settingsSecurityBtn'), settingsDevicesBtn: $('settingsDevicesBtn'), settingsLogoutBtn: $('settingsLogoutBtn'),
    chooseProjectFolderBtn: $('chooseProjectFolderBtn'),
    inspectProjectFolderBtn: $('inspectProjectFolderBtn'), registerProjectBtn: $('registerProjectBtn'),
    settingsProjectPath: $('settingsProjectPath'), settingsProjectName: $('settingsProjectName'),
    settingsProjectDesc: $('settingsProjectDesc'), settingsProjectRepo: $('settingsProjectRepo'),
    settingsProjectBranch: $('settingsProjectBranch'), settingsProjectMeta: $('settingsProjectMeta'),
    settingsProjectResult: $('settingsProjectResult'), settingsRuntimePermission: $('settingsRuntimePermission'),
    settingsRuntimePermissionHint: $('settingsRuntimePermissionHint')
  };

  let models = null;
  const doctor = window.ElonPcDoctor.create({
    state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer,
    setRails, renderChannels, setDoctorMode
  });
  const node = window.ElonPcNode.create({
    state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer,
    setRails, renderChannels, setNodeMode, localNodeApi, ensureLocalNodeLogin,
    openSettings, loadBaseData
  });
  const projectReadiness = window.ElonPcProjectReadiness.create({
    state, $, clean, escapeHtml, api, openSettings,
    selectNode: () => node.selectNode(),
    selectProject,
    selectProjectChannel
  });
  const projectLanding = window.ElonPcProjectLanding.create({
    state, els, clean, escapeHtml, firstChar, formatTime, titleOf, iconUrlOf,
    channelName, channelGlyph, selectProjectChannel, setHeader, setComposer, setNodeMode,
    api, localNodeApi, ensureLocalNodeLogin, loadBaseData, selectProject
  });
  models = window.ElonPcModels.create({ state, els, clean, escapeHtml, api });

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

  function socialAiFriend() {
    return state.friends.find((friend) => sameId(friend && friend.id, SOCIAL_AI_USER_ID)) || null;
  }

  function socialFriends() {
    return state.friends.filter((friend) => !sameId(friend && friend.id, SOCIAL_AI_USER_ID));
  }

  function userAccountMeta(user) {
    const account = clean(user && (user.account || user.phone || user.email));
    const id = clean(user && user.id);
    if (account && account !== userName(user)) return `账号：${account}`;
    if (id) return `用户 ID：${id}`;
    return state.token ? '在线' : '需要登录';
  }

  function userAccountValue(user) {
    return clean(user && (user.account || user.phone || user.email)) || (state.token ? '账号信息未完善' : '请先登录');
  }

  function setBadge(el, value) {
    if (!el) return;
    const n = Number(value || 0);
    el.textContent = n > 99 ? '99+' : String(n);
    el.classList.toggle('show', n > 0);
  }

  function setAuthClaimBanner(visible) {
    if (!els.authClaimBanner || !els.pcShell) return;
    els.authClaimBanner.hidden = !visible;
    els.pcShell.classList.toggle('has-auth-banner', !!visible);
  }

  function pcAuthTabs() {
    return Array.from(document.querySelectorAll('[data-pc-auth-mode]'));
  }

  function updatePcAuthSubmitLabel(busy) {
    if (!els.pcAuthSubmitBtn) return;
    if (busy) {
      els.pcAuthSubmitBtn.textContent = pcAuthMode === 'register' ? '创建中…' : '登录中…';
      return;
    }
    els.pcAuthSubmitBtn.textContent = pcAuthMode === 'register' ? '创建账号' : '登录';
  }

  function setPcAuthMode(mode) {
    pcAuthMode = mode === 'register' ? 'register' : 'login';
    const isRegister = pcAuthMode === 'register';
    pcAuthTabs().forEach((button) => {
      button.classList.toggle('active', button.dataset.pcAuthMode === pcAuthMode);
    });
    if (els.pcAuthNicknameField) els.pcAuthNicknameField.hidden = !isRegister;
    if (els.pcAuthPasswordInput) els.pcAuthPasswordInput.autocomplete = isRegister ? 'new-password' : 'current-password';
    setPcAuthError('');
    updatePcAuthSubmitLabel(false);
  }

  function setPcAuthError(message) {
    if (!els.pcAuthError) return;
    els.pcAuthError.textContent = message || '';
    els.pcAuthError.classList.toggle('show', !!message);
  }

  function setPcAuthBusy(busy) {
    if (!els.pcAuthSubmitBtn) return;
    els.pcAuthSubmitBtn.disabled = !!busy;
    updatePcAuthSubmitLabel(!!busy);
  }

  function openAuthModal(mode) {
    setAccountMenu(false);
    if (els.settingsBackdrop && !els.settingsBackdrop.hidden) closeSettings();
    setPcAuthMode(mode);
    if (els.pcAuthBackdrop) els.pcAuthBackdrop.hidden = false;
    setTimeout(() => {
      const target = pcAuthMode === 'register' && els.pcAuthNicknameInput && clean(els.pcAuthAccountInput.value)
        ? els.pcAuthNicknameInput
        : els.pcAuthAccountInput;
      if (target) target.focus();
    }, 0);
  }

  function closeAuthModal() {
    if (els.pcAuthBackdrop) els.pcAuthBackdrop.hidden = true;
    setPcAuthError('');
  }

  function keepAuthModalOpenOnOutsideClick(event) {
    if (!els.pcAuthBackdrop || els.pcAuthBackdrop.hidden) return;
    if (event.target.closest('.pc-auth-dialog')) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }

  function authFetchWithTimeout(url, options, ms) {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), ms || 15000);
    return fetch(url, Object.assign({}, options || {}, { signal: ctrl.signal }))
      .finally(() => clearTimeout(timer));
  }

  async function submitPcAuth(event) {
    event.preventDefault();
    setPcAuthBusy(true);
    setPcAuthError('');
    try {
      const account = clean(els.pcAuthAccountInput.value);
      const password = els.pcAuthPasswordInput.value || '';
      if (!account) throw new Error('请输入账号');
      if (!password) throw new Error('请输入密码');
      if (pcAuthMode === 'register' && password.length < 6) throw new Error('密码至少 6 位');
      const payload = { account, password, device_name: 'pc-web' };
      if (pcAuthMode === 'register') payload.nickname = clean(els.pcAuthNicknameInput.value);
      const res = await authFetchWithTimeout(pcAuthMode === 'register' ? '/api/auth/register' : '/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      }, 15000);
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error || (pcAuthMode === 'register' ? '注册失败' : '登录失败'));
      if (!data.token) throw new Error('登录态返回异常，请重试');
      saveToken(data.token);
      state.user = data.user || null;
      await refreshActive();
      closeAuthModal();
    } catch (error) {
      const message = error && error.name === 'AbortError' ? '请求超时，请检查网络后重试' : (error && error.message) || '网络错误';
      setPcAuthError(message);
    } finally {
      setPcAuthBusy(false);
    }
  }

  function setRails(kind) {
    els.aiRail.classList.toggle('active', kind === 'ai' || kind === 'store' || kind === 'tasks');
    els.friendsRail.classList.toggle('active', kind === 'friends');
    els.projectsRail.classList.toggle('active', kind === 'projects');
    els.projectPlazaRail.classList.toggle('active', kind === 'project-plaza');
    els.doctorRail.classList.toggle('active', kind === 'doctor');
    els.nodeRail.classList.toggle('active', kind === 'node');
    els.voiceRail.classList.toggle('active', kind === 'voice');
    els.apkRail.classList.toggle('active', kind === 'apk');
    Array.from(els.projectRailList.children).forEach((btn) => {
      btn.classList.toggle('active', kind === 'project' && btn.dataset.projectId === state.activeProjectId);
    });
  }

  function renderUser() {
    const name = userName(state.user);
    const avatar = avatarUrlOf(state.user);
    const meta = userAccountMeta(state.user);
    els.userName.textContent = name;
    els.userMeta.textContent = state.token ? '在线' : '需要登录';
    els.userDot.classList.toggle('fallback', !avatar);
    els.userDot.innerHTML = avatarContents(avatar, name, '龙');
    if (els.accountMenuName) els.accountMenuName.textContent = name;
    if (els.accountMenuMeta) els.accountMenuMeta.textContent = meta;
    if (els.accountMenuAvatar) {
      els.accountMenuAvatar.classList.toggle('fallback', !avatar);
      els.accountMenuAvatar.innerHTML = avatarContents(avatar, name, '龙');
    }
    if (els.settingsUserName) els.settingsUserName.textContent = name;
    if (els.settingsUserMeta) els.settingsUserMeta.textContent = meta;
    if (els.settingsDisplayName) els.settingsDisplayName.textContent = name;
    if (els.settingsAccountValue) els.settingsAccountValue.textContent = userAccountValue(state.user);
    if (els.settingsUserId) els.settingsUserId.textContent = clean(state.user && state.user.id) || '--';
    if (els.settingsUserAvatar) {
      els.settingsUserAvatar.classList.toggle('fallback', !avatar);
      els.settingsUserAvatar.innerHTML = avatarContents(avatar, name, '龙');
    }
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

  function setSidebarPlaceholder(text) {
    if (els.sidebarSearch) els.sidebarSearch.placeholder = text;
  }

  function renderChannels() {
    const query = filterText();
    if (state.activeKind === 'ai' || state.activeKind === 'store' || state.activeKind === 'tasks') return renderAiSidebar(query);
    if (state.activeKind === 'friends') return renderFriendChannels(query);
    if (state.activeKind === 'projects') return renderProjectHomeChannels(query);
    if (state.activeKind === 'project-plaza') return renderProjectPlazaChannels(query);
    if (state.activeKind === 'apk') return renderApkChannels();
    if (state.activeKind === 'doctor') return doctor.renderChannels(channelButton);
    if (state.activeKind === 'node') return node.renderChannels(channelButton);
    if (state.activeKind === 'voice') return window.ElonVoiceProject.renderChannels(voiceContext());
    return renderProjectChannels(query);
  }

  function aiSidebarButton(item) {
    return `<button class="channel-item ${item.primary ? 'ai-primary' : ''} ${item.active ? 'active' : ''}" type="button" data-ai-action="${escapeHtml(item.id)}">
      <span class="glyph">${escapeHtml(item.glyph || '#')}</span>
      <span class="main"><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.sub || '')}</span></span>
    </button>`;
  }

  function renderAiSidebar(query) {
    const aiFriend = socialAiFriend();
    const actionMatches = (item) => !query || `${item.title} ${item.sub || ''}`.toLowerCase().includes(query);
    const primaryActions = [
      { id: 'new-chat', glyph: '+', title: '新对话', sub: aiFriend ? '直接问一龙AI' : '登录后开启', primary: true, active: state.activeKind === 'ai' },
      { id: 'search', glyph: '搜', title: '搜索', sub: '搜索对话、项目和工具' },
      { id: 'store', glyph: '插', title: '插件', sub: '项目和 APK 商店', active: state.activeKind === 'store' },
      { id: 'tasks', glyph: '自', title: '自动化', sub: '任务和提醒', active: state.activeKind === 'tasks' },
      { id: 'mobile', glyph: '手', title: '一龙移动版', sub: 'APK 和移动网页版', active: state.activeKind === 'apk' }
    ].filter(actionMatches);
    const workspaceActions = [
      { id: 'projects', glyph: '项', title: '项目', sub: `${state.projects.length} 个项目`, active: state.activeKind === 'projects' },
      { id: 'plaza', glyph: '广', title: '项目广场', sub: '浏览公开项目', active: state.activeKind === 'project-plaza' },
      { id: 'doctor', glyph: '医', title: '电脑医生', sub: '检查和修复本机环境' },
      { id: 'node', glyph: '节', title: 'PC 节点', sub: `${state.nodes.filter((n) => n.online).length} 台在线` }
    ].filter(actionMatches);
    const projects = state.projects
      .filter((project) => !query || titleOf(project).toLowerCase().includes(query))
      .slice(0, 8);
    const projectItems = projects.map((project) => {
      const title = titleOf(project);
      const sub = project.updated_at || project.updatedAt ? `更新 ${formatTime(project.updated_at || project.updatedAt)}` : projectRoleLabel(project);
      return `<button class="channel-item" type="button" data-ai-project-id="${escapeHtml(project.id)}">
        ${avatarElement('span', 'glyph channel-avatar', iconUrlOf(project), title, firstChar(title, '项'))}
        <span class="main"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(sub)}</span></span>
      </button>`;
    }).join('');
    const sections = [
      '<div class="channel-section">一龙AI</div>',
      primaryActions.map(aiSidebarButton).join('') || '<div class="ai-sidebar-muted">没有匹配的功能</div>',
      '<div class="ai-sidebar-spacer"></div>',
      '<div class="channel-section">工作台</div>',
      workspaceActions.map(aiSidebarButton).join(''),
      '<div class="channel-section">项目</div>',
      projectItems || '<div class="ai-sidebar-muted">暂无匹配项目</div>'
    ];
    els.channelList.innerHTML = sections.join('');
    els.channelList.querySelectorAll('[data-ai-action]').forEach((btn) => {
      btn.addEventListener('click', () => {
        const action = btn.dataset.aiAction;
        if (action === 'new-chat') return selectAiAssistant(true);
        if (action === 'search') {
          els.sidebarSearch.focus();
          els.sidebarSearch.select();
          return;
        }
        if (action === 'store') return selectStore();
        if (action === 'tasks') return selectTasks();
        if (action === 'mobile') return selectApkDownload();
        if (action === 'projects') return selectProjectsHome();
        if (action === 'plaza') return selectProjectPlaza();
        if (action === 'doctor') return doctor.selectDoctor();
        if (action === 'node') return node.selectNode();
      });
    });
    els.channelList.querySelectorAll('[data-ai-project-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.aiProjectId));
    });
  }

  function renderFriendChannels(query) {
    const friends = socialFriends().filter((f) => userName(f).toLowerCase().includes(query));
    const groups = state.groups.filter((g) => clean(g.name || g.title || g.id).toLowerCase().includes(query));
    const sections = [
      '<div class="channel-section">好友</div>',
      friends.map((friend) => channelButton({
      id: friend.id, kind: 'friend', avatar: avatarUrlOf(friend), avatarFallback: userName(friend), title: userName(friend),
      sub: friend.is_online ? '在线' : '离线', online: !!friend.is_online,
      active: state.activePeer && state.activePeer.kind === 'friend' && sameId(state.activePeer.id, friend.id)
      })).join('') || '<div class="empty-state">暂无好友</div>',
      '<div class="channel-section">群聊</div>',
      groups.map((group) => channelButton({
      id: group.id, kind: 'group', avatar: avatarUrlOf(group), avatarFallback: clean(group.name || group.title || '群聊'), glyph: '群', title: clean(group.name || group.title || '未命名群聊'),
      sub: `${Number(group.member_count || group.members_count || 0)} 位成员`,
      active: state.activePeer && state.activePeer.kind === 'group' && sameId(state.activePeer.id, group.id)
      })).join('') || '<div class="empty-state">暂无群聊</div>'
    ];
    els.channelList.innerHTML = sections.join('');
    els.channelList.querySelectorAll('[data-peer-kind]').forEach((btn) => {
      btn.addEventListener('click', () => selectPeer(btn.dataset.peerKind, btn.dataset.itemId));
    });
  }

  function projectDescription(project) {
    return clean(project.project_description || project.projectDescription || project.description || project.subtitle) || '暂无简介';
  }

  function projectRoleLabel(project) {
    const role = clean(project.role || project.member_role || project.memberRole);
    if (!role || role === 'owner') return '拥有者';
    if (role === 'admin') return '管理员';
    if (role === 'editor') return '协作者';
    return role;
  }

  function projectMemberCount(project) {
    const count = Number(project.member_count || project.memberCount || project.members || 0);
    return Number.isFinite(count) && count > 0 ? count : 1;
  }

  function renderProjectHomeChannels(query) {
    const projects = state.projects.filter((project) => titleOf(project).toLowerCase().includes(query));
    els.channelList.innerHTML = [
      '<div class="channel-section">项目</div>',
      '<button class="channel-item" type="button" data-project-home-action="overview"><span class="glyph">项</span><span class="main"><strong>我的项目</strong><span>查看项目列表</span></span></button>',
      '<button class="channel-item" type="button" data-project-home-action="plaza"><span class="glyph">广</span><span class="main"><strong>项目广场</strong><span>发现公开项目</span></span></button>',
      '<div class="channel-section">项目列表</div>',
      state.token
        ? (projects.map((project) => channelButton({
          id: project.id,
          kind: 'project-entry',
          avatar: iconUrlOf(project),
          avatarFallback: titleOf(project),
          glyph: '项',
          title: titleOf(project),
          sub: `${projectRoleLabel(project)} · ${projectMemberCount(project)} 位成员`,
          active: state.activeProjectId && sameId(project.id, state.activeProjectId)
        })).join('') || '<div class="empty-state">暂无项目</div>')
        : '<div class="empty-state">登录后显示我的项目</div>'
    ].join('');
    els.channelList.querySelectorAll('[data-project-home-action]').forEach((btn) => {
      btn.addEventListener('click', () => {
        if (btn.dataset.projectHomeAction === 'plaza') selectProjectPlaza();
        else renderProjectHomeSurface();
      });
    });
    els.channelList.querySelectorAll('[data-peer-kind="project-entry"]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.itemId));
    });
  }

  function renderProjectPlazaChannels(query) {
    const joinedIds = new Set(state.projects.map((project) => project && project.id).filter(Boolean));
    const joined = state.plaza.projects
      .filter((project) => joinedIds.has(project.id) && titleOf(project).toLowerCase().includes(query))
      .slice(0, 20);
    els.channelList.innerHTML = [
      '<div class="channel-section">项目广场</div>',
      '<button class="channel-item active" type="button" data-plaza-channel="all"><span class="glyph">广</span><span class="main"><strong>全部公开项目</strong><span>搜索、加入和下载 APK</span></span></button>',
      '<button class="channel-item" type="button" data-plaza-channel="mine"><span class="glyph">已</span><span class="main"><strong>已加入</strong><span>我已加入的公开项目</span></span></button>',
      '<div class="channel-section">已加入</div>',
      state.token
        ? (joined.map((project) => channelButton({
          id: project.id,
          kind: 'project-entry',
          avatar: iconUrlOf(project),
          avatarFallback: titleOf(project),
          glyph: '项',
          title: titleOf(project),
          sub: `${projectMemberCount(project)} 位成员`,
          active: false
        })).join('') || '<div class="empty-state">暂无已加入项目</div>')
        : '<div class="empty-state">登录后查看已加入项目</div>'
    ].join('');
    els.channelList.querySelectorAll('[data-plaza-channel]').forEach((btn) => {
      btn.addEventListener('click', () => {
        state.plaza.filterKey = btn.dataset.plazaChannel === 'mine' ? 'joined' : 'all';
        loadProjectPlaza(true);
      });
    });
    els.channelList.querySelectorAll('[data-peer-kind="project-entry"]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.itemId));
    });
  }

  function renderProjectChannels(query) {
    const channels = ((state.projectSpace && state.projectSpace.channels) || [])
      .filter((channel) => channelName(channel).toLowerCase().includes(query));
    const homeVisible = !query || '首页开始介绍下载overviewhome'.includes(query);
    els.channelList.innerHTML = [
      '<div class="channel-section">频道</div>',
      homeVisible ? channelButton({
        id: 'project-home',
        kind: 'project-home',
        glyph: '首',
        title: '首页',
        sub: '项目介绍与下载',
        active: !state.activeChannelId
      }) : '',
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
    els.channelList.querySelectorAll('[data-project-home]').forEach((btn) => {
      btn.addEventListener('click', selectProjectLanding);
    });
  }

  function channelButton(item) {
    const attrs = item.kind === 'project-channel'
      ? `data-channel-id="${escapeHtml(item.id)}"`
      : (item.kind === 'project-home'
        ? 'data-project-home="1"'
      : (item.kind === 'doctor-section'
        ? `data-doctor-section="${escapeHtml(item.id)}"`
        : `data-peer-kind="${escapeHtml(item.kind)}" data-item-id="${escapeHtml(item.id)}"`));
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
    const canSelectModel = !!state.token && !!state.user;
    els.aiTaskBtn.disabled = !canSelectModel;
    els.aiTaskBtn.classList.toggle('enabled', canSelectModel);
    els.aiTaskBtn.classList.toggle('task-ready', !!aiEnabled);
    els.input.placeholder = placeholder || '输入消息';
    if (models) models.updateButton();
  }

  function clearSurfaceModes() {
    els.messageList.classList.remove('node-mode', 'doctor-mode', 'project-landing-mode');
  }

  function setNodeMode(enabled) {
    clearSurfaceModes();
    if (enabled) els.messageList.classList.add('node-mode');
  }

  function setDoctorMode(enabled) {
    clearSurfaceModes();
    if (enabled) els.messageList.classList.add('doctor-mode');
  }

  function setAccountMenu(open) {
    if (!els.accountMenu) return;
    const visible = !!open;
    els.accountMenu.hidden = !visible;
    if (els.userProfileBtn) els.userProfileBtn.setAttribute('aria-expanded', visible ? 'true' : 'false');
  }

  function toggleAccountMenu() {
    setAccountMenu(els.accountMenu && els.accountMenu.hidden);
  }

  function openProfileCenter() {
    setAccountMenu(false);
    window.open('/web?tab=profile', '_blank');
  }

  async function init() {
    bindEvents();
    if (!state.token) {
      showLoginState();
      return;
    }
    await loadBaseData();
    models.loadModelOptions(false);
    selectAiAssistant();
  }

  function bindEvents() {
    els.aiRail.addEventListener('click', () => selectAiAssistant(true));
    els.friendsRail.addEventListener('click', selectFriends);
    els.projectsRail.addEventListener('click', selectProjectsHome);
    els.projectPlazaRail.addEventListener('click', selectProjectPlaza);
    els.doctorRail.addEventListener('click', doctor.selectDoctor);
    els.nodeRail.addEventListener('click', node.selectNode);
    els.voiceRail.addEventListener('click', () => selectVoiceProject());
    els.authClaimBtn.addEventListener('click', () => openAuthModal('register'));
    pcAuthTabs().forEach((button) => button.addEventListener('click', () => setPcAuthMode(button.dataset.pcAuthMode)));
    els.pcAuthForm.addEventListener('submit', submitPcAuth);
    els.pcAuthCloseBtn.addEventListener('click', closeAuthModal);
    document.addEventListener('pointerdown', keepAuthModalOpenOnOutsideClick, true);
    document.addEventListener('click', keepAuthModalOpenOnOutsideClick, true);
    [els.aiRail, els.friendsRail, els.projectsRail, els.projectPlazaRail, els.doctorRail, els.nodeRail, els.voiceRail, els.apkRail].forEach(attachRailTooltip);
    $('refreshBtn').addEventListener('click', refreshActive);
    els.apkRail.addEventListener('click', selectApkDownload);
    $('openLegacyWebBtn').addEventListener('click', () => window.open('/web', '_blank'));
    $('openLocalNodeBtn').addEventListener('click', node.selectNode);
    els.userProfileBtn.addEventListener('click', toggleAccountMenu);
    els.userSettingsBtn.addEventListener('click', () => openSettings('account'));
    els.profileCenterBtn.addEventListener('click', () => openSettings('account'));
    els.pcSettingsMenuBtn.addEventListener('click', () => {
      setAccountMenu(false);
      openSettings('workbench');
    });
    els.logoutMenuBtn.addEventListener('click', logout);
    document.querySelectorAll('[data-settings-section]').forEach((button) => {
      button.addEventListener('click', () => setSettingsSection(button.dataset.settingsSection));
    });
    els.settingsVerifyBtn.addEventListener('click', () => {
      if (state.token) openProfileCenter();
      else openAuthModal('register');
    });
    els.settingsEditProfileBtn.addEventListener('click', openProfileCenter);
    els.settingsLoginBtn.addEventListener('click', () => openAuthModal('login'));
    els.settingsSecurityBtn.addEventListener('click', () => setSettingsSection('security'));
    els.settingsDevicesBtn.addEventListener('click', () => setSettingsSection('devices'));
    els.settingsLogoutBtn.addEventListener('click', logout);
    els.settingsCloseBtn.addEventListener('click', closeSettings);
    els.settingsBackdrop.addEventListener('click', (event) => {
      if (event.target === els.settingsBackdrop) closeSettings();
    });
    els.chooseProjectFolderBtn.addEventListener('click', chooseLocalProjectFolder);
    els.inspectProjectFolderBtn.addEventListener('click', inspectLocalProjectFolder);
    els.registerProjectBtn.addEventListener('click', registerLocalProject);
    if (els.settingsRuntimePermission) {
      els.settingsRuntimePermission.addEventListener('change', syncSettingsRuntimePermissionHint);
      syncSettingsRuntimePermissionHint();
    }
    $('logoutBtn').addEventListener('click', logout);
    els.sidebarSearch.addEventListener('input', renderChannels);
    els.composer.addEventListener('submit', (event) => {
      event.preventDefault();
      sendCurrentMessage(false);
    });
    els.aiTaskBtn.addEventListener('click', () => models.openModelPicker());
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
      if (event.key === 'Escape' && els.pcAuthBackdrop && !els.pcAuthBackdrop.hidden) {
        closeAuthModal();
        return;
      }
      if (event.key === 'Escape' && els.accountMenu && !els.accountMenu.hidden) {
        setAccountMenu(false);
        return;
      }
      if (event.key === 'Escape' && !els.settingsBackdrop.hidden) closeSettings();
    });
    document.addEventListener('click', (event) => {
      if (!els.accountMenu || els.accountMenu.hidden) return;
      if (event.target.closest('#accountMenu') || event.target.closest('.user-strip')) return;
      setAccountMenu(false);
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
    setAuthClaimBanner(false);
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
    setBadge(els.aiBadge, socialAiFriend() && socialAiFriend().unread_count);
    setBadge(els.friendBadge, socialFriends().filter((f) => f.is_online).length);
    setBadge(els.nodeBadge, state.nodes.filter((n) => n.online).length);
  }

  function valueOf(result) {
    if (result.status === 'fulfilled') return result.value || {};
    return {};
  }

  function showLoginState() {
    setAccountMenu(false);
    setAuthClaimBanner(true);
    hideRailTooltip();
    state.user = null;
    state.projects = [];
    state.friends = [];
    state.groups = [];
    state.nodes = [];
    state.activeKind = 'ai';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeChannelKind = '';
    state.activePeer = null;
    state.projectSpace = null;
    renderUser();
    renderProjectRail();
    setBadge(els.aiBadge, 0);
    setBadge(els.friendBadge, 0);
    setBadge(els.nodeBadge, 0);
    setRails('ai');
    els.workspaceName.textContent = '一龙AI';
    els.workspaceMeta.textContent = '未登录';
    setSidebarPlaceholder('搜索对话、项目和工具');
    setHeader('AI', '需要登录', '先登录网页版，再回到 PC 工作台');
    setComposer(false, '登录后可输入消息', false);
    renderAiSidebar(filterText());
    els.memberList.innerHTML = '';
    setNodeMode(false);
    els.messageList.innerHTML = `<div class="empty-state">
      <strong>登录后使用一龙AI、项目和 PC 工作台</strong>
      <p>PC 工作台读取账号登录态。点击下方按钮登录或注册后，即可进入一龙AI工作区。</p>
      <button class="text-button" type="button" id="loginWeb">登录或注册账号</button>
    </div>`;
    $('loginWeb').addEventListener('click', () => openAuthModal('login'));
  }

  function selectProjectsHome() {
    state.activeKind = 'projects';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.projectSpace = null;
    setAuthClaimBanner(!state.token);
    setRails('projects');
    els.workspaceName.textContent = '项目 / 我的项目';
    els.workspaceMeta.textContent = state.token ? `${state.projects.length} 个项目` : '需要登录';
    setSidebarPlaceholder('搜索项目');
    setHeader('项', '项目 / 我的项目', '查看你加入和创建的项目');
    setComposer(false, '选择项目后开始输入', false);
    setNodeMode(false);
    renderProjectHomeChannels(filterText());
    renderMembers('项目成员', []);
    renderProjectHomeSurface();
  }

  function renderProjectHomeSurface() {
    setHeader('项', '项目 / 我的项目', '查看你加入和创建的项目');
    setComposer(false, '选择项目后开始输入', false);
    setNodeMode(false);
    const projects = state.projects.slice().sort((a, b) => String(b.updated_at || b.updatedAt || '').localeCompare(String(a.updated_at || a.updatedAt || '')));
    if (!state.token) {
      els.messageList.innerHTML = `<div class="empty-state">
        <strong>登录后查看我的项目</strong>
        <p>项目列表、协作空间和本机项目注册都需要登录账号。</p>
        <button class="text-button" type="button" id="projectLoginBtn">登录或注册账号</button>
      </div>`;
      $('projectLoginBtn').addEventListener('click', () => openAuthModal('login'));
      return;
    }
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>我的项目</h2>
          <p>集中管理个人项目、联合项目和已加入的协作空间。</p>
        </div>
        <button class="text-button" type="button" id="projectOpenWebBtn">打开网页版项目页</button>
      </div>
      ${projects.length ? `<div class="pc-project-grid">${projects.map(renderProjectCard).join('')}</div>` : '<div class="pc-project-empty">还没有项目<br>可以从项目广场加入，或在 PC 工作台设置里注册本地项目。</div>'}
    </section>`;
    const webBtn = $('projectOpenWebBtn');
    if (webBtn) webBtn.addEventListener('click', () => window.open('/web', '_blank'));
    els.messageList.querySelectorAll('[data-open-project-id]').forEach((button) => {
      button.addEventListener('click', () => selectProject(button.dataset.openProjectId));
    });
  }

  function renderProjectCard(project) {
    const title = titleOf(project);
    const icon = iconUrlOf(project);
    const hue = projectHue(project);
    const iconMarkup = icon ? `<img src="${escapeHtml(icon)}" alt="" onerror="this.remove()" />` : escapeHtml(firstChar(title, '项'));
    return `<button class="pc-project-card" type="button" data-open-project-id="${escapeHtml(project.id)}" style="--project-hue:${hue}">
      <span class="pc-project-icon">${iconMarkup}</span>
      <span>
        <strong>${escapeHtml(title)}</strong>
        <p>${escapeHtml(projectDescription(project))}</p>
        <span class="pc-project-meta">
          <span>${escapeHtml(projectRoleLabel(project))}</span>
          <span>${escapeHtml(projectMemberCount(project))} 位成员</span>
        </span>
      </span>
    </button>`;
  }

  function selectProjectPlaza() {
    state.activeKind = 'project-plaza';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.projectSpace = null;
    setAuthClaimBanner(!state.token);
    setRails('project-plaza');
    els.workspaceName.textContent = '项目广场';
    els.workspaceMeta.textContent = state.plaza.loading ? '加载中' : `${state.plaza.projects.length} 个公开项目`;
    setSidebarPlaceholder('搜索项目广场');
    setHeader('广', '项目广场', '发现、加入和下载公开项目');
    setComposer(false, '加入项目后可输入消息', false);
    setNodeMode(false);
    renderMembers('项目广场', []);
    renderProjectPlazaChannels(filterText());
    renderProjectPlazaSurface();
    if (!state.plaza.loaded && !state.plaza.loading) loadProjectPlaza(false);
  }

  function plazaFilter() {
    return PLAZA_FILTERS.find((item) => item.key === state.plaza.filterKey) || PLAZA_FILTERS[0];
  }

  async function loadProjectPlaza(force) {
    if (state.plaza.loading) return;
    if (!force && state.plaza.loaded) {
      renderProjectPlazaSurface();
      return;
    }
    state.plaza.loading = true;
    state.plaza.error = '';
    renderProjectPlazaSurface();
    const filter = plazaFilter();
    const params = new URLSearchParams({ limit: '80', offset: '0' });
    if (state.plaza.query) params.set('q', state.plaza.query);
    if (filter.hasApk != null) params.set('has_apk', String(filter.hasApk));
    if (filter.sort) params.set('sort', filter.sort);
    try {
      const data = await api('/api/store/projects?' + params.toString(), { cache: 'no-store' });
      const projects = Array.isArray(data.projects) ? data.projects : [];
      const joinedIds = new Set(state.projects.map((project) => project && project.id).filter(Boolean));
      state.plaza.projects = projects.filter((project) => {
        const joined = joinedIds.has(project.id);
        return (!filter.joinedOnly || joined) &&
          (!filter.noApprovalOnly || clean(project.join_mode || project.joinMode).toLowerCase() !== 'approval');
      });
      state.plaza.loaded = true;
      els.workspaceMeta.textContent = `${state.plaza.projects.length} 个公开项目`;
    } catch (error) {
      state.plaza.projects = [];
      state.plaza.error = error.message || '加载失败';
    } finally {
      state.plaza.loading = false;
      renderProjectPlazaChannels(filterText());
      renderProjectPlazaSurface();
    }
  }

  function renderProjectPlazaSurface() {
    setHeader('广', '项目广场', '发现、加入和下载公开项目');
    setComposer(false, '加入项目后可输入消息', false);
    setNodeMode(false);
    const filterButtons = PLAZA_FILTERS.map((filter) => `<button class="pc-plaza-filter ${filter.key === state.plaza.filterKey ? 'active' : ''}" type="button" data-plaza-filter="${escapeHtml(filter.key)}">${escapeHtml(filter.label)}</button>`).join('');
    const body = state.plaza.loading
      ? '<div class="pc-project-empty">项目广场加载中...</div>'
      : (state.plaza.error
        ? `<div class="pc-project-empty">加载失败<br>${escapeHtml(state.plaza.error)}</div>`
        : (state.plaza.projects.length
          ? `<div class="pc-project-grid">${state.plaza.projects.map(renderPlazaCard).join('')}</div>`
          : '<div class="pc-project-empty">暂无匹配项目</div>'));
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>项目广场</h2>
          <p>浏览公开项目，加入协作空间或下载可安装 APK。</p>
        </div>
        <button class="text-button" type="button" id="projectPlazaRefreshBtn">刷新</button>
      </div>
      <div class="pc-plaza-toolbar">
        <input class="pc-plaza-search" id="projectPlazaSearchInput" type="search" placeholder="搜索公开项目" value="${escapeHtml(state.plaza.query)}" />
        <div class="pc-plaza-filter-row">${filterButtons}</div>
      </div>
      ${body}
    </section>`;
    const refreshBtn = $('projectPlazaRefreshBtn');
    if (refreshBtn) refreshBtn.addEventListener('click', () => loadProjectPlaza(true));
    const search = $('projectPlazaSearchInput');
    if (search) search.addEventListener('keydown', (event) => {
      if (event.key !== 'Enter') return;
      state.plaza.query = clean(search.value);
      loadProjectPlaza(true);
    });
    els.messageList.querySelectorAll('[data-plaza-filter]').forEach((button) => {
      button.addEventListener('click', () => {
        state.plaza.filterKey = button.dataset.plazaFilter || 'all';
        loadProjectPlaza(true);
      });
    });
    els.messageList.querySelectorAll('[data-plaza-action]').forEach((button) => {
      button.addEventListener('click', () => handlePlazaAction(button));
    });
  }

  function renderPlazaCard(project) {
    const title = titleOf(project);
    const icon = iconUrlOf(project);
    const hue = projectHue(project);
    const joined = state.projects.some((item) => item && sameId(item.id, project.id));
    const mode = clean(project.join_mode || project.joinMode || 'open').toLowerCase();
    const apkUrl = clean(project.latest_apk_url || project.last_apk_url);
    const busy = state.plaza.busyId && sameId(state.plaza.busyId, project.id);
    const primaryAction = joined ? 'open' : (mode === 'approval' ? 'apply' : 'join');
    const primaryLabel = busy ? '处理中...' : (joined ? '进入空间' : (mode === 'approval' ? '申请加入' : '加入项目'));
    const iconMarkup = icon ? `<img src="${escapeHtml(icon)}" alt="" onerror="this.remove()" />` : escapeHtml(firstChar(title, '项'));
    return `<article class="pc-plaza-card" style="--project-hue:${hue}">
      <div class="pc-plaza-card-head">
        <span class="pc-project-icon">${iconMarkup}</span>
        <span>
          <strong>${escapeHtml(title)}</strong>
          <p>${escapeHtml(projectDescription(project))}</p>
        </span>
      </div>
      <div class="pc-project-meta">
        <span class="pc-plaza-pill">${escapeHtml(mode === 'approval' ? '需审批' : '无需审批')}</span>
        <span class="pc-plaza-pill">${escapeHtml(apkUrl ? '可安装' : '暂无 APK')}</span>
        <span class="pc-plaza-pill">${escapeHtml(projectMemberCount(project))} 位成员</span>
      </div>
      <div class="pc-plaza-actions">
        <button class="primary" type="button" data-plaza-action="${escapeHtml(primaryAction)}" data-project-id="${escapeHtml(project.id)}" ${busy ? 'disabled' : ''}>${escapeHtml(primaryLabel)}</button>
        <button type="button" data-plaza-action="download" data-project-id="${escapeHtml(project.id)}" ${apkUrl ? '' : 'disabled'}>下载 APK</button>
      </div>
    </article>`;
  }

  async function handlePlazaAction(button) {
    const id = clean(button.dataset.projectId);
    const action = button.dataset.plazaAction;
    if (!id) return;
    const project = state.plaza.projects.find((item) => sameId(item.id, id));
    if (action === 'download') {
      const apkUrl = clean(project && (project.latest_apk_url || project.last_apk_url));
      if (apkUrl) window.open(apkUrl, '_blank', 'noopener');
      return;
    }
    if (action === 'open') {
      const localProject = projectById(id);
      if (localProject) await selectProject(id);
      else window.alert('项目已加入，但当前列表还未同步，请刷新后重试。');
      return;
    }
    if (!state.token) {
      openAuthModal('login');
      return;
    }
    state.plaza.busyId = id;
    renderProjectPlazaSurface();
    try {
      if (action === 'apply') {
        const request = await api(`/api/projects/${encodeURIComponent(id)}/request-join`, {
          method: 'POST',
          body: JSON.stringify({ message: '' })
        });
        window.alert(request.message || '申请已提交，等待审核');
      } else if (action === 'join') {
        const joined = await api(`/api/projects/${encodeURIComponent(id)}/join`, { method: 'POST' });
        if (joined.ok === false) throw new Error(joined.message || '加入失败');
        await loadBaseData();
        await selectProject(id);
      }
    } catch (error) {
      window.alert(error.message || '操作失败');
    } finally {
      state.plaza.busyId = '';
      if (state.activeKind === 'project-plaza') {
        renderProjectPlazaChannels(filterText());
        renderProjectPlazaSurface();
      }
    }
  }

  function selectStore() {
    state.activeKind = 'store';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.projectSpace = null;
    setAuthClaimBanner(!state.token);
    setRails('store');
    els.workspaceName.textContent = '一龙AI';
    els.workspaceMeta.textContent = '插件和商店';
    setSidebarPlaceholder('搜索功能、项目和工具');
    renderChannels();
    renderMembers('商店', []);
    renderStoreSurface();
  }

  function renderStoreSurface() {
    setHeader('商', '商店', '项目、APK 和移动端入口');
    setComposer(false, '选择项目或下载入口后开始', false);
    setNodeMode(false);
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>商店</h2>
          <p>集中进入项目广场、可安装项目和手机端下载。</p>
        </div>
        <button class="text-button" type="button" data-store-action="refresh-plaza">刷新项目广场</button>
      </div>
      <div class="pc-feature-grid">
        <button class="pc-feature-card" type="button" data-store-action="plaza">
          <span class="pc-feature-glyph">广</span>
          <span>
            <strong>项目广场</strong>
            <p>浏览公开项目，加入协作空间。</p>
          </span>
        </button>
        <button class="pc-feature-card" type="button" data-store-action="installable">
          <span class="pc-feature-glyph">APK</span>
          <span>
            <strong>可安装项目</strong>
            <p>筛选带 APK 的公开项目。</p>
          </span>
        </button>
        <button class="pc-feature-card" type="button" data-store-action="apk">
          <span class="pc-feature-glyph">下</span>
          <span>
            <strong>APK 下载 / 手机端入口</strong>
            <p>下载手机端，或打开移动网页版。</p>
          </span>
        </button>
      </div>
    </section>`;
    els.messageList.querySelectorAll('[data-store-action]').forEach((button) => {
      button.addEventListener('click', () => {
        const action = button.dataset.storeAction;
        if (action === 'apk') return selectApkDownload();
        if (action === 'installable') state.plaza.filterKey = 'installable';
        else state.plaza.filterKey = 'all';
        state.plaza.loaded = false;
        selectProjectPlaza();
      });
    });
  }

  function selectTasks() {
    state.activeKind = 'tasks';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.projectSpace = null;
    setAuthClaimBanner(!state.token);
    setRails('tasks');
    els.workspaceName.textContent = '一龙AI';
    els.workspaceMeta.textContent = '自动化';
    setSidebarPlaceholder('搜索自动化和项目');
    renderChannels();
    renderMembers('任务', []);
    renderTasksSurface();
  }

  function renderTasksSurface() {
    setHeader('任', '任务', '任务和提醒');
    setComposer(false, '选择项目频道后开始输入', false);
    setNodeMode(false);
    const loginAction = state.token
      ? ''
      : '<button class="text-button" type="button" id="taskLoginBtn">登录或注册账号</button>';
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>任务</h2>
          <p>项目协作、审核和待处理提醒会在这里汇总。</p>
        </div>
        <button class="text-button" type="button" id="taskProjectsBtn">打开我的项目</button>
      </div>
      <div class="pc-task-panel">
        <div class="pc-task-item">
          <span class="pc-task-dot"></span>
          <span>
            <strong>${state.token ? '暂无待处理任务' : '登录后查看任务'}</strong>
            <p>${state.token ? '当前没有项目审核、协作邀请或本机处理任务。' : '登录或注册后，任务中心会读取你的项目和账号提醒。'}</p>
          </span>
          ${loginAction}
        </div>
      </div>
    </section>`;
    $('taskProjectsBtn').addEventListener('click', selectProjectsHome);
    const taskLoginBtn = $('taskLoginBtn');
    if (taskLoginBtn) taskLoginBtn.addEventListener('click', () => openAuthModal('login'));
  }

  function renderApkChannels() {
    els.channelList.innerHTML = [
      '<div class="channel-section">手机端</div>',
      '<button class="channel-item active" type="button" data-apk-channel="download"><span class="glyph">下</span><span class="main"><strong>APK 下载</strong><span>安装手机端</span></span></button>',
      '<button class="channel-item" type="button" data-apk-channel="web"><span class="glyph">网</span><span class="main"><strong>移动网页版</strong><span>在浏览器打开</span></span></button>'
    ].join('');
    els.channelList.querySelectorAll('[data-apk-channel]').forEach((button) => {
      button.addEventListener('click', () => {
        if (button.dataset.apkChannel === 'web') window.open('/web', '_blank', 'noopener');
        else renderApkDownloadSurface();
      });
    });
  }

  function selectApkDownload() {
    state.activeKind = 'apk';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activePeer = null;
    state.projectSpace = null;
    setAuthClaimBanner(!state.token);
    setRails('apk');
    els.workspaceName.textContent = '手机端';
    els.workspaceMeta.textContent = 'APK 下载 / 手机端入口';
    setSidebarPlaceholder('搜索手机端入口');
    renderApkChannels();
    renderMembers('手机端', []);
    renderApkDownloadSurface();
  }

  function renderApkDownloadSurface() {
    setHeader('下', 'APK 下载 / 手机端入口', '安装手机端或打开移动网页版');
    setComposer(false, '登录后可输入消息', false);
    setNodeMode(false);
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>APK 下载 / 手机端入口</h2>
          <p>手机端用于安装 APK；移动网页版用于快速登录和同步账号状态。</p>
        </div>
        <button class="text-button" type="button" id="apkOpenDownloadBtn">打开下载页</button>
      </div>
      <div class="pc-apk-panel">
        <div class="pc-apk-device">
          <span class="pc-apk-screen"></span>
        </div>
        <div class="pc-apk-copy">
          <strong>一龙手机端</strong>
          <p>下载 APK 后可在手机上使用项目、账号和工作台能力；网页版和 APK 数据互通。</p>
          <div class="pc-apk-actions">
            <button class="primary" type="button" id="apkDownloadBtn">下载 APK</button>
            <button type="button" id="apkWebBtn">打开移动网页版</button>
            ${state.token ? '' : '<button type="button" id="apkLoginBtn">登录或注册</button>'}
          </div>
        </div>
      </div>
    </section>`;
    $('apkOpenDownloadBtn').addEventListener('click', () => window.open('/download', '_blank', 'noopener'));
    $('apkDownloadBtn').addEventListener('click', () => window.open('/download', '_blank', 'noopener'));
    $('apkWebBtn').addEventListener('click', () => window.open('/web', '_blank', 'noopener'));
    const apkLoginBtn = $('apkLoginBtn');
    if (apkLoginBtn) apkLoginBtn.addEventListener('click', () => openAuthModal('login'));
  }

  async function selectAiAssistant(focusComposer) {
    state.activeKind = 'ai';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeChannelKind = '';
    state.projectSpace = null;
    const aiFriend = socialAiFriend();
    state.activePeer = aiFriend ? { kind: 'friend', id: aiFriend.id } : null;
    setAuthClaimBanner(!state.token);
    setRails('ai');
    els.workspaceName.textContent = '一龙AI';
    els.workspaceMeta.textContent = 'AI 助手和工作台';
    setSidebarPlaceholder('搜索对话、项目和工具');
    renderChannels();
    renderMembers('一龙AI', aiFriend ? [Object.assign({}, aiFriend, { name: userName(aiFriend), sub: aiFriend.is_online ? '在线' : '离线' })] : []);
    setHeader('AI', '一龙AI', aiFriend && aiFriend.is_online ? '在线助手' : 'AI 助手');
    setComposer(!!aiFriend, aiFriend ? '发送给一龙AI' : '登录后可输入消息', false);
    setNodeMode(false);
    if (!aiFriend) {
      els.messageList.innerHTML = `<div class="empty-state">
        <strong>登录后使用一龙AI</strong>
        <p>一龙AI 是独立的工作台入口，登录后可直接提问、打开项目和进入工具。</p>
        <button class="text-button" type="button" id="aiLoginBtn">登录或注册账号</button>
      </div>`;
      const aiLoginBtn = $('aiLoginBtn');
      if (aiLoginBtn) aiLoginBtn.addEventListener('click', () => openAuthModal('login'));
      return;
    }
    els.messageList.innerHTML = '<div class="empty-state">加载一龙AI消息中…</div>';
    try {
      const data = await api(`/api/me/friends/${encodeURIComponent(aiFriend.id)}/messages?limit=100`);
      renderMessages(data.messages || [], 'friend');
      setBadge(els.aiBadge, 0);
      if (focusComposer) setTimeout(() => els.input.focus(), 0);
    } catch (error) {
      showError(error);
    }
  }

  function selectFriends() {
    state.activeKind = 'friends';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeChannelKind = '';
    if (!state.activePeer || (state.activePeer.kind === 'friend' && sameId(state.activePeer.id, SOCIAL_AI_USER_ID))) {
      state.activePeer = null;
    }
    setRails('friends');
    els.workspaceName.textContent = '好友';
    els.workspaceMeta.textContent = `${socialFriends().length} 位好友 · ${state.groups.length} 个群聊`;
    setSidebarPlaceholder('搜索好友或群聊');
    renderChannels();
    renderMembers('好友在线', socialFriends().map((f) => Object.assign({}, f, { name: userName(f), sub: f.is_online ? '在线' : '离线' })));
    if (state.activePeer) selectPeer(state.activePeer.kind, state.activePeer.id);
    else {
      setHeader('友', '好友列表', '选择左侧好友或群聊开始对话');
      setComposer(false, '选择好友或群聊后开始输入', false);
      setNodeMode(false);
      els.messageList.innerHTML = '<div class="empty-state"><strong>好友和群聊</strong><p>这里只显示普通好友和群聊。一龙AI 已经移动到左侧最上方的独立入口。</p></div>';
    }
  }

  async function selectPeer(kind, id) {
    if (kind === 'friend' && sameId(id, SOCIAL_AI_USER_ID)) return selectAiAssistant(true);
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
    setSidebarPlaceholder('搜索声音工具');
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
    setSidebarPlaceholder('搜索项目频道');
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
      })), {
        prefixHtml: projectReadiness.renderMemberPanel(projectById(projectId) || project)
      });
      projectReadiness.bindMemberPanel(projectById(projectId) || project);
      renderChannels();
      selectProjectLanding();
    } catch (error) {
      showError(error);
    }
  }

  function selectProjectLanding() {
    state.activeChannelId = '';
    state.activeChannelKind = 'home';
    renderChannels();
    projectLanding.render();
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

  function renderMembers(title, members, options) {
    els.memberPanelTitle.textContent = title;
    const prefix = options && options.prefixHtml ? options.prefixHtml : '';
    const rows = (members || []).map((member) => {
      const name = clean(member.name || member.nickname || member.account || member.user_account || member.phone || member.email) || '成员';
      const sub = clean(member.sub || member.role || member.status || member.id) || '';
      return `<div class="member-row">${avatarElement('div', 'member-avatar', avatarUrlOf(member), name, '员')}<div><strong>${escapeHtml(name)}</strong><span>${escapeHtml(sub)}</span></div></div>`;
    }).join('') || '<div class="empty-state">暂无成员</div>';
    els.memberList.innerHTML = prefix + rows;
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
      } else if ((state.activeKind === 'friends' || state.activeKind === 'ai') && state.activePeer) {
        const path = state.activePeer.kind === 'group'
          ? `/api/me/groups/${encodeURIComponent(state.activePeer.id)}/messages`
          : `/api/me/friends/${encodeURIComponent(state.activePeer.id)}/messages`;
        await api(path, { method: 'POST', body: JSON.stringify({ content }) });
        els.input.value = '';
        if (state.activeKind === 'ai') await selectAiAssistant(true);
        else await selectPeer(state.activePeer.kind, state.activePeer.id);
      } else if (state.activeKind === 'project' && state.activeProjectId && state.activeChannelId) {
        const shouldUseAiTask = useAiTask || state.activeChannelKind === 'ai_development';
        const path = shouldUseAiTask
          ? `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/ai-tasks`
          : `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/messages`;
        const body = { content };
        if (shouldUseAiTask) {
          const agent = models.selectedAgentForRequest();
          if (agent) body.agent = agent;
        }
        await api(path, { method: 'POST', body: JSON.stringify(body) });
        els.input.value = '';
        await selectProjectChannel(state.activeChannelId);
      }
    } catch (error) {
      showError(error);
    } finally {
      els.sendBtn.disabled = false;
    }
  }

  function setSettingsSection(section) {
    const selected = ['account', 'workbench', 'notifications'].includes(section) ? section : 'placeholder';
    const placeholderTitles = {
      security: ['密码和安全中心', '密码、多重认证和登录设备会在这里集中管理。'],
      devices: ['已登录的设备', '这里会显示当前账号登录过的 PC 网页版和移动端设备。']
    };
    document.querySelectorAll('[data-settings-section]').forEach((button) => {
      const active = button.dataset.settingsSection === section || (selected === 'account' && section === 'account' && button.dataset.settingsSection === 'account');
      button.classList.toggle('active', active);
      if (button.hasAttribute('aria-selected')) button.setAttribute('aria-selected', active ? 'true' : 'false');
    });
    [els.settingsAccountPanel, els.settingsWorkbenchPanel, els.settingsNotificationsPanel, els.settingsPlaceholderPanel].forEach((panel) => {
      if (panel) panel.classList.remove('active');
    });
    if (selected === 'account') {
      els.settingsAccountPanel.classList.add('active');
      $('settingsTitle').textContent = '账户';
      els.settingsSubtitle.textContent = '账号信息、登录状态和安全设置';
    } else if (selected === 'workbench') {
      els.settingsWorkbenchPanel.classList.add('active');
      $('settingsTitle').textContent = 'PC 工作台设置';
      els.settingsSubtitle.textContent = '本地项目注册和节点绑定';
    } else if (selected === 'notifications') {
      els.settingsNotificationsPanel.classList.add('active');
      $('settingsTitle').textContent = '通知';
      els.settingsSubtitle.textContent = '项目、节点和聊天提醒';
    } else {
      const copy = placeholderTitles[section] || ['设置', '这个分类会随着功能完善继续补充。'];
      els.settingsPlaceholderPanel.classList.add('active');
      els.settingsPlaceholderTitle.textContent = copy[0];
      els.settingsPlaceholderText.textContent = copy[1];
      $('settingsTitle').textContent = copy[0];
      els.settingsSubtitle.textContent = copy[1];
    }
  }

  function openSettings(section) {
    setAccountMenu(false);
    renderUser();
    setSettingsSection(section || 'workbench');
    els.settingsBackdrop.hidden = false;
    setSettingsResult('');
    setTimeout(() => {
      if ((section || 'workbench') === 'workbench') els.settingsProjectPath.focus();
    }, 0);
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

  function normalizeRuntimePermission(value) {
    return clean(value) === 'full_access' ? 'full_access' : 'project_write';
  }

  function syncSettingsRuntimePermissionHint() {
    if (!els.settingsRuntimePermissionHint || !els.settingsRuntimePermission) return;
    const mode = normalizeRuntimePermission(els.settingsRuntimePermission.value);
    els.settingsRuntimePermissionHint.textContent = mode === 'full_access'
      ? '完全访问会让 AI CLI 按用户授权绕过项目沙箱，可能读取或修改项目目录外的本机文件。'
      : 'AI 只能读写当前项目目录，并运行开发相关命令。';
  }

  async function saveProjectRuntimePermission(project, requestedMode) {
    const projectId = clean(project && project.id);
    if (!projectId) return;
    const mode = normalizeRuntimePermission(requestedMode);
    const data = await api(`/api/projects/${encodeURIComponent(projectId)}/runtime-permission`, {
      method: 'PATCH',
      body: JSON.stringify({ mode, confirmFullAccess: mode === 'full_access' })
    });
    project.runtime_permission = data.mode || mode;
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
    const runtimeMode = normalizeRuntimePermission(els.settingsRuntimePermission && els.settingsRuntimePermission.value);
    if (runtimeMode === 'full_access') {
      const ok = window.confirm(`确认给项目「${name}」开启完全访问？AI CLI 可能读取或修改项目目录外的本机文件和系统设置。`);
      if (!ok) {
        setSettingsResult('已取消完全访问授权，项目尚未注册。');
        return;
      }
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
      await saveProjectRuntimePermission(project, runtimeMode);
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
    models.loadModelOptions(false);
    if (state.activeKind === 'ai') return selectAiAssistant();
    if (state.activeKind === 'store') return selectStore();
    if (state.activeKind === 'tasks') return selectTasks();
    if (state.activeKind === 'apk') return selectApkDownload();
    if (state.activeKind === 'projects') return selectProjectsHome();
    if (state.activeKind === 'project-plaza') return selectProjectPlaza();
    if (state.activeKind === 'doctor') return doctor.selectDoctor();
    if (state.activeKind === 'node') return node.selectNode();
    if (state.activeKind === 'voice') return selectVoiceProject(state.activeVoiceChannel);
    if (state.activeKind === 'project' && state.activeProjectId) return selectProject(state.activeProjectId);
    return selectAiAssistant();
  }

  function logout() {
    setAccountMenu(false);
    closeSettings();
    TOKEN_KEYS.forEach((key) => localStorage.removeItem(key));
    saveToken('');
    state.user = null;
    state.projects = [];
    state.friends = [];
    state.groups = [];
    state.nodes = [];
    models.reset();
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
