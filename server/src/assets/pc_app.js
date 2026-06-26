(function () {
  const kit = window.ElonPcKit || {};
  const { readToken, safeNodeAdminUrl, escapeHtml, clean, firstChar, formatTime } = kit;
  const markdown = window.ElonPcMarkdown || {};
  const TOKEN_KEYS = kit.TOKEN_KEYS || ['lodex_token', 'elon_token'];
  const SOCIAL_AI_USER_ID = 'usr_elon_ai';
  const LOCAL_ADMIN_HEADER_FALLBACK = 'X-Elon-Local-Admin-Token';
  const APK_DOWNLOAD_URL = '/app/ElonSpeed-latest.apk';
  const APK_DOWNLOAD_PAGE_URL = '/app/download';
  const CLIENT_PROTOCOL_TARGETS = {
    open: 'elon-node://open',
    logs: 'elon-node://logs',
    launcher_logs: 'elon-node://launcher-logs',
    task_journal: 'elon-node://task-journal',
    config_dir: 'elon-node://config',
    install_dir: 'elon-node://install-dir',
    diagnostics_dir: 'elon-node://diagnostics',
    repair: 'elon-node://repair'
  };
  const $ = (id) => document.getElementById(id);
  const state = {
    token: readToken(), user: null, projects: [], friends: [], groups: [], nodes: [],
    activeKind: 'ai', activeProjectId: '', activeChannelId: '', activeChannelKind: '',
    aiConversations: [], activeAiConversationId: '', activeAiConversationTitle: '',
    activeConversationId: '', activeMemberUserId: '',
    activeVoiceChannel: 'studio',
    activePeer: null, projectSpace: null,
    aiProjectConversations: { userId: '', items: {}, errors: {}, loadingIds: {}, expandedIds: {}, drafts: {} },
    projectCenterGroups: { mine: true, plaza: false },
    plaza: { loaded: false, loading: false, projects: [], query: '', filterKey: 'all', busyId: '', error: '' },
    nodeAdminUrl: safeNodeAdminUrl(),
    localAdminToken: '',
    localAdminTokenHeader: LOCAL_ADMIN_HEADER_FALLBACK,
    clientMaintenance: null,
    clientPackageLatest: null,
    localProjectInfo: null,
    localNodeLaunchAttempted: false,
    workbenchRegistrationProjectId: ''
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
  let inlineAuthMode = 'login';
  let activeProjectFolderPickController = null;
  let projectFolderPickHintTimer = 0;

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
    settingsSecurityBtn: $('settingsSecurityBtn'), settingsDevicesBtn: $('settingsDevicesBtn'), settingsLogoutBtn: $('settingsLogoutBtn'),
    settingsClientStatus: $('settingsClientStatus'), settingsClientPaths: $('settingsClientPaths'),
    settingsClientActions: $('settingsClientActions'),
    settingsCliBridgeStatus: $('settingsCliBridgeStatus'),
    settingsNodeStatusCard: $('settingsNodeStatusCard'), settingsNodeStatusTitle: $('settingsNodeStatusTitle'),
    settingsNodeStatusDetail: $('settingsNodeStatusDetail'), settingsStepNode: $('settingsStepNode'),
    settingsStepFolder: $('settingsStepFolder'), settingsStepRegister: $('settingsStepRegister'),
    openNodeSetupFromSettingsBtn: $('openNodeSetupFromSettingsBtn'),
    refreshClientMaintenanceBtn: $('refreshClientMaintenanceBtn'),
    chooseProjectFolderBtn: $('chooseProjectFolderBtn'),
    inspectProjectFolderBtn: $('inspectProjectFolderBtn'), registerProjectBtn: $('registerProjectBtn'),
    settingsProjectPath: $('settingsProjectPath'), settingsProjectName: $('settingsProjectName'),
    settingsProjectDesc: $('settingsProjectDesc'), settingsProjectRepo: $('settingsProjectRepo'),
    settingsProjectBranch: $('settingsProjectBranch'), settingsProjectMeta: $('settingsProjectMeta'),
    settingsProjectResult: $('settingsProjectResult'), settingsRuntimePermission: $('settingsRuntimePermission'),
    settingsRuntimePermissionHint: $('settingsRuntimePermissionHint'),
    pcProjectCreateBackdrop: $('pcProjectCreateBackdrop'), pcProjectCreateForm: $('pcProjectCreateForm'),
    pcProjectCreateCloseBtn: $('pcProjectCreateCloseBtn'), pcProjectCreateCancelBtn: $('pcProjectCreateCancelBtn'),
    pcProjectCreateTitle: $('pcProjectCreateTitle'), pcProjectCreateSubtitle: $('pcProjectCreateSubtitle'),
    pcProjectCreateChatHint: $('pcProjectCreateChatHint'),
    pcProjectNameInput: $('pcProjectNameInput'), pcProjectDescInput: $('pcProjectDescInput'),
    pcProjectTemplateSelect: $('pcProjectTemplateSelect'), pcProjectNodeSelect: $('pcProjectNodeSelect'),
    pcProjectStorageNodeSelect: $('pcProjectStorageNodeSelect'), pcProjectStorageHint: $('pcProjectStorageHint'),
    pcProjectRepoInput: $('pcProjectRepoInput'), pcProjectBranchInput: $('pcProjectBranchInput'),
    pcProjectCreateError: $('pcProjectCreateError'), pcProjectCreateSubmitBtn: $('pcProjectCreateSubmitBtn')
  };

  let models = null;
  let projectCreate = null;
  let devComposer = null;
  let devTasks = null;
  let agentRuns = null;
  let devTaskSnapshots = null;
  let clientMaintenanceActions = null;
  let devTaskRefreshTimer = 0;
  const doctor = window.ElonPcDoctor.create({
    state, els, $, clean, escapeHtml, renderMembers, setHeader, setComposer,
    setRails, renderChannels, setDoctorMode, localNodeApi
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
    api, localNodeApi, ensureLocalNodeLogin, loadBaseData, selectProject, openSettings
  });
  models = window.ElonPcModels.create({ state, els, clean, escapeHtml, api });
  projectCreate = window.ElonPcProjectCreate.create({
    state, els, clean, escapeHtml, api, loadBaseData, selectProject,
    refreshActive, renderProjectRail, sameId
  });
  devComposer = window.ElonPcDevComposer.create({
    state, els, clean, escapeHtml, openSettings,
    selectNode: () => node.selectNode(),
    openModelPicker: () => models.openModelPicker()
  });
  devTasks = window.ElonPcDevTasks.create({
    clean, escapeHtml, markdown,
    refreshActiveChannel: refreshActiveProjectChannel,
    cancelTask: cancelProjectAiTask,
    approveTool: approveProjectTool,
    draftContinuation: draftProjectAiContinuation
  });
  agentRuns = window.ElonPcAgentRuns && window.ElonPcAgentRuns.create({
    state, clean, escapeHtml, localNodeApi, sameId,
    activeProject: () => projectById(state.activeProjectId),
    renderMessages,
    draftContinuation: draftProjectAiContinuation,
    logError: (error) => console.warn('PC agent run log refresh failed', error)
  });
  devTaskSnapshots = window.ElonPcTaskSnapshots && window.ElonPcTaskSnapshots.create({
    state, api, localNodeApi, clean, sameId, devTasks,
    renderMessages,
    refreshActiveChannel: refreshActiveProjectChannel,
    logError: (error) => console.warn('PC task snapshot refresh failed', error)
  });
  clientMaintenanceActions = window.ElonPcClientMaintenance && window.ElonPcClientMaintenance.create({
    clean,
    escapeHtml,
    localNodeApi,
    setResult: setSettingsResult,
    refreshMaintenance: refreshClientMaintenance,
    launchProtocol: launchClientProtocol,
    protocolUrlForTarget: clientProtocolUrlForTarget,
    repairProtocolUrl: CLIENT_PROTOCOL_TARGETS.repair
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
    const needsAdmin = localNodeNeedsAdmin(path);
    if (needsAdmin && !state.localAdminToken) await refreshLocalAdminToken();
    localNodeApplyHeaders(request, needsAdmin);
    let resp = await localNodeFetch(path, request);
    if (needsAdmin && resp.status === 403) {
      state.localAdminToken = '';
      await refreshLocalAdminToken();
      localNodeApplyHeaders(request, needsAdmin);
      resp = await localNodeFetch(path, request);
    }
    const text = await resp.text();
    const data = text ? JSON.parse(text) : {};
    rememberLocalAdmin(data);
    if (!resp.ok || data.ok === false) {
      throw new Error(data.error || data.message || resp.statusText);
    }
    return data;
  }

  async function localNodeFetch(path, request) {
    const fetchRequest = Object.assign({}, request || {});
    const timeoutMs = Number(fetchRequest.timeoutMs || 0);
    delete fetchRequest.timeoutMs;
    let timeoutTimer = 0;
    if (timeoutMs > 0 && !fetchRequest.signal) {
      const controller = new AbortController();
      fetchRequest.signal = controller.signal;
      timeoutTimer = window.setTimeout(() => controller.abort(), timeoutMs);
    }
    try {
      return await fetch(nodeAdminEndpoint(path), fetchRequest);
    } catch (error) {
      if (error && error.name === 'AbortError') {
        const aborted = new Error('已停止等待本机助手返回。');
        aborted.name = 'AbortError';
        throw aborted;
      }
      throw new Error(`无法连接本机助手 ${state.nodeAdminUrl}，请确认一龙 Win 端正在运行并已更新。`);
    } finally {
      if (timeoutTimer) window.clearTimeout(timeoutTimer);
    }
  }

  function localNodeNeedsAdmin(path) {
    return String(path || '').replace(/^\/+/, '') !== 'api/status';
  }

  function localNodeApplyHeaders(request, needsAdmin) {
    const headers = Object.assign({}, request.headers || {});
    if (request.body && !Object.keys(headers).some((key) => key.toLowerCase() === 'content-type')) {
      headers['Content-Type'] = 'application/json';
    }
    if (needsAdmin && state.localAdminToken) {
      headers[state.localAdminTokenHeader || LOCAL_ADMIN_HEADER_FALLBACK] = state.localAdminToken;
    }
    request.headers = headers;
  }

  function rememberLocalAdmin(data) {
    const token = clean(data && data.local_admin_token);
    const header = clean(data && data.local_admin_token_header);
    if (token) state.localAdminToken = token;
    if (header) state.localAdminTokenHeader = header;
  }

  async function refreshLocalAdminToken() {
    const resp = await localNodeFetch('/api/status', { cache: 'no-store' });
    const text = await resp.text();
    const data = text ? JSON.parse(text) : {};
    if (!resp.ok || data.ok === false) {
      throw new Error(data.error || data.message || resp.statusText);
    }
    rememberLocalAdmin(data);
    return data;
  }

  function titleOf(project) {
    return clean(project.display_name || project.displayName || project.alias || project.name || project.title) || '未命名项目';
  }

  function iconUrlOf(project) {
    return clean(project.icon_data_url || project.iconDataUrl ||
      project.project_icon_data_url || project.projectIconDataUrl ||
      project.icon_url || project.iconUrl ||
      project.logo_url || project.logoUrl ||
      project.icon || project.logo || project.avatar);
  }

  function avatarUrlOf(entity) {
    if (!entity) return '';
    return clean(entity.avatar_data_url || entity.avatarDataUrl ||
      entity.sender_avatar_data_url || entity.senderAvatarDataUrl ||
      entity.sender_avatar_url || entity.senderAvatarUrl ||
      entity.user_avatar || entity.userAvatar ||
      entity.user_avatar_url || entity.userAvatarUrl ||
      entity.member_avatar_url || entity.memberAvatarUrl ||
      entity.profile_avatar_url || entity.profileAvatarUrl ||
      entity.avatar_url || entity.avatarUrl ||
      entity.icon_data_url || entity.iconDataUrl ||
      entity.logo_url || entity.logoUrl ||
      entity.photo_url || entity.photoUrl ||
      entity.head_img_url || entity.headImgUrl ||
      entity.portrait_url || entity.portraitUrl ||
      entity.image_url || entity.imageUrl ||
      entity.avatar);
  }

  function entityPresenceClass(entity) {
    if (!entity) return '';
    const status = clean(entity.presence || entity.online_status || entity.onlineStatus || entity.status).toLowerCase();
    if (entity.is_online === true || entity.isOnline === true || status === 'online' || status === 'active') return 'online';
    if (entity.is_online === false || entity.isOnline === false || status === 'offline' || status === 'inactive') return 'offline';
    return '';
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

  function isSystemProject(project) {
    const sourceType = clean(project && (project.source_type || project.sourceType)).toLowerCase();
    const template = clean(project && project.template).toLowerCase();
    if (sourceType === 'agent_balloon' || sourceType === 'chat_memory') return true;
    if (template === 'agent_balloon' || template === 'chat_memory') return true;
    const title = titleOf(project);
    return title === '手机控制' || title === '聊天记忆';
  }

  function normalizedProjectPathPart(value) {
    const parts = clean(value)
      .replace(/\\/g, '/')
      .replace(/\/+$/g, '')
      .split('/')
      .filter(Boolean);
    return clean(parts.pop()).toLowerCase();
  }

  function isDefaultJointProject(project) {
    const title = titleOf(project);
    if (title === '一龙网游加速器' || title === '多冠体育') return true;
    const identifiers = new Set(['bb64a', 'fb2']);
    const fields = [
      project && project.name,
      project && project.display_name,
      project && project.displayName,
      project && project.workspace_path,
      project && project.workspacePath,
      project && project.storage_repo_path,
      project && project.storageRepoPath,
      project && project.storage_worktree_path,
      project && project.storageWorktreePath
    ];
    return fields.some((value) => identifiers.has(normalizedProjectPathPart(value)));
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
    return clean(user && (
      user.name || user.display_name || user.displayName || user.nickname ||
      user.phone || user.email || user.account || user.user_account ||
      user.userAccount || user.id || user.user_id || user.userId
    )) || '未登录';
  }

  function memberIdOf(member) {
    return clean(member && (
      member.user_id || member.userId || member.member_user_id || member.memberUserId ||
      member.id || member.account || member.user_account || member.userAccount
    ));
  }

  function memberNameOf(member) {
    return clean(member && (
      member.name || member.display_name || member.displayName || member.nickname ||
      member.account || member.user_account || member.userAccount ||
      member.phone || member.email || member.id || member.user_id || member.userId
    )) || '成员';
  }

  function memberRoleKey(member) {
    return clean(member && (member.role || member.member_role || member.memberRole)).toLowerCase();
  }

  function memberRoleLabel(member) {
    const role = typeof member === 'string' ? clean(member).toLowerCase() : memberRoleKey(member);
    if (!role) return '';
    if (role === 'owner') return '拥有者';
    if (role === 'admin') return '管理员';
    if (role === 'editor' || role === 'developer' || role === 'maintainer') return '协作者';
    if (role === 'member') return '成员';
    if (role === 'observer' || role === 'viewer') return '只读成员';
    return role;
  }

  function friendById(id) {
    const memberId = clean(id);
    if (!memberId) return null;
    return state.friends.find((friend) => sameId(friend && friend.id, memberId)) || null;
  }

  function memberPresence(member) {
    const explicit = entityPresenceClass(member);
    if (explicit) return explicit;
    const id = memberIdOf(member);
    if (id && sameId(id, currentUserId())) return 'online';
    const friend = friendById(id);
    if (friend && friend.is_online === true) return 'online';
    if (friend && friend.is_online === false) return 'offline';
    return '';
  }

  function shortUserId(value) {
    const raw = clean(value).replace(/^usr[_-]?/i, '');
    const compact = raw.replace(/[^a-z0-9]/gi, '').toUpperCase();
    return compact ? `U${compact.slice(0, 6)}` : '--';
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
    if (id) return `用户 ID：${shortUserId(id)}`;
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

  function resetComposerInputHeight() {
    if (!els.input) return;
    els.input.style.height = '46px';
    els.input.style.overflowY = 'hidden';
  }

  function autosizeComposerInput() {
    if (!els.input) return;
    const minHeight = 46;
    const maxHeight = 120;
    els.input.style.height = `${minHeight}px`;
    const nextHeight = Math.min(maxHeight, Math.max(minHeight, els.input.scrollHeight));
    els.input.style.height = `${nextHeight}px`;
    els.input.style.overflowY = els.input.scrollHeight > maxHeight ? 'auto' : 'hidden';
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

  async function runPcAuth(mode, values) {
    const authMode = mode === 'register' ? 'register' : 'login';
    const account = clean(values.account);
    const password = values.password || '';
    if (!account) throw new Error('请输入账号');
    if (!password) throw new Error('请输入密码');
    if (authMode === 'register' && password.length < 6) throw new Error('密码至少 6 位');
    const payload = { account, password, device_name: 'pc-web' };
    if (authMode === 'register') payload.nickname = clean(values.nickname);
    const res = await authFetchWithTimeout(authMode === 'register' ? '/api/auth/register' : '/api/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    }, 15000);
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || (authMode === 'register' ? '注册失败' : '登录失败'));
    if (!data.token) throw new Error('登录态返回异常，请重试');
    saveToken(data.token);
    state.user = data.user || null;
    await refreshActive();
    closeAuthModal();
  }

  async function submitPcAuth(event) {
    event.preventDefault();
    setPcAuthBusy(true);
    setPcAuthError('');
    try {
      await runPcAuth(pcAuthMode, {
        account: els.pcAuthAccountInput.value,
        password: els.pcAuthPasswordInput.value,
        nickname: els.pcAuthNicknameInput.value
      });
    } catch (error) {
      const message = error && error.name === 'AbortError' ? '请求超时，请检查网络后重试' : (error && error.message) || '网络错误';
      setPcAuthError(message);
    } finally {
      setPcAuthBusy(false);
    }
  }

  function inlineAuthEls() {
    const form = $('inlineAuthForm');
    if (!form) return {};
    return {
      form,
      accountInput: $('inlineAuthAccountInput'),
      nicknameField: $('inlineAuthNicknameField'),
      nicknameInput: $('inlineAuthNicknameInput'),
      passwordInput: $('inlineAuthPasswordInput'),
      error: $('inlineAuthError'),
      submitBtn: $('inlineAuthSubmitBtn'),
      tabs: Array.from(form.querySelectorAll('[data-inline-auth-mode]'))
    };
  }

  function setInlineAuthError(message) {
    const auth = inlineAuthEls();
    if (!auth.error) return;
    auth.error.textContent = message || '';
    auth.error.classList.toggle('show', !!message);
  }

  function updateInlineAuthSubmitLabel(busy) {
    const auth = inlineAuthEls();
    if (!auth.submitBtn) return;
    if (busy) {
      auth.submitBtn.textContent = inlineAuthMode === 'register' ? '创建中…' : '登录中…';
      return;
    }
    auth.submitBtn.textContent = inlineAuthMode === 'register' ? '创建账号' : '登录';
  }

  function setInlineAuthBusy(busy) {
    const auth = inlineAuthEls();
    if (!auth.submitBtn) return;
    auth.submitBtn.disabled = !!busy;
    updateInlineAuthSubmitLabel(!!busy);
  }

  function setInlineAuthMode(mode) {
    inlineAuthMode = mode === 'register' ? 'register' : 'login';
    const auth = inlineAuthEls();
    const isRegister = inlineAuthMode === 'register';
    auth.tabs?.forEach((button) => {
      button.classList.toggle('active', button.dataset.inlineAuthMode === inlineAuthMode);
    });
    if (auth.nicknameField) auth.nicknameField.hidden = !isRegister;
    if (auth.passwordInput) {
      auth.passwordInput.autocomplete = isRegister ? 'new-password' : 'current-password';
      auth.passwordInput.placeholder = isRegister ? '至少 6 位' : '输入登录密码';
    }
    setInlineAuthError('');
    updateInlineAuthSubmitLabel(false);
  }

  async function submitInlineAuth(event) {
    event.preventDefault();
    const auth = inlineAuthEls();
    setInlineAuthBusy(true);
    setInlineAuthError('');
    try {
      await runPcAuth(inlineAuthMode, {
        account: auth.accountInput?.value || '',
        password: auth.passwordInput?.value || '',
        nickname: auth.nicknameInput?.value || ''
      });
    } catch (error) {
      const message = error && error.name === 'AbortError' ? '请求超时，请检查网络后重试' : (error && error.message) || '网络错误';
      setInlineAuthError(message);
    } finally {
      setInlineAuthBusy(false);
    }
  }

  function bindInlineAuthForm() {
    const auth = inlineAuthEls();
    if (!auth.form) return;
    auth.tabs.forEach((button) => button.addEventListener('click', () => setInlineAuthMode(button.dataset.inlineAuthMode)));
    auth.form.addEventListener('submit', submitInlineAuth);
    setInlineAuthMode('login');
    setTimeout(() => {
      const nextAuth = inlineAuthEls();
      if (nextAuth.accountInput) nextAuth.accountInput.focus();
    }, 0);
  }

  function inlineAuthStateMarkup(title, description) {
    return `<div class="empty-state">
      <strong>${escapeHtml(title)}</strong>
      <p>${escapeHtml(description)}</p>
      <form class="inline-auth-card" id="inlineAuthForm">
        <div class="inline-auth-tabs" role="tablist" aria-label="账号入口">
          <button class="inline-auth-tab active" type="button" data-inline-auth-mode="login">登录</button>
          <button class="inline-auth-tab" type="button" data-inline-auth-mode="register">注册</button>
        </div>
        <label class="inline-auth-field">
          <span>账号</span>
          <input id="inlineAuthAccountInput" autocomplete="username" placeholder="手机号、邮箱或账号 ID" />
        </label>
        <label class="inline-auth-field" id="inlineAuthNicknameField" hidden>
          <span>昵称</span>
          <input id="inlineAuthNicknameInput" autocomplete="nickname" placeholder="工作台展示名" />
        </label>
        <label class="inline-auth-field">
          <span>密码</span>
          <input id="inlineAuthPasswordInput" type="password" autocomplete="current-password" placeholder="输入登录密码" />
        </label>
        <div class="inline-auth-error" id="inlineAuthError" aria-live="polite"></div>
        <button class="inline-auth-submit" type="submit" id="inlineAuthSubmitBtn">登录</button>
      </form>
    </div>`;
  }

  function setRails(kind) {
    els.aiRail.classList.toggle('active', kind === 'ai' || kind === 'store' || kind === 'tasks' || kind === 'project-conversation');
    els.friendsRail.classList.toggle('active', kind === 'friends');
    els.projectsRail.classList.toggle('active', kind === 'projects' || kind === 'project-plaza');
    els.projectPlazaRail.classList.toggle('active', false);
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
    if (els.settingsUserId) {
      const fullUserId = clean(state.user && state.user.id);
      els.settingsUserId.textContent = shortUserId(fullUserId);
      els.settingsUserId.title = fullUserId || '';
    }
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

  function newAiConversationId() {
    return `chat-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
  }

  function aiTitleFromContent(content) {
    const title = clean(String(content || '').replace(/\s+/g, ' '));
    if (!title) return '新对话';
    return title.length > 28 ? `${title.slice(0, 28)}…` : title;
  }

  function aiConversationTitle(conversation) {
    return clean(conversation && conversation.title)
      || aiTitleFromContent(conversation && conversation.last_message)
      || '新对话';
  }

  function aiConversationPreview(conversation) {
    const last = clean(conversation && conversation.last_message);
    if (last) return last.length > 36 ? `${last.slice(0, 36)}…` : last;
    const updated = clean(conversation && (conversation.last_message_at || conversation.updated_at || conversation.updatedAt));
    return updated ? `更新 ${formatTime(updated)}` : '还没有消息';
  }

  async function loadAiConversations() {
    if (!state.token) {
      state.aiConversations = [];
      return state.aiConversations;
    }
    const data = await api('/api/me/ai/conversations?limit=80', { cache: 'no-store' });
    state.aiConversations = Array.isArray(data.conversations) ? data.conversations : [];
    return state.aiConversations;
  }

  function startNewAiChat(focusComposer) {
    state.activeAiConversationId = '';
    state.activeAiConversationTitle = '';
    return selectAiAssistant(focusComposer, '', true);
  }

  function renderChannels() {
    const query = filterText();
    if (state.activeKind === 'ai' || state.activeKind === 'store' || state.activeKind === 'tasks' || state.activeKind === 'project-conversation') return renderAiSidebar(query);
    if (state.activeKind === 'friends') return renderFriendChannels(query);
    if (state.activeKind === 'projects' || state.activeKind === 'project-plaza') return renderProjectCenterChannels(query);
    if (state.activeKind === 'apk') return renderApkChannels();
    if (state.activeKind === 'doctor') return doctor.renderChannels(channelButton);
    if (state.activeKind === 'node') return node.renderChannels(channelButton);
    if (state.activeKind === 'voice') return window.ElonVoiceProject.renderChannels(voiceContext());
    return renderProjectChannels(query);
  }

  function aiSidebarButton(item) {
    const extraClass = clean(item.className);
    return `<button class="channel-item ${item.primary ? 'ai-primary' : ''} ${extraClass ? escapeHtml(extraClass) : ''} ${item.active ? 'active' : ''}" type="button" data-ai-action="${escapeHtml(item.id)}">
      <span class="glyph">${escapeHtml(item.glyph || '#')}</span>
      <span class="main"><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.sub || '')}</span></span>
    </button>`;
  }

  function aiSidebarProjects() {
    return state.projects
      .filter((project) => project && project.id && !isSystemProject(project) && !isDefaultJointProject(project))
      .slice()
      .sort((left, right) => String(right.updated_at || right.updatedAt || '').localeCompare(String(left.updated_at || left.updatedAt || '')));
  }

  function currentUserId() {
    return clean(state.user && (state.user.id || state.user.user_id || state.user.userId));
  }

  function conversationIdOf(conversation) {
    return clean(conversation && (conversation.id || conversation.conversation_id || conversation.conversationId));
  }

  function projectConversationDraftKey(projectId, conversationId) {
    return `${clean(projectId)}::${clean(conversationId)}`;
  }

  function draftProjectConversationId() {
    const raw = window.crypto && window.crypto.randomUUID
      ? window.crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2, 12)}`;
    return `pc-${String(raw).replace(/[^a-zA-Z0-9_-]/g, '').slice(0, 48)}`;
  }

  function aiConversationCache() {
    const userId = currentUserId();
    const cache = state.aiProjectConversations;
    if (cache.userId !== userId) {
      cache.userId = userId;
      cache.items = {};
      cache.errors = {};
      cache.loadingIds = {};
      cache.expandedIds = {};
      cache.drafts = {};
    }
    return cache;
  }

  function aiProjectDrafts(projectId) {
    const cache = aiConversationCache();
    return Object.values(cache.drafts || {})
      .filter((draft) => sameId(draft && draft.project_id, projectId))
      .sort((left, right) => String(right.updated_at || '').localeCompare(String(left.updated_at || '')));
  }

  function mergeAiProjectDrafts(projectId, conversations) {
    const items = Array.isArray(conversations) ? conversations.slice() : [];
    const ids = new Set(items.map(conversationIdOf).filter(Boolean));
    const drafts = aiProjectDrafts(projectId).filter((draft) => !ids.has(conversationIdOf(draft)));
    return drafts.concat(items);
  }

  function isDraftProjectConversation(projectId, conversationId) {
    return !!aiConversationCache().drafts[projectConversationDraftKey(projectId, conversationId)];
  }

  function rememberAiProject(projectId) {
    const cleanProjectId = clean(projectId);
    if (!cleanProjectId) return;
    try {
      localStorage.setItem('elon_pc_last_ai_project_id', cleanProjectId);
    } catch (_) {}
  }

  function preferredAiProjectForNewConversation(projectId) {
    const projects = aiSidebarProjects();
    if (!projects.length) return null;
    const explicit = clean(projectId);
    if (explicit) {
      const project = projects.find((item) => sameId(item.id, explicit));
      if (project) return project;
    }
    if (state.activeProjectId) {
      const active = projects.find((item) => sameId(item.id, state.activeProjectId));
      if (active) return active;
    }
    try {
      const lastProjectId = clean(localStorage.getItem('elon_pc_last_ai_project_id'));
      const last = projects.find((item) => sameId(item.id, lastProjectId));
      if (last) return last;
    } catch (_) {}
    return projects[0];
  }

  function createDraftProjectConversation(project) {
    const projectId = clean(project && project.id);
    const userId = currentUserId();
    if (!projectId || !userId) return null;
    const cache = aiConversationCache();
    const id = draftProjectConversationId();
    const nowIso = new Date().toISOString();
    const draft = {
      id,
      project_id: projectId,
      user_id: userId,
      user_account: userName(state.user),
      title: '新对话',
      status: 'draft',
      is_draft: true,
      message_count: 0,
      task_count: 0,
      created_at: nowIso,
      updated_at: nowIso,
      last_message_at: nowIso
    };
    cache.drafts[projectConversationDraftKey(projectId, id)] = draft;
    cache.items[projectId] = mergeAiProjectDrafts(projectId, cache.items[projectId] || []);
    cache.expandedIds[projectId] = true;
    rememberAiProject(projectId);
    return draft;
  }

  function aiSidebarVisible() {
    return state.activeKind === 'ai' || state.activeKind === 'store' || state.activeKind === 'tasks' || state.activeKind === 'project-conversation';
  }

  async function loadAiProjectConversations(project, force) {
    const userId = currentUserId();
    const projectId = clean(project && project.id);
    if (!state.token || !userId || !projectId) return;
    const cache = aiConversationCache();
    if (!force && (cache.items[projectId] || cache.errors[projectId] || cache.loadingIds[projectId])) return;
    cache.loadingIds[projectId] = true;
    if (force) {
      delete cache.items[projectId];
      delete cache.errors[projectId];
    }
    try {
      const data = await api(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(userId)}/conversations?limit=20`, { cache: 'no-store' });
      const serverConversations = Array.isArray(data.conversations) ? data.conversations : [];
      const serverIds = new Set(serverConversations.map(conversationIdOf).filter(Boolean));
      Object.keys(cache.drafts || {}).forEach((key) => {
        const draft = cache.drafts[key];
        if (sameId(draft && draft.project_id, projectId) && serverIds.has(conversationIdOf(draft))) {
          delete cache.drafts[key];
        }
      });
      cache.items[projectId] = mergeAiProjectDrafts(projectId, serverConversations);
      delete cache.errors[projectId];
    } catch (error) {
      cache.errors[projectId] = clean(error && error.message) || '最近会话加载失败';
    } finally {
      delete cache.loadingIds[projectId];
    }
  }

  function ensureAiProjectConversations(projects) {
    if (!state.token || !currentUserId()) return;
    const cache = aiConversationCache();
    const missing = projects.filter((project) => {
      const projectId = clean(project && project.id);
      return projectId && !cache.items[projectId] && !cache.errors[projectId] && !cache.loadingIds[projectId];
    });
    if (!missing.length) return;
    Promise.allSettled(missing.map((project) => loadAiProjectConversations(project, false))).then(() => {
      if (aiSidebarVisible()) renderAiSidebar(filterText());
    });
  }

  function compactText(value, maxLength) {
    const text = clean(value).replace(/\s+/g, ' ');
    const max = maxLength || 32;
    return text.length > max ? `${text.slice(0, max - 1)}…` : text;
  }

  function conversationTitle(conversation) {
    const title = clean(conversation && (conversation.title || conversation.conversation_title || conversation.conversationTitle));
    if (title && title !== '项目开发会话') return title;
    const last = compactText(conversation && (conversation.last_message || conversation.lastMessage || conversation.message), 30);
    return last || title || '项目 AI 会话';
  }

  function conversationTimeValue(conversation) {
    return clean(conversation && (conversation.last_message_at || conversation.lastMessageAt || conversation.updated_at || conversation.updatedAt || conversation.created_at || conversation.createdAt));
  }

  function relativeTimeLabel(value) {
    if (!value) return '';
    const date = new Date(Number(value) || value);
    if (Number.isNaN(date.getTime())) return formatTime(value);
    const diffMs = Math.max(0, Date.now() - date.getTime());
    const minute = 60 * 1000;
    const hour = 60 * minute;
    const day = 24 * hour;
    if (diffMs < minute) return '刚刚';
    if (diffMs < hour) return `${Math.max(1, Math.floor(diffMs / minute))} 分钟`;
    if (diffMs < day) return `${Math.max(1, Math.floor(diffMs / hour))} 小时`;
    if (diffMs < 7 * day) return `${Math.max(1, Math.floor(diffMs / day))} 天`;
    if (diffMs < 31 * day) return `${Math.max(1, Math.floor(diffMs / (7 * day)))} 周`;
    if (diffMs < 365 * day) return `${Math.max(1, Math.floor(diffMs / (31 * day)))} 个月`;
    return `${Math.max(1, Math.floor(diffMs / (365 * day)))} 年`;
  }

  function conversationMatches(conversation, query) {
    if (!query) return true;
    const text = [
      conversationTitle(conversation),
      conversation && (conversation.last_message || conversation.lastMessage),
      conversation && (conversation.last_task_status || conversation.lastTaskStatus)
    ].map(clean).join(' ').toLowerCase();
    return text.includes(query);
  }

  function renderAiProjectGroup(project, query) {
    const cache = aiConversationCache();
    const projectId = clean(project && project.id);
    const title = titleOf(project);
    const projectText = `${title} ${project.workspace_path || project.workspacePath || ''}`.toLowerCase();
    const loadedConversations = cache.items[projectId] || [];
    const projectMatched = !query || projectText.includes(query);
    const conversations = projectMatched
      ? loadedConversations
      : loadedConversations.filter((conversation) => conversationMatches(conversation, query));
    if (query && !projectMatched && !conversations.length) return '';
    const expanded = !!cache.expandedIds[projectId];
    const visibleConversations = expanded ? conversations : conversations.slice(0, 5);
    const conversationRows = visibleConversations.map((conversation) => {
      const conversationId = conversationIdOf(conversation);
      const draft = isDraftProjectConversation(projectId, conversationId);
      const active = state.activeKind === 'project-conversation'
        && sameId(projectId, state.activeProjectId)
        && sameId(conversationId, state.activeConversationId);
      return `<button class="ai-project-thread ${active ? 'active' : ''} ${draft ? 'draft' : ''}" type="button" data-ai-project-id="${escapeHtml(projectId)}" data-ai-conversation-id="${escapeHtml(conversationId)}">
        <span class="ai-project-thread-title">${escapeHtml(conversationTitle(conversation))}</span>
        <span class="ai-project-thread-time">${escapeHtml(draft ? '草稿' : relativeTimeLabel(conversationTimeValue(conversation)))}</span>
      </button>`;
    }).join('');
    const expandRow = conversations.length > 5
      ? `<button class="ai-project-expand" type="button" data-ai-project-expand-id="${escapeHtml(projectId)}">${expanded ? '收起' : '展开显示'}</button>`
      : '';
    const loading = cache.loadingIds[projectId] ? '<div class="ai-project-loading">加载最近任务…</div>' : '';
    const error = cache.errors[projectId] ? `<div class="ai-project-loading">${escapeHtml(cache.errors[projectId])}</div>` : '';
    const empty = !loading && !error && cache.items[projectId] && !conversations.length
      ? '<div class="ai-project-empty">暂无最近任务</div>'
      : '';
    const folderActive = (state.activeKind === 'project' || state.activeKind === 'project-conversation') && sameId(projectId, state.activeProjectId);
    return `<div class="ai-project-group">
      <div class="ai-project-folder-row">
        <button class="ai-project-folder ${folderActive ? 'active' : ''}" type="button" data-ai-project-folder-id="${escapeHtml(projectId)}">
          <span class="ai-project-folder-icon" aria-hidden="true"></span>
          <span class="ai-project-folder-title">${escapeHtml(title)}</span>
        </button>
        <button class="ai-project-new-chat" type="button" title="在 ${escapeHtml(title)} 下新建对话" aria-label="在 ${escapeHtml(title)} 下新建对话" data-ai-project-new-chat-id="${escapeHtml(projectId)}">+</button>
      </div>
      ${conversationRows || loading || error || empty}
      ${expandRow}
    </div>`;
  }

  function renderAiSidebar(query) {
    const actionMatches = (item) => !query || `${item.title} ${item.sub || ''}`.toLowerCase().includes(query);
    const primaryActions = [
      {
        id: 'new-chat',
        glyph: '+',
        title: '新对话',
        sub: state.token ? '默认聊天' : '登录后开启',
        primary: true,
        active: state.activeKind === 'ai' && !state.activeAiConversationId
      },
      {
        id: 'new-project',
        glyph: '+',
        title: '新建项目',
        sub: state.token ? '聊天深入后可直接开发' : '登录后创建',
        className: 'ai-project-create-action'
      },
      {
        id: 'my-projects',
        glyph: '项',
        title: '我的项目',
        sub: state.token ? '跳转到项目中心' : '登录后查看',
        className: 'ai-my-projects-action',
        active: state.activeKind === 'projects'
      }
    ].filter(actionMatches);
    const conversations = state.aiConversations
      .filter((conversation) => {
        if (!query) return true;
        return `${aiConversationTitle(conversation)} ${aiConversationPreview(conversation)}`
          .toLowerCase()
          .includes(query);
      })
      .slice(0, 40);
    const conversationItems = conversations.map((conversation) => {
      const title = aiConversationTitle(conversation);
      const sub = aiConversationPreview(conversation);
      const timeValue = conversationTimeValue(conversation);
      const time = relativeTimeLabel(timeValue);
      const active = state.activeKind === 'ai' && sameId(conversation.id, state.activeAiConversationId);
      return `<button class="channel-item ai-history-item ${active ? 'active' : ''}" type="button" data-ai-chat-conversation-id="${escapeHtml(conversation.id)}">
        <span class="glyph">聊</span>
        <span class="main"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(sub)}</span></span>
        ${time ? `<span class="channel-time" title="${escapeHtml(formatTime(timeValue))}">${escapeHtml(time)}</span>` : ''}
      </button>`;
    }).join('');
    const projects = state.token ? aiSidebarProjects() : [];
    if (projects.length) ensureAiProjectConversations(projects.slice(0, 12));
    const projectGroups = projects.map((project) => renderAiProjectGroup(project, query)).filter(Boolean).join('');
    const sections = [
      '<div class="channel-section">一龙AI</div>',
      primaryActions.map(aiSidebarButton).join('') || '<div class="ai-sidebar-muted">没有匹配的功能</div>',
      '<div class="ai-sidebar-spacer"></div>',
      '<div class="channel-section">聊天历史</div>',
      conversationItems || `<div class="ai-sidebar-muted">${state.token ? '暂无聊天历史' : '登录后显示聊天历史'}</div>`,
      '<div class="ai-sidebar-spacer"></div>',
      '<div class="channel-section">项目</div>',
      projectGroups || `<div class="ai-sidebar-muted">${state.token ? '暂无项目，先从上方新建项目开始' : '登录后显示项目'}</div>`
    ];
    els.channelList.innerHTML = sections.join('');
    els.channelList.querySelectorAll('[data-ai-action]').forEach((btn) => {
      btn.addEventListener('click', () => {
        const action = btn.dataset.aiAction;
        if (action === 'new-chat') return startNewAiChat(true);
        if (action === 'new-project') return openAiProjectCreate();
        if (action === 'my-projects') return selectProjectsHome();
      });
    });
    els.channelList.querySelectorAll('[data-ai-chat-conversation-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectAiAssistant(true, btn.dataset.aiChatConversationId));
    });
    els.channelList.querySelectorAll('[data-ai-project-folder-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.aiProjectFolderId, {
        preferredChannelKind: 'ai_development',
        focusComposer: true
      }));
    });
    els.channelList.querySelectorAll('[data-ai-project-new-chat-id]').forEach((btn) => {
      btn.addEventListener('click', () => startProjectConversationFromSidebar(btn.dataset.aiProjectNewChatId));
    });
    els.channelList.querySelectorAll('[data-ai-project-id][data-ai-conversation-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProjectConversation(
        btn.dataset.aiProjectId,
        btn.dataset.aiConversationId,
        { focusComposer: true }
      ));
    });
    els.channelList.querySelectorAll('[data-ai-project-expand-id]').forEach((btn) => {
      btn.addEventListener('click', () => {
        const projectId = btn.dataset.aiProjectExpandId;
        const cache = aiConversationCache();
        cache.expandedIds[projectId] = !cache.expandedIds[projectId];
        renderAiSidebar(filterText());
      });
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

  function renderProjectGroupToggle(group, title, sub, expanded) {
    return `<button class="channel-item project-group-toggle" type="button" data-project-center-group="${escapeHtml(group)}">
      <span class="glyph">${expanded ? 'v' : '>'}</span>
      <span class="main"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(sub)}</span></span>
    </button>`;
  }

  function renderProjectCenterChannels(query) {
    const groups = state.projectCenterGroups || { mine: true, plaza: false };
    const mineExpanded = !!groups.mine || !!query;
    const plazaExpanded = !!groups.plaza || !!query;
    const projects = state.projects.filter((project) => titleOf(project).toLowerCase().includes(query));
    const joinedIds = new Set(state.projects.map((project) => project && project.id).filter(Boolean));
    const joined = state.plaza.projects
      .filter((project) => joinedIds.has(project.id) && titleOf(project).toLowerCase().includes(query))
      .slice(0, 20);
    const mineChildren = mineExpanded ? [
      `<button class="channel-item project-center-child ${state.activeKind === 'projects' && !state.activeProjectId ? 'active' : ''}" type="button" data-project-center-child="overview"><span class="glyph">首</span><span class="main"><strong>项目概览</strong><span>新建、导入和管理项目</span></span></button>`,
      state.token
        ? (projects.map((project) => channelButton({
          id: project.id,
          kind: 'project-entry',
          avatar: iconUrlOf(project),
          avatarFallback: titleOf(project),
          glyph: '项',
          title: titleOf(project),
          sub: `${projectRoleLabel(project)} · ${projectMemberCount(project)} 位成员`,
          nested: true,
          active: state.activeProjectId && sameId(project.id, state.activeProjectId)
        })).join('') || '<div class="empty-state project-center-empty">暂无项目</div>')
        : '<div class="empty-state project-center-empty">登录后显示我的项目</div>'
    ].join('') : '';
    const plazaChildren = plazaExpanded ? [
      `<button class="channel-item project-center-child ${state.activeKind === 'project-plaza' && state.plaza.filterKey !== 'joined' ? 'active' : ''}" type="button" data-project-plaza-child="all"><span class="glyph">全</span><span class="main"><strong>全部公开项目</strong><span>搜索、加入和下载 APK</span></span></button>`,
      `<button class="channel-item project-center-child ${state.activeKind === 'project-plaza' && state.plaza.filterKey === 'joined' ? 'active' : ''}" type="button" data-project-plaza-child="joined"><span class="glyph">已</span><span class="main"><strong>已加入</strong><span>我已加入的公开项目</span></span></button>`,
      state.token
        ? (joined.map((project) => channelButton({
          id: project.id,
          kind: 'project-entry',
          avatar: iconUrlOf(project),
          avatarFallback: titleOf(project),
          glyph: '项',
          title: titleOf(project),
          sub: `${projectMemberCount(project)} 位成员`,
          nested: true,
          active: false
        })).join('') || '<div class="empty-state project-center-empty">暂无已加入项目</div>')
        : '<div class="empty-state project-center-empty">登录后查看已加入项目</div>'
    ].join('') : '';
    els.channelList.innerHTML = [
      '<div class="channel-section">项目中心</div>',
      renderProjectGroupToggle('mine', '我的项目', '展开项目列表', mineExpanded),
      mineChildren,
      renderProjectGroupToggle('plaza', '项目广场', '展开公开项目入口', plazaExpanded),
      plazaChildren
    ].join('');
    els.channelList.querySelectorAll('[data-project-center-group]').forEach((btn) => {
      btn.addEventListener('click', () => {
        const group = btn.dataset.projectCenterGroup;
        state.projectCenterGroups[group] = !state.projectCenterGroups[group];
        renderProjectCenterChannels(filterText());
      });
    });
    els.channelList.querySelectorAll('[data-project-center-child]').forEach((btn) => {
      btn.addEventListener('click', selectProjectsHome);
    });
    els.channelList.querySelectorAll('[data-project-plaza-child]').forEach((btn) => {
      btn.addEventListener('click', () => {
        state.plaza.filterKey = btn.dataset.projectPlazaChild === 'joined' ? 'joined' : 'all';
        selectProjectPlaza();
        loadProjectPlaza(true);
      });
    });
    els.channelList.querySelectorAll('[data-peer-kind="project-entry"]').forEach((btn) => {
      btn.addEventListener('click', () => selectProject(btn.dataset.itemId));
    });
  }

  function renderProjectChannels(query) {
    const channels = ((state.projectSpace && state.projectSpace.channels) || [])
      .filter((channel) => projectChannelSearchText(channel).includes(query));
    const homeVisible = !query || '项目首页下一步入口安装使用下载应用首页开始介绍overviewhome'.includes(query);
    const homeButton = homeVisible ? channelButton({
        id: 'project-home',
        kind: 'project-home',
        glyph: '首',
        title: '项目首页',
        sub: '下一步与下载',
        active: !state.activeChannelId
      }) : '';
    if (query) {
      els.channelList.innerHTML = [
        '<div class="channel-section">搜索结果</div>',
        homeButton,
        channels.map(projectChannelButton).join('') || '<div class="empty-state">暂无频道</div>'
      ].join('');
    } else {
      const grouped = groupedProjectChannels(channels);
      const startChannels = grouped.start.map(projectChannelButton).join('');
      els.channelList.innerHTML = [
        homeButton ? '<div class="channel-section">概览</div>' : '',
        homeButton,
        startChannels ? '<div class="channel-section">开始</div>' : '',
        startChannels || (!homeButton ? '<div class="empty-state">暂无入口</div>' : ''),
        grouped.info.length ? '<div class="channel-section">项目资料</div>' : '',
        grouped.info.map(projectChannelButton).join(''),
        grouped.feedback.length ? '<div class="channel-section">需求反馈</div>' : '',
        grouped.feedback.map(projectChannelButton).join(''),
        grouped.other.length ? '<div class="channel-section">其他</div>' : '',
        grouped.other.map(projectChannelButton).join('')
      ].join('');
    }
    els.channelList.querySelectorAll('[data-channel-id]').forEach((btn) => {
      btn.addEventListener('click', () => selectProjectChannel(btn.dataset.channelId));
    });
    els.channelList.querySelectorAll('[data-project-home]').forEach((btn) => {
      btn.addEventListener('click', selectProjectLanding);
    });
  }

  function groupedProjectChannels(channels) {
    const grouped = { start: [], info: [], feedback: [], other: [] };
    (channels || []).forEach((channel) => {
      grouped[projectChannelGroup(channel)].push(channel);
    });
    return grouped;
  }

  function projectChannelButton(channel) {
    return channelButton({
      id: channel.id,
      kind: 'project-channel',
      glyph: channelGlyph(channel),
      title: channelTitle(channel),
      sub: channelSubtitle(channel),
      active: channel.id === state.activeChannelId,
      primary: channelKind(channel) === 'ai_development'
    });
  }

  function projectChannelSearchText(channel) {
    return [
      channelTitle(channel),
      channelSubtitle(channel),
      channelKind(channel),
      clean(channel && channel.id)
    ].join(' ').toLowerCase();
  }

  function projectChannelGroup(channel) {
    const kind = channelKind(channel);
    if (['ai_development', 'builds'].includes(kind)) return 'start';
    if (['announcements', 'docs', 'discussion'].includes(kind)) return 'info';
    if (['requirements', 'suggestions', 'issues'].includes(kind)) return 'feedback';
    return 'other';
  }

  function cachedProjectConversation(projectId, conversationId) {
    const conversations = aiConversationCache().items[clean(projectId)] || [];
    return conversations.find((conversation) => sameId(conversationIdOf(conversation), conversationId)) || null;
  }

  function channelButton(item) {
    const attrs = item.kind === 'project-channel'
      ? `data-channel-id="${escapeHtml(item.id)}"`
      : (item.kind === 'project-home'
        ? 'data-project-home="1"'
      : (item.kind === 'doctor-section'
        ? `data-doctor-section="${escapeHtml(item.id)}"`
        : `data-peer-kind="${escapeHtml(item.kind)}" data-item-id="${escapeHtml(item.id)}"`));
    const glyph = item.hideGlyph
      ? ''
      : (item.avatar || item.avatarFallback
        ? avatarElement('span', 'glyph channel-avatar', item.avatar, item.avatarFallback || item.title || item.glyph || '#', item.glyph || '#')
        : `<span class="glyph">${escapeHtml(item.glyph || '#')}</span>`);
    return `<button class="channel-item ${item.primary ? 'ai-primary' : ''} ${item.nested ? 'project-center-child' : ''} ${item.hideGlyph ? 'no-glyph' : ''} ${item.active ? 'active' : ''}" type="button" ${attrs}>
      ${glyph}
      <span class="main"><strong>${escapeHtml(item.title)}</strong><span>${escapeHtml(item.sub || '')}</span></span>
      ${typeof item.online === 'boolean' ? `<i class="presence-dot ${item.online ? 'online' : ''}"></i>` : ''}
    </button>`;
  }

  function channelName(channel) {
    return clean(channel.name || channel.title || channel.display_name || channel.id) || '频道';
  }

  function channelKind(channel) {
    return clean(channel && (channel.kind || channel.channel_kind)).toLowerCase();
  }

  function projectSpaceChannelByKind(kind) {
    const target = clean(kind).toLowerCase();
    return ((state.projectSpace && state.projectSpace.channels) || [])
      .find((channel) => channelKind(channel) === target) || null;
  }

  function channelTitle(channel) {
    const name = channelName(channel);
    const kind = channelKind(channel);
    const fallback = {
      announcements: '项目公告',
      docs: '项目文档',
      discussion: '成员讨论',
      requirements: '功能需求',
      suggestions: '意见建议',
      issues: '问题反馈',
      ai_development: '开始做应用',
      builds: '生成安装包'
    };
    if (fallback[kind]) return fallback[kind];
    return (name === kind || name === clean(channel && channel.id)) ? (fallback[kind] || name) : name;
  }

  function channelSubtitle(channel) {
    const label = {
      announcements: '项目公告',
      docs: '资料文档',
      discussion: '成员讨论',
      requirements: '功能需求',
      suggestions: '意见建议',
      issues: '问题反馈',
      ai_development: '向一龙AI说需求',
      builds: '打包并交付应用'
    };
    return label[channelKind(channel)] || '项目频道';
  }

  function channelGlyph(channel) {
    const kind = channelKind(channel);
    if (kind === 'ai_development') return 'AI';
    if (kind === 'builds') return '包';
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
    if (devComposer) devComposer.render();
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
    els.doctorRail.addEventListener('click', doctor.selectDoctor);
    els.nodeRail.addEventListener('click', node.selectNode);
    els.voiceRail.addEventListener('click', () => selectVoiceProject());
    els.authClaimBtn.addEventListener('click', () => openAuthModal('register'));
    pcAuthTabs().forEach((button) => button.addEventListener('click', () => setPcAuthMode(button.dataset.pcAuthMode)));
    els.pcAuthForm.addEventListener('submit', submitPcAuth);
    els.pcAuthCloseBtn.addEventListener('click', closeAuthModal);
    document.addEventListener('pointerdown', keepAuthModalOpenOnOutsideClick, true);
    document.addEventListener('click', keepAuthModalOpenOnOutsideClick, true);
    projectCreate.bindEvents();
    [els.aiRail, els.friendsRail, els.projectsRail, els.doctorRail, els.nodeRail, els.voiceRail].forEach(attachRailTooltip);
    $('refreshBtn').addEventListener('click', refreshActive);
    $('openLegacyWebBtn').addEventListener('click', selectApkDownload);
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
    els.settingsSecurityBtn.addEventListener('click', () => setSettingsSection('security'));
    els.settingsDevicesBtn.addEventListener('click', () => setSettingsSection('devices'));
    els.settingsLogoutBtn.addEventListener('click', logout);
    els.settingsCloseBtn.addEventListener('click', closeSettings);
    els.settingsBackdrop.addEventListener('click', (event) => {
      if (event.target === els.settingsBackdrop) closeSettings();
    });
    els.chooseProjectFolderBtn.addEventListener('click', () => chooseLocalProjectFolder({ autoRegister: true }));
    els.inspectProjectFolderBtn.addEventListener('click', inspectLocalProjectFolder);
    els.registerProjectBtn.addEventListener('click', registerLocalProject);
    els.settingsProjectPath.addEventListener('input', markLocalProjectPathDirty);
    els.refreshClientMaintenanceBtn.addEventListener('click', () => refreshClientMaintenance(true));
    els.openNodeSetupFromSettingsBtn.addEventListener('click', openNodeSetupFromSettings);
    if (els.settingsRuntimePermission) {
      els.settingsRuntimePermission.addEventListener('change', () => {
        syncSettingsRuntimePermissionHint();
        refreshProjectRegistrationPreview();
      });
      syncSettingsRuntimePermissionHint();
    }
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
      autosizeComposerInput();
    });
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && els.pcAuthBackdrop && !els.pcAuthBackdrop.hidden) {
        closeAuthModal();
        return;
      }
      if (event.key === 'Escape' && els.pcProjectCreateBackdrop && !els.pcProjectCreateBackdrop.hidden) {
        projectCreate.close();
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
    window.addEventListener('resize', hideRailTooltip);
    document.addEventListener('scroll', hideRailTooltip, true);
  }

  function attachRailTooltip(button) {
    if (!button || button.dataset.tooltipBound) return;
    button.dataset.tooltipBound = '1';
    if (els.railTooltip) button.setAttribute('aria-describedby', 'railTooltip');
    button.removeAttribute('title');
    button.addEventListener('mouseenter', showRailTooltip);
    button.addEventListener('focus', showRailTooltip);
    button.addEventListener('mouseleave', hideRailTooltip);
    button.addEventListener('blur', hideRailTooltip);
  }

  function positionRailTooltip(anchor) {
    if (!els.railTooltip || !anchor) return;
    const rect = anchor.getBoundingClientRect();
    const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
    const tooltipHeight = els.railTooltip.offsetHeight || 0;
    const desiredTop = rect.top + rect.height / 2;
    const minTop = 12 + tooltipHeight / 2;
    const maxTop = Math.max(minTop, viewportHeight - 12 - tooltipHeight / 2);
    const top = Math.min(Math.max(desiredTop, minTop), maxTop);
    const arrowLimit = Math.max(0, tooltipHeight / 2 - 9);
    const arrowOffset = Math.min(Math.max(desiredTop - top, -arrowLimit), arrowLimit);
    els.railTooltip.style.left = `${Math.round(rect.right + 12)}px`;
    els.railTooltip.style.top = `${Math.round(top)}px`;
    els.railTooltip.style.setProperty('--rail-tooltip-arrow-y', `${Math.round(arrowOffset)}px`);
  }

  function showRailTooltip(event) {
    if (!els.railTooltip) return;
    const button = event.currentTarget;
    const label = clean(button.dataset.label || button.getAttribute('aria-label'));
    if (!label) return;
    els.railTooltip.textContent = label;
    els.railTooltip.classList.add('show');
    positionRailTooltip(button);
  }

  function hideRailTooltip() {
    if (!els.railTooltip) return;
    els.railTooltip.classList.remove('show');
    els.railTooltip.style.removeProperty('--rail-tooltip-arrow-y');
  }

  async function loadBaseData() {
    setAuthClaimBanner(false);
    const [me, projects, friends, groups, nodes] = await Promise.allSettled([
      api('/api/me'),
      api('/api/me/projects'),
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
    state.aiConversations = [];
    state.activeKind = 'ai';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeAiConversationId = '';
    state.activeAiConversationTitle = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
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
    setSidebarPlaceholder('搜索聊天历史');
    setHeader('AI', '需要登录', '先登录网页版，再回到 PC 工作台');
    setComposer(false, '登录后可输入消息', false);
    renderAiSidebar(filterText());
    renderMembers('成员', [], { emptyText: '登录后显示成员' });
    setNodeMode(false);
    els.messageList.innerHTML = inlineAuthStateMarkup(
      '登录后使用一龙AI、项目和 PC 工作台',
      'PC 工作台读取账号登录态。输入账号和密码后，即可进入一龙AI工作区。'
    );
    bindInlineAuthForm();
  }

  function selectProjectsHome() {
    state.activeKind = 'projects';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activePeer = null;
    state.projectSpace = null;
    state.projectCenterGroups.mine = true;
    setAuthClaimBanner(!state.token);
    setRails('projects');
    els.workspaceName.textContent = '项目中心';
    els.workspaceMeta.textContent = state.token ? `${state.projects.length} 个项目` : '需要登录';
    setSidebarPlaceholder('搜索项目');
    setHeader('项', '项目中心', '查看我的项目和项目广场');
    setComposer(false, '选择项目后开始输入', false);
    setNodeMode(false);
    renderProjectCenterChannels(filterText());
    renderMembers('项目成员', []);
    renderProjectHomeSurface();
  }

  function renderProjectHomeSurface() {
    setHeader('项', '项目中心', '查看我的项目和项目广场');
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
        <div class="pc-project-actions">
          <button class="send-button" type="button" id="projectCreateBtn">新建项目</button>
          <button class="text-button" type="button" id="projectRegisterLocalBtn">导入电脑代码文件夹</button>
          <button class="text-button" type="button" id="projectOpenWebBtn">打开网页版项目页</button>
        </div>
      </div>
      ${projects.length ? `<div class="pc-project-grid">${projects.map(renderProjectCard).join('')}</div>` : `<div class="pc-project-empty">
        <strong>还没有项目</strong>
        <span>可以新建一个项目，或导入这台电脑上已有的代码文件夹。</span>
        <div class="pc-project-empty-actions">
          <button class="send-button" type="button" id="projectEmptyCreateBtn">新建项目</button>
          <button class="text-button" type="button" id="projectEmptyRegisterBtn">导入电脑代码文件夹</button>
          <button class="text-button" type="button" id="projectEmptyPlazaBtn">去项目广场</button>
        </div>
      </div>`}
    </section>`;
    const createBtn = $('projectCreateBtn');
    if (createBtn) createBtn.addEventListener('click', openPcProjectCreate);
    const registerBtn = $('projectRegisterLocalBtn');
    if (registerBtn) registerBtn.addEventListener('click', () => openSettings('workbench', { autoPickAndRegister: true }));
    const webBtn = $('projectOpenWebBtn');
    if (webBtn) webBtn.addEventListener('click', () => window.open('/web', '_blank'));
    const emptyCreateBtn = $('projectEmptyCreateBtn');
    if (emptyCreateBtn) emptyCreateBtn.addEventListener('click', openPcProjectCreate);
    const emptyRegisterBtn = $('projectEmptyRegisterBtn');
    if (emptyRegisterBtn) emptyRegisterBtn.addEventListener('click', () => openSettings('workbench', { autoPickAndRegister: true }));
    const emptyPlazaBtn = $('projectEmptyPlazaBtn');
    if (emptyPlazaBtn) emptyPlazaBtn.addEventListener('click', selectProjectPlaza);
    els.messageList.querySelectorAll('[data-open-project-id]').forEach((button) => {
      button.addEventListener('click', () => selectProject(button.dataset.openProjectId));
    });
  }

  function openPcProjectCreate() {
    if (!state.token) {
      openAuthModal('login');
      return;
    }
    closeSettings();
    projectCreate.open();
  }

  function openAiProjectCreate() {
    if (!state.token) {
      openAuthModal('login');
      return;
    }
    closeSettings();
    projectCreate.open({
      mode: 'ai',
      afterCreate: async (project) => {
        const projectId = clean(project && project.id);
        if (projectId) await startProjectConversationFromSidebar(projectId);
        else await refreshActive();
      }
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
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activePeer = null;
    state.projectSpace = null;
    state.projectCenterGroups.plaza = true;
    setAuthClaimBanner(!state.token);
    setRails('project-plaza');
    els.workspaceName.textContent = '项目广场';
    els.workspaceMeta.textContent = state.plaza.loading ? '加载中' : `${state.plaza.projects.length} 个公开项目`;
    setSidebarPlaceholder('搜索项目广场');
    setHeader('广', '项目广场', '发现、加入和下载公开项目');
    setComposer(false, '加入项目后可输入消息', false);
    setNodeMode(false);
    renderMembers('项目广场', []);
    renderProjectCenterChannels(filterText());
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
      renderProjectCenterChannels(filterText());
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
        renderProjectCenterChannels(filterText());
        renderProjectPlazaSurface();
      }
    }
  }

  function selectStore() {
    state.activeKind = 'store';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
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
    setHeader('商', '商店', '项目和移动端入口');
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
            <strong>打开移动端</strong>
            <p>下载 Android APK，或打开 iOS / PWA 网页版。</p>
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
    state.activeConversationId = '';
    state.activeMemberUserId = '';
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
    const activeChannel = state.mobileEntryChannel || 'download';
    els.channelList.innerHTML = [
      '<div class="channel-section">移动端</div>',
      `<button class="channel-item ${activeChannel === 'download' ? 'active' : ''}" type="button" data-apk-channel="download"><span class="glyph">下</span><span class="main"><strong>下载与分享</strong><span>安装手机端</span></span></button>`,
      `<button class="channel-item ${activeChannel === 'web' ? 'active' : ''}" type="button" data-apk-channel="web"><span class="glyph">网</span><span class="main"><strong>移动网页版（iOS）</strong><span>iOS / PWA 入口</span></span></button>`
    ].join('');
    els.channelList.querySelectorAll('[data-apk-channel]').forEach((button) => {
      button.addEventListener('click', () => {
        state.mobileEntryChannel = button.dataset.apkChannel === 'web' ? 'web' : 'download';
        renderApkChannels();
        if (state.mobileEntryChannel === 'web') renderMobileWebSurface();
        else renderApkDownloadSurface();
      });
    });
  }

  function selectApkDownload() {
    state.activeKind = 'apk';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activePeer = null;
    state.projectSpace = null;
    state.mobileEntryChannel = 'download';
    setAuthClaimBanner(!state.token);
    setRails('apk');
    els.workspaceName.textContent = '移动端';
    els.workspaceMeta.textContent = '手机端下载和移动网页版（iOS）';
    setSidebarPlaceholder('搜索移动端入口');
    renderApkChannels();
    renderMembers('移动端', []);
    renderApkDownloadSurface();
  }

  function renderApkDownloadSurface() {
    const apkUrl = absoluteUrl(APK_DOWNLOAD_URL);
    const pageUrl = absoluteUrl(APK_DOWNLOAD_PAGE_URL);
    const promoText = `我正在用「一龙」云端 APK 开发平台，手机里直接提需求，云端帮你改代码、打包并生成安装包。\n\n下载地址：${apkUrl}`;
    setHeader('端', '打开移动端', '安装手机端或打开移动网页版（iOS）');
    setComposer(false, '登录后可输入消息', false);
    setNodeMode(false);
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>打开移动端</h2>
          <p>下载 Android APK、复制手机安装地址，或直接打开面向 iOS / PWA 的移动网页版。</p>
        </div>
        <button class="text-button" type="button" id="apkCopyUrlBtn">复制下载地址</button>
      </div>
      <div class="pc-apk-panel">
        <div class="pc-apk-device">
          <span class="pc-apk-screen"></span>
        </div>
        <div class="pc-apk-copy">
          <strong>一龙手机端</strong>
          <p>下载 APK 后可在手机上使用项目、账号和工作台能力；网页版和 APK 数据互通。</p>
          <div class="pc-apk-status" id="apkVersionStatus" aria-live="polite">正在读取最新版本...</div>
          <div class="pc-apk-url" id="apkUrlText">${escapeHtml(apkUrl)}</div>
          <div class="pc-apk-actions">
            <a class="primary" id="apkDownloadBtn" href="${escapeHtml(APK_DOWNLOAD_URL)}" download>下载最新 APK</a>
            <button type="button" id="apkWebBtn">打开移动网页版（iOS）</button>
            ${state.token ? '' : '<button type="button" id="apkLoginBtn">登录或注册</button>'}
          </div>
          <div class="pc-apk-share-panel">
            <label for="apkPromoText">分享给手机</label>
            <textarea id="apkPromoText" readonly>${escapeHtml(promoText)}</textarea>
            <div class="pc-apk-actions compact">
              <button type="button" id="apkCopyPromoBtn">复制推广语</button>
              <button type="button" id="apkShareBtn">系统分享</button>
            </div>
            <div class="pc-apk-tip">下载页地址：${escapeHtml(pageUrl)}</div>
          </div>
        </div>
      </div>
    </section>`;
    $('apkCopyUrlBtn').addEventListener('click', () => copyApkDownloadUrl(apkUrl));
    $('apkWebBtn').addEventListener('click', () => window.open('/web', '_blank', 'noopener'));
    $('apkCopyPromoBtn').addEventListener('click', () => copyApkPromoText(promoText));
    $('apkShareBtn').addEventListener('click', () => shareApkDownload(apkUrl, promoText));
    const apkLoginBtn = $('apkLoginBtn');
    if (apkLoginBtn) apkLoginBtn.addEventListener('click', () => openAuthModal('login'));
    loadApkVersion();
  }

  function renderMobileWebSurface() {
    setHeader('网', '移动网页版（iOS）', '在浏览器中使用 iOS / PWA 入口');
    setComposer(false, '登录后可输入消息', false);
    setNodeMode(false);
    els.messageList.innerHTML = `<section class="pc-project-view">
      <div class="pc-project-hero">
        <div>
          <h2>移动网页版（iOS）</h2>
          <p>主要服务 iOS / PWA 场景，需要在浏览器里快速登录、同步账号或体验手机端界面时，从这里打开。</p>
        </div>
        <button class="text-button" type="button" id="openMobileWebPageBtn">打开移动网页版（iOS）</button>
      </div>
      <div class="pc-apk-panel">
        <div class="pc-apk-device">
          <span class="pc-apk-screen web"></span>
        </div>
        <div class="pc-apk-copy">
          <strong>浏览器入口</strong>
          <p>移动网页版（iOS）会在新标签中打开；下载 APK 和分享安装地址仍保留在“下载与分享”频道里。</p>
          <div class="pc-apk-url">${escapeHtml(absoluteUrl('/web'))}</div>
          <div class="pc-apk-actions">
            <button class="primary" type="button" id="mobileWebOpenBtn">打开移动网页版（iOS）</button>
            <button type="button" id="mobileBackToDownloadBtn">回到下载与分享</button>
          </div>
        </div>
      </div>
    </section>`;
    $('openMobileWebPageBtn').addEventListener('click', () => window.open('/web', '_blank', 'noopener'));
    $('mobileWebOpenBtn').addEventListener('click', () => window.open('/web', '_blank', 'noopener'));
    $('mobileBackToDownloadBtn').addEventListener('click', () => {
      state.mobileEntryChannel = 'download';
      renderApkChannels();
      renderApkDownloadSurface();
    });
  }

  function absoluteUrl(path) {
    try {
      return new URL(path, window.location.href).href;
    } catch (_) {
      return path;
    }
  }

  async function copyPlainText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return;
    }
    const el = document.createElement('textarea');
    el.value = text;
    el.style.position = 'fixed';
    el.style.left = '-9999px';
    document.body.appendChild(el);
    el.focus();
    el.select();
    document.execCommand('copy');
    document.body.removeChild(el);
  }

  function setApkStatus(message) {
    const status = $('apkVersionStatus');
    if (status) status.textContent = message;
  }

  async function copyApkDownloadUrl(apkUrl) {
    await copyPlainText(apkUrl);
    setApkStatus('下载地址已复制');
    window.setTimeout(loadApkVersion, 1400);
  }

  async function copyApkPromoText(promoText) {
    await copyPlainText(promoText);
    setApkStatus('推广语已复制');
    window.setTimeout(loadApkVersion, 1400);
  }

  async function shareApkDownload(apkUrl, promoText) {
    if (navigator.share) {
      await navigator.share({ title: '一龙 APK 下载', text: promoText, url: apkUrl });
      return;
    }
    await copyPlainText(promoText);
    setApkStatus('当前浏览器不支持系统分享，已复制推广语');
    window.setTimeout(loadApkVersion, 1600);
  }

  async function loadApkVersion() {
    const status = $('apkVersionStatus');
    if (!status) return;
    try {
      const res = await fetch('/app/version.json', { cache: 'no-store' });
      const data = await res.json();
      const version = data.versionName || data.version_name || '?';
      const code = data.versionCode || data.version_code || '?';
      const size = data.fileSize ? ` · ${(data.fileSize / 1048576).toFixed(1)} MB` : '';
      status.textContent = `最新版本：v${version} (build ${code})${size}`;
    } catch (_) {
      status.textContent = '最新版本信息暂不可用，下载地址保持固定可用。';
    }
  }

  async function selectAiAssistant(focusComposer, conversationId, forceNew) {
    state.activeKind = 'ai';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activeChannelKind = '';
    state.projectSpace = null;
    const aiFriend = socialAiFriend();
    state.activePeer = null;
    setAuthClaimBanner(!state.token);
    setRails('ai');
    els.workspaceName.textContent = '一龙AI';
    els.workspaceMeta.textContent = '普通聊天';
    setSidebarPlaceholder('搜索聊天历史');
    renderMembers('一龙AI', aiFriend ? [Object.assign({}, aiFriend, { name: userName(aiFriend), sub: aiFriend.is_online ? '在线' : '离线' })] : []);
    setHeader('AI', '一龙AI', '普通聊天');
    setComposer(!!state.token, state.token ? '发送给一龙AI' : '登录后可输入消息', false);
    setNodeMode(false);
    if (!state.token) {
      state.aiConversations = [];
      state.activeAiConversationId = '';
      state.activeAiConversationTitle = '';
      renderChannels();
      els.messageList.innerHTML = inlineAuthStateMarkup(
        '登录后使用一龙AI',
        '登录后可直接提问，普通聊天历史会显示在左侧。输入账号和密码后，即可进入一龙AI工作区。'
      );
      bindInlineAuthForm();
      return;
    }
    let conversations = state.aiConversations;
    try {
      conversations = await loadAiConversations();
    } catch (error) {
      showError(error);
      conversations = [];
      state.aiConversations = [];
    }

    const requestedId = clean(conversationId);
    const latestId = clean(conversations[0] && conversations[0].id);
    const targetId = forceNew ? '' : (requestedId || clean(state.activeAiConversationId) || latestId);
    const target = conversations.find((conversation) => sameId(conversation.id, targetId));
    state.activeAiConversationId = targetId;
    state.activeAiConversationTitle = target ? aiConversationTitle(target) : '';
    renderChannels();

    if (!targetId) {
      setHeader('AI', '一龙AI', '新对话');
      els.messageList.innerHTML = '<div class="empty-state"><strong>新对话</strong><p>从下方输入第一句话，会自动保存到左侧聊天历史。</p></div>';
      if (focusComposer) setTimeout(() => els.input.focus(), 0);
      return;
    }

    const headerTitle = target ? aiConversationTitle(target) : '一龙AI';
    setHeader('AI', headerTitle, '普通聊天');
    els.messageList.innerHTML = '<div class="empty-state">加载聊天消息中…</div>';
    try {
      const data = await api(`/api/me/ai/conversations/${encodeURIComponent(targetId)}/messages?limit=120`, { cache: 'no-store' });
      renderMessages(data.messages || [], 'ai');
      setBadge(els.aiBadge, 0);
      if (focusComposer) setTimeout(() => els.input.focus(), 0);
    } catch (error) {
      showError(error);
    }
  }

  async function startProjectConversationFromSidebar(projectId) {
    if (!state.token || !currentUserId()) {
      openAuthModal('login');
      return;
    }
    const project = preferredAiProjectForNewConversation(projectId);
    if (!project) {
      window.alert('请先注册或加入一个项目，新对话会显示在对应项目下面。');
      selectProjectsHome();
      return;
    }
    const draft = createDraftProjectConversation(project);
    if (!draft) return;
    await selectProjectConversation(project.id, draft.id, { focusComposer: true, draftOnly: true });
  }

  function renderDirectMembers(friend) {
    const members = [
      Object.assign({}, friend, {
        name: userName(friend),
        sub: friend && friend.is_online ? '在线' : '离线'
      })
    ];
    if (state.user && currentUserId()) {
      members.unshift(Object.assign({}, state.user, {
        id: currentUserId(),
        name: userName(state.user),
        sub: '我',
        is_online: true
      }));
    }
    renderMembers('私聊成员', members, {
      groupBy: 'presence',
      emptyText: '暂无成员'
    });
  }

  function renderFriendGroupMembers(group) {
    const preview = Array.isArray(group && group.members) ? group.members : [];
    const members = preview.map((member) => {
      const id = memberIdOf(member);
      const friend = friendById(id);
      const isSelf = id && sameId(id, currentUserId());
      const onlineKnown = isSelf || (friend && friend.is_online === true);
      const offlineKnown = friend && friend.is_online === false;
      return Object.assign({}, member, {
        name: memberNameOf(member),
        sub: onlineKnown ? '在线' : '群成员',
        is_online: onlineKnown ? true : (offlineKnown ? false : undefined)
      });
    });
    if (state.user && currentUserId() && !members.some((member) => sameId(memberIdOf(member), currentUserId()))) {
      members.unshift(Object.assign({}, state.user, {
        id: currentUserId(),
        name: userName(state.user),
        sub: '我',
        is_online: true
      }));
    }
    const total = Math.max(Number(group && (group.member_count || group.memberCount || group.members_count || group.membersCount) || 0), members.length);
    renderMembers('群成员', members, {
      groupBy: 'presence',
      totalCount: total,
      emptyText: '暂无群成员',
      overflowText: total > members.length ? `还有 ${total - members.length} 位成员未在预览中显示` : ''
    });
  }

  function selectFriends() {
    state.activeKind = 'friends';
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activeChannelKind = '';
    if (!state.activePeer || (state.activePeer.kind === 'friend' && sameId(state.activePeer.id, SOCIAL_AI_USER_ID))) {
      state.activePeer = null;
    }
    setRails('friends');
    els.workspaceName.textContent = '好友';
    els.workspaceMeta.textContent = `${socialFriends().length} 位好友 · ${state.groups.length} 个群聊`;
    setSidebarPlaceholder('搜索好友或群聊');
    renderChannels();
    renderMembers('好友在线', socialFriends().map((f) => Object.assign({}, f, { name: userName(f), sub: f.is_online ? '在线' : '离线' })), {
      groupBy: 'presence',
      emptyText: '暂无好友'
    });
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
    state.activeProjectId = '';
    state.activeChannelId = '';
    state.activeChannelKind = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    state.activePeer = { kind, id };
    renderChannels();
    const title = kind === 'group' ? clean(item.name || item.title || '群聊') : userName(item);
    setHeader(kind === 'group' ? '群' : '@', title, kind === 'group' ? '群聊频道' : (item.is_online ? '在线好友' : '离线好友'));
    setComposer(true, `发送给 ${title}`, false);
    setNodeMode(false);
    if (kind === 'group') renderFriendGroupMembers(item);
    else renderDirectMembers(item);
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
    state.activeConversationId = '';
    state.activeMemberUserId = '';
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

  function renderProjectMembers(title, members, options) {
    const rows = (Array.isArray(members) ? members : []).map((member) => {
      const id = memberIdOf(member);
      return Object.assign({}, member, {
        id: id || member.id,
        name: memberNameOf(member),
        sub: memberRoleLabel(member) || '项目成员',
        is_online: id && sameId(id, currentUserId()) ? true : undefined
      });
    });
    renderMembers(title || '项目成员', rows, Object.assign({
      groupBy: 'role',
      emptyText: '暂无项目成员'
    }, options || {}));
  }

  async function selectProject(projectId, options) {
    const project = state.projects.find((p) => String(p.id) === String(projectId));
    if (!project) return;
    state.activeKind = 'project';
    state.activeProjectId = String(projectId);
    state.activeChannelId = '';
    state.activeChannelKind = '';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
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
      renderProjectMembers('项目成员', members, {
        prefixHtml: projectReadiness.renderMemberPanel(projectById(projectId) || project)
      });
      projectReadiness.bindMemberPanel(projectById(projectId) || project);
      renderChannels();
      const preferredKind = clean(options && options.preferredChannelKind).toLowerCase();
      const preferredChannel = preferredKind ? projectSpaceChannelByKind(preferredKind) : null;
      if (preferredChannel) {
        await selectProjectChannel(preferredChannel.id, {
          focusComposer: !!(options && options.focusComposer)
        });
      } else {
        selectProjectLanding();
      }
    } catch (error) {
      showError(error);
    }
  }

  async function selectProjectConversation(projectId, conversationId, options) {
    const project = projectById(projectId);
    const userId = currentUserId();
    const cleanConversationId = clean(conversationId);
    if (!project || !userId || !cleanConversationId) return;
    const conversation = cachedProjectConversation(projectId, cleanConversationId);
    const isDraft = isDraftProjectConversation(projectId, cleanConversationId) || (conversation && conversation.is_draft);
    state.activeKind = 'project-conversation';
    state.activeProjectId = String(projectId);
    state.activeConversationId = cleanConversationId;
    state.activeMemberUserId = userId;
    state.activeChannelId = '';
    state.activeChannelKind = 'member_conversation';
    state.activePeer = null;
    setAuthClaimBanner(false);
    setRails('project-conversation');
    els.workspaceName.textContent = titleOf(project);
    els.workspaceMeta.textContent = '项目 AI 会话';
    setSidebarPlaceholder('搜索项目和会话');
    setHeader('项', conversationTitle(conversation), titleOf(project));
    setComposer(true, '向一龙AI说明这个项目要做什么', true);
    setNodeMode(false);
    renderChannels();
    rememberAiProject(projectId);
    if (isDraft || (options && options.draftOnly)) {
      renderMembers('项目会话', [{ name: userName(state.user), sub: titleOf(project), is_online: true }], {
        groupBy: 'none'
      });
      els.messageList.innerHTML = `<div class="empty-state">
        <strong>新项目对话</strong>
        <p>发送第一条消息后，它会正式保存到「${escapeHtml(titleOf(project))}」下面。</p>
      </div>`;
      if (options && options.focusComposer) setTimeout(() => els.input.focus(), 0);
      return;
    }
    els.messageList.innerHTML = '<div class="empty-state">加载项目会话中…</div>';
    try {
      try {
        state.projectSpace = await api(`/api/projects/${encodeURIComponent(projectId)}/space`);
        const members = state.projectSpace.members || [];
        renderProjectMembers('项目成员', members);
      } catch (_) {
        state.projectSpace = null;
        renderMembers('项目会话', [{ name: userName(state.user), sub: '当前账号', is_online: true }], {
          groupBy: 'none'
        });
      }
      const data = await api(`/api/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(userId)}/conversations/${encodeURIComponent(cleanConversationId)}/messages?limit=120`, { cache: 'no-store' });
      renderMessages(data.messages || [], 'project');
      if (options && options.focusComposer) setTimeout(() => els.input.focus(), 0);
    } catch (error) {
      showError(error);
    }
  }

  function selectProjectLanding() {
    clearDevTaskRefresh();
    state.activeChannelId = '';
    state.activeChannelKind = 'home';
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    renderChannels();
    projectLanding.render();
  }

  async function selectProjectChannel(channelId, options) {
    const channel = ((state.projectSpace && state.projectSpace.channels) || [])
      .find((item) => String(item.id) === String(channelId));
    if (!channel) return;
    state.activeChannelId = String(channelId);
    state.activeChannelKind = clean(channel.kind || channel.channel_kind).toLowerCase();
    state.activeConversationId = '';
    state.activeMemberUserId = '';
    renderChannels();
    setHeader(channelGlyph(channel), channelTitle(channel), channelSubtitle(channel));
    const canWrite = state.activeChannelKind !== 'docs';
    setComposer(
      canWrite,
      canWrite
        ? (state.activeChannelKind === 'ai_development' ? '输入你想做的应用或要修改的功能' : `在 #${channelTitle(channel)} 发送消息`)
        : '文档频道只读',
      state.activeChannelKind === 'ai_development'
    );
    setNodeMode(false);
    els.messageList.innerHTML = '<div class="empty-state">加载频道消息中…</div>';
    try {
      const data = await api(`/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(channelId)}/messages?limit=120`);
      renderMessages(data.messages || [], 'project');
      if (options && options.focusComposer) setTimeout(() => els.input.focus(), 0);
    } catch (error) {
      showError(error);
    }
  }

  function renderMembers(title, members, options) {
    const opts = options || {};
    const normalized = (Array.isArray(members) ? members : []).map((member) => {
      const id = memberIdOf(member);
      const roleLabel = memberRoleLabel(member);
      const presence = memberPresence(member);
      const explicitSub = clean(member && (
        member.sub || member.subtitle || member.status_text || member.statusText ||
        member.activity || member.note
      ));
      return Object.assign({}, member, {
        __id: id,
        __name: memberNameOf(member),
        __presence: presence,
        __roleKey: memberRoleKey(member),
        __roleLabel: roleLabel,
        __sub: explicitSub || roleLabel || (presence === 'online' ? '在线' : (presence === 'offline' ? '离线' : '成员'))
      });
    });
    const memberCount = normalized.length;
    const totalCount = Math.max(Number(opts.totalCount || opts.total || 0), memberCount);
    const meta = totalCount ? `${totalCount} 位` : '';
    els.memberPanelTitle.innerHTML = `<span>${escapeHtml(title || '成员')}</span>${meta ? `<small>${escapeHtml(meta)}</small>` : ''}`;
    const prefix = options && options.prefixHtml ? options.prefixHtml : '';
    const groupBy = memberGroupMode(normalized, opts.groupBy);
    const groups = memberGroups(normalized, groupBy);
    const rows = groups.map((group) => {
      const groupRows = group.members.map((member) => {
        const presence = member.__presence;
        const role = member.__roleLabel;
        return `<div class="member-row ${presence ? `is-${escapeHtml(presence)}` : ''}" title="${escapeHtml(member.__name)}">
          ${avatarElement('div', `member-avatar ${presence}`, avatarUrlOf(member), member.__name, '员')}
          <div class="member-copy">
            <div class="member-line">
              <strong>${escapeHtml(member.__name)}</strong>
              ${role && groupBy !== 'role' ? `<em class="member-role-pill">${escapeHtml(role)}</em>` : ''}
            </div>
            <span class="member-sub">${escapeHtml(member.__sub)}</span>
          </div>
        </div>`;
      }).join('');
      const heading = group.label
        ? `<div class="member-section-heading"><span>${escapeHtml(group.label)}</span><small>${group.members.length}</small></div>`
        : '';
      return `<section class="member-section">${heading}${groupRows}</section>`;
    }).join('');
    const empty = `<div class="member-empty">${escapeHtml(opts.emptyText || '暂无成员')}</div>`;
    const overflow = clean(opts.overflowText)
      ? `<div class="member-overflow">${escapeHtml(opts.overflowText)}</div>`
      : '';
    els.memberList.innerHTML = prefix + (rows || empty) + overflow;
  }

  function memberGroupMode(members, requested) {
    const mode = clean(requested).toLowerCase();
    if (['none', 'presence', 'role'].includes(mode)) return mode;
    if (members.some((member) => member.__roleKey)) return 'role';
    if (members.some((member) => member.__presence)) return 'presence';
    return 'none';
  }

  function memberGroups(members, mode) {
    if (mode === 'none') return [{ label: '', members }];
    if (mode === 'role') {
      const labels = {
        owner: '拥有者',
        admin: '管理员',
        editor: '协作者',
        developer: '协作者',
        maintainer: '协作者',
        member: '成员',
        observer: '只读成员',
        viewer: '只读成员'
      };
      const order = ['owner', 'admin', 'editor', 'developer', 'maintainer', 'member', 'observer', 'viewer'];
      return groupedMembers(members, (member) => memberRoleGroupKey(member.__roleKey), (key) => labels[key] || memberRoleLabel(key) || '成员', order);
    }
    return groupedMembers(members, (member) => member.__presence || 'unknown', (key) => {
      if (key === 'online') return '在线';
      if (key === 'offline') return '离线';
      return '成员';
    }, ['online', 'unknown', 'offline']);
  }

  function memberRoleGroupKey(role) {
    const key = clean(role).toLowerCase();
    if (key === 'developer' || key === 'maintainer') return 'editor';
    if (key === 'viewer') return 'observer';
    return key || 'member';
  }

  function groupedMembers(members, keyOf, labelOf, order) {
    const buckets = new Map();
    members.forEach((member) => {
      const key = keyOf(member);
      if (!buckets.has(key)) buckets.set(key, []);
      buckets.get(key).push(member);
    });
    return Array.from(buckets.keys())
      .sort((left, right) => {
        const leftIndex = order.indexOf(left);
        const rightIndex = order.indexOf(right);
        if (leftIndex !== -1 || rightIndex !== -1) {
          return (leftIndex === -1 ? 99 : leftIndex) - (rightIndex === -1 ? 99 : rightIndex);
        }
        return String(labelOf(left)).localeCompare(String(labelOf(right)));
      })
      .map((key) => ({
        label: labelOf(key),
        members: buckets.get(key).slice().sort((left, right) => {
          const presenceDelta = (right.__presence === 'online' ? 1 : 0) - (left.__presence === 'online' ? 1 : 0);
          if (presenceDelta) return presenceDelta;
          return left.__name.localeCompare(right.__name);
        })
      }));
  }

  function senderIdOf(message) {
    return clean(message.sender_user_id || message.senderUserId || message.sender_id || message.senderId || message.user_id || message.userId);
  }

  function messageOutgoingState(message) {
    const value = message && (message.outgoing ?? message.is_outgoing ?? message.isOutgoing);
    if (value === true || value === 1) return true;
    if (value === false || value === 0) return false;
    const text = clean(value).toLowerCase();
    if (['true', '1', 'yes', 'y'].includes(text)) return true;
    if (['false', '0', 'no', 'n'].includes(text)) return false;
    return null;
  }

  function messageSenderIdentityMatchesUser(message) {
    if (!state.user) return false;
    const sender = [
      message.sender_account, message.senderAccount, message.user_account, message.userAccount,
      message.sender_name, message.senderName
    ].map(clean).filter(Boolean);
    if (!sender.length) return false;
    const own = [
      state.user.account, state.user.phone, state.user.email, state.user.nickname,
      state.user.name, userName(state.user)
    ].map(clean).filter(Boolean);
    return sender.some((left) => own.some((right) => sameId(left, right)));
  }

  function isOwnMessage(message) {
    const senderId = senderIdOf(message);
    const outgoing = messageOutgoingState(message);
    if (outgoing === true) return true;
    if (senderId && sameId(senderId, SOCIAL_AI_USER_ID)) return false;
    if (state.user && senderId && sameId(senderId, state.user.id)) return true;
    if (outgoing === false) return false;
    return messageSenderIdentityMatchesUser(message);
  }

  function avatarForMessage(message, scope) {
    const direct = avatarUrlOf(message);
    if (direct) return direct;
    const senderId = senderIdOf(message);
    if (isOwnMessage(message)) return avatarUrlOf(state.user);
    if (scope === 'ai') return avatarUrlOf(socialAiFriend());
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

  function emptyMessagesHtml(scope) {
    if (scope === 'project' && state.activeChannelKind === 'ai_development') {
      return '<div class="empty-state"><strong>从这里开始做应用</strong><p>直接输入你想做的 App，或告诉一龙AI要修改什么功能。</p></div>';
    }
    if (scope === 'project' && state.activeChannelKind === 'builds') {
      return '<div class="empty-state"><strong>做完后在这里生成安装包</strong><p>安装包生成后，入口会出现在「安装使用」。</p></div>';
    }
    return '<div class="empty-state"><strong>还没有消息</strong><p>从下方输入框发送第一条消息。</p></div>';
  }

  function renderMessages(messages, scope) {
    setNodeMode(false);
    const agentRunsHtml = scope === 'project' && agentRuns
      ? agentRuns.renderSection(messages, scope)
      : '';
    if (!messages.length && !agentRunsHtml) {
      clearDevTaskRefresh();
      els.messageList.innerHTML = emptyMessagesHtml(scope);
      return;
    }
    const devTaskContext = scope === 'project' && devTasks
      ? devTasks.buildContext(messages, devTaskSnapshots ? devTaskSnapshots.contextExtras() : null)
      : null;
    const messageRows = messages.length ? messages.map((message) => {
      const own = isOwnMessage(message);
      const role = clean(message.role || message.kind || message.message_kind);
      const fallbackName = role.startsWith('ai_')
        ? '一龙开发Agent'
        : (scope === 'ai' ? '一龙AI' : (scope === 'project' ? '项目成员' : '好友'));
      const name = clean(message.sender_name || message.user_account || message.sender || message.author_name || message.from_name) ||
        (own ? userName(state.user) : fallbackName);
      const tone = role.includes('assistant') || role.includes('ai') ? 'ai' : (role.includes('task') ? 'task' : '');
      const devTaskHtml = devTaskContext ? devTasks.renderMessage(message, devTaskContext) : '';
      const contentHtml = devTaskHtml || renderMessageContent(message, {
        className: tone,
        markdown: !!tone,
        copy: !!tone
      });
      return `<article class="message-row ${own ? 'own' : 'other'}">
        ${avatarElement('div', 'message-avatar', avatarForMessage(message, scope), name, '员')}
        <div class="message-body">
          <div class="message-meta"><strong>${escapeHtml(name)}</strong><span>${escapeHtml(formatTime(message.created_at || message.createdAt))}</span></div>
          ${contentHtml}
        </div>
      </article>`;
    }).join('') : emptyMessagesHtml(scope);
    els.messageList.innerHTML = `${messageRows}${agentRunsHtml}`;
    els.messageList.querySelectorAll('.project-share-action').forEach((button) => {
      button.addEventListener('click', () => handleProjectShareAction(button));
    });
    if (devTasks) devTasks.bindActions(els.messageList);
    if (agentRuns) agentRuns.bindActions(els.messageList, messages, scope);
    if (markdown.bindCopyButtons) markdown.bindCopyButtons(els.messageList);
    scheduleDevTaskRefresh(messages, scope, devTaskContext);
    if (agentRuns) agentRuns.schedule(messages, scope);
    els.messageList.scrollTop = els.messageList.scrollHeight;
  }

  function clearDevTaskRefresh() {
    if (devTaskRefreshTimer) {
      clearTimeout(devTaskRefreshTimer);
      devTaskRefreshTimer = 0;
    }
    if (devTaskSnapshots) devTaskSnapshots.clear();
    if (agentRuns) agentRuns.clear();
  }

  function scheduleDevTaskRefresh(messages, scope, devTaskContext) {
    clearDevTaskRefresh();
    if (scope !== 'project' || state.activeChannelKind !== 'ai_development' || !devTasks) return;
    if (!devTasks.hasOpenTasks(messages, devTaskContext)) return;
    if (devTaskSnapshots && devTaskSnapshots.schedule(messages, scope, devTaskContext)) return;
    const projectId = state.activeProjectId;
    const channelId = state.activeChannelId;
    devTaskRefreshTimer = setTimeout(() => {
      if (state.activeKind !== 'project') return;
      if (!sameId(state.activeProjectId, projectId) || !sameId(state.activeChannelId, channelId)) return;
      refreshActiveProjectChannel().catch(showError);
    }, 4500);
  }

  async function refreshActiveProjectChannel() {
    if (state.activeKind !== 'project' || !state.activeProjectId || !state.activeChannelId) return;
    await selectProjectChannel(state.activeChannelId);
  }

  async function cancelProjectAiTask(taskId) {
    const cleanTaskId = clean(taskId);
    if (!cleanTaskId || state.activeKind !== 'project' || !state.activeProjectId || !state.activeChannelId) return;
    await api(`/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/ai-tasks/${encodeURIComponent(cleanTaskId)}/cancel`, {
      method: 'POST'
    });
    await refreshActiveProjectChannel();
  }

  async function approveProjectTool(taskId, approvalId, decision) {
    const cleanTaskId = clean(taskId);
    const cleanApprovalId = clean(approvalId);
    const cleanDecision = clean(decision);
    if (!cleanTaskId || !cleanApprovalId || !cleanDecision || state.activeKind !== 'project' || !state.activeProjectId || !state.activeChannelId) return;
    await api(`/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/ai-tasks/${encodeURIComponent(cleanTaskId)}/tool-approvals/${encodeURIComponent(cleanApprovalId)}/decision`, {
      method: 'POST',
      body: JSON.stringify({ decision: cleanDecision })
    });
    await refreshActiveProjectChannel();
  }

  function draftProjectAiContinuation(content) {
    const draft = clean(content);
    if (!draft || !els.input) return;
    els.input.value = draft;
    els.input.dispatchEvent(new Event('input', { bubbles: true }));
    els.input.focus();
    try {
      els.input.selectionStart = els.input.selectionEnd = els.input.value.length;
    } catch (_) {}
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
    let submittingDevTask = false;
    try {
      if (state.activeKind === 'doctor') {
        await doctor.sendComposerMessage(content);
        els.input.value = '';
        resetComposerInputHeight();
      } else if (state.activeKind === 'ai') {
        const conversationId = state.activeAiConversationId || newAiConversationId();
        const conversationTitle = state.activeAiConversationTitle || aiTitleFromContent(content);
        state.activeAiConversationId = conversationId;
        state.activeAiConversationTitle = conversationTitle;
        const body = {
          scope: 'chat_memory',
          conversation_id: conversationId,
          conversation_title: conversationTitle,
          messages: [{ role: 'user', content }]
        };
        const agent = models.selectedAgentForRequest();
        if (agent) body.agent = agent;
        await api('/api/llm/chat', { method: 'POST', body: JSON.stringify(body) });
        els.input.value = '';
        resetComposerInputHeight();
        await loadAiConversations();
        await selectAiAssistant(true, conversationId);
      } else if (state.activeKind === 'friends' && state.activePeer) {
        const path = state.activePeer.kind === 'group'
          ? `/api/me/groups/${encodeURIComponent(state.activePeer.id)}/messages`
          : `/api/me/friends/${encodeURIComponent(state.activePeer.id)}/messages`;
        await api(path, { method: 'POST', body: JSON.stringify({ content }) });
        els.input.value = '';
        resetComposerInputHeight();
        await selectPeer(state.activePeer.kind, state.activePeer.id);
      } else if (state.activeKind === 'project-conversation' && state.activeProjectId && state.activeConversationId) {
        const project = projectById(state.activeProjectId);
        const conversation = cachedProjectConversation(state.activeProjectId, state.activeConversationId);
        const body = {
          message: content,
          conversation_id: state.activeConversationId
        };
        if (conversation && isDraftProjectConversation(state.activeProjectId, state.activeConversationId)) {
          body.conversation_title = compactText(content, 28);
        }
        const agent = models.selectedAgentForRequest();
        if (agent) body.agent = agent;
        els.input.value = '';
        resetComposerInputHeight();
        els.messageList.innerHTML = `<div class="empty-state">
          <strong>一龙AI正在处理</strong>
          <p>这条对话会保存在「${escapeHtml(project ? titleOf(project) : '当前项目')}」下面。</p>
        </div>`;
        await api(`/api/projects/${encodeURIComponent(state.activeProjectId)}/chat`, {
          method: 'POST',
          body: JSON.stringify(body)
        });
        await loadAiProjectConversations(project, true);
        await selectProjectConversation(state.activeProjectId, state.activeConversationId);
      } else if (state.activeKind === 'project' && state.activeProjectId && state.activeChannelId) {
        const shouldUseAiTask = useAiTask || state.activeChannelKind === 'ai_development';
        const path = shouldUseAiTask
          ? `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/ai-tasks`
          : `/api/projects/${encodeURIComponent(state.activeProjectId)}/channels/${encodeURIComponent(state.activeChannelId)}/messages`;
        const body = { content };
        if (shouldUseAiTask) {
          submittingDevTask = true;
          if (devComposer) devComposer.setBusy(true);
          const agent = models.selectedAgentForRequest();
          if (agent) body.agent = agent;
          const runtimeRoute = devComposer && devComposer.selectedRouteForRequest
            ? clean(devComposer.selectedRouteForRequest())
            : '';
          if (runtimeRoute) body.runtimeRoute = runtimeRoute;
        }
        await api(path, { method: 'POST', body: JSON.stringify(body) });
        els.input.value = '';
        resetComposerInputHeight();
        await selectProjectChannel(state.activeChannelId);
      }
    } catch (error) {
      showError(error);
    } finally {
      if (submittingDevTask && devComposer) devComposer.setBusy(false);
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
    if (els.settingsBackdrop) {
      els.settingsBackdrop.classList.toggle('is-workbench-flow', selected === 'workbench');
    }
    if (selected === 'account') {
      els.settingsAccountPanel.classList.add('active');
      $('settingsTitle').textContent = '账户';
      els.settingsSubtitle.textContent = '账号信息、登录状态和安全设置';
    } else if (selected === 'workbench') {
      els.settingsWorkbenchPanel.classList.add('active');
      $('settingsTitle').textContent = '本机开发设置';
      els.settingsSubtitle.textContent = '连接这台电脑、绑定账号并选择项目目录';
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

  function openSettings(section, options) {
    const targetSection = section || 'workbench';
    const autoPickAndRegister = !!(options && options.autoPickAndRegister);
    const workbenchProjectId = clean(options && options.projectId);
    setAccountMenu(false);
    renderUser();
    setSettingsSection(targetSection);
    els.settingsBackdrop.hidden = false;
    setSettingsResult('');
    if (targetSection === 'workbench') {
      state.workbenchRegistrationProjectId = workbenchProjectId;
      if (els.settingsRuntimePermission) {
        els.settingsRuntimePermission.value = 'project_write';
        syncSettingsRuntimePermissionHint();
      }
      applyWorkbenchProjectContext();
      updateWorkbenchOnboarding();
      refreshClientMaintenance(false);
    } else {
      state.workbenchRegistrationProjectId = '';
    }
    setTimeout(() => {
      if (targetSection !== 'workbench') return;
      if (autoPickAndRegister) {
        chooseLocalProjectFolder({ autoRegister: true });
        return;
      }
      els.chooseProjectFolderBtn.focus();
    }, 0);
  }

  function applyWorkbenchProjectContext() {
    const project = state.workbenchRegistrationProjectId
      ? projectById(state.workbenchRegistrationProjectId)
      : null;
    if (!project) return;
    if (els.settingsProjectName) els.settingsProjectName.value = titleOf(project);
    if (els.settingsProjectDesc && !clean(els.settingsProjectDesc.value)) {
      els.settingsProjectDesc.value = clean(project.description || project.project_description || project.projectDescription);
    }
    if (els.settingsProjectRepo && !clean(els.settingsProjectRepo.value)) {
      els.settingsProjectRepo.value = clean(project.repo_url || project.repoUrl);
    }
    if (els.settingsProjectBranch && !clean(els.settingsProjectBranch.value)) {
      els.settingsProjectBranch.value = clean(project.branch);
    }
  }

  function closeSettings() {
    cancelLocalProjectFolderPick();
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
      if (!button.disabled) button.dataset.label = button.textContent;
      button.disabled = true;
      button.textContent = label || '处理中…';
    } else {
      button.disabled = false;
      button.textContent = button.dataset.label || button.textContent;
      delete button.dataset.label;
    }
  }

  function setProjectFolderPickBusy(busy) {
    const button = els.chooseProjectFolderBtn;
    if (!button) return;
    if (busy) {
      if (!button.dataset.label) button.dataset.label = button.textContent;
      button.disabled = false;
      button.textContent = '取消等待';
      button.classList.add('is-cancel-action');
      return;
    }
    button.disabled = false;
    button.textContent = button.dataset.label || button.textContent;
    delete button.dataset.label;
    button.classList.remove('is-cancel-action');
    updateWorkbenchOnboarding();
  }

  function revealManualProjectPathInput(focusInput) {
    const details = document.querySelector('.settings-troubleshoot-details');
    if (details) details.open = true;
    if (focusInput && els.settingsProjectPath) {
      els.settingsProjectPath.scrollIntoView({ block: 'center', behavior: 'smooth' });
      window.setTimeout(() => els.settingsProjectPath.focus(), 120);
    }
  }

  function cancelLocalProjectFolderPick() {
    if (activeProjectFolderPickController) activeProjectFolderPickController.abort();
  }

  function clearProjectFolderPickHint() {
    if (!projectFolderPickHintTimer) return;
    window.clearTimeout(projectFolderPickHintTimer);
    projectFolderPickHintTimer = 0;
  }

  function setSetupStepState(element, stateName) {
    if (!element) return;
    element.classList.remove('is-active', 'is-done');
    if (stateName) element.classList.add(`is-${stateName}`);
  }

  function updateWorkbenchOnboarding() {
    if (!els.settingsNodeStatusCard) return;
    const connected = !!state.clientMaintenance;
    const path = clean(els.settingsProjectPath && els.settingsProjectPath.value);
    const info = state.localProjectInfo || null;
    const blocked = !!(info && info.canRegister === false);
    const launchAttempted = !!state.localNodeLaunchAttempted && !connected;
    let title = launchAttempted ? '还不能读取电脑文件夹' : '需要连接这台电脑';
    let detail = launchAttempted
      ? '可能还没安装一龙 Win 端，或浏览器没有成功打开本机助手。'
      : '网页不能直接读取本机文件夹或运行 Git，需要一龙 Win 端帮你选择目录。';
    let actionLabel = launchAttempted ? '下载 / 安装一龙 Win 端' : '连接这台电脑';

    if (connected && !path) {
      state.localNodeLaunchAttempted = false;
      title = '已连接这台电脑';
      detail = '现在可以选择电脑里的代码文件夹。';
      actionLabel = '选择代码文件夹';
    } else if (connected && blocked) {
      state.localNodeLaunchAttempted = false;
      title = '还差一点项目信息';
      detail = '打开“排查 / 高级设置”，补齐缺少字段后再注册。';
      actionLabel = '重新选择代码文件夹';
    } else if (connected && path) {
      state.localNodeLaunchAttempted = false;
      title = '项目目录已选好';
      detail = '正在加入项目列表，完成后会自动打开项目。';
      actionLabel = '重新选择代码文件夹';
    }

    if (els.settingsNodeStatusTitle) els.settingsNodeStatusTitle.textContent = title;
    if (els.settingsNodeStatusDetail) els.settingsNodeStatusDetail.textContent = detail;
    els.settingsNodeStatusCard.classList.toggle('is-ready', connected && !!path && !blocked);
    els.settingsNodeStatusCard.classList.toggle('is-pending', !connected && !launchAttempted);
    els.settingsNodeStatusCard.classList.toggle('is-help', launchAttempted);
    els.settingsNodeStatusCard.classList.toggle('is-warning', connected && blocked);
    if (els.chooseProjectFolderBtn && !els.chooseProjectFolderBtn.dataset.label) {
      els.chooseProjectFolderBtn.textContent = actionLabel;
    }

    setSetupStepState(els.settingsStepNode, connected ? 'done' : 'active');
    setSetupStepState(els.settingsStepFolder, path ? 'done' : (connected ? 'active' : ''));
    setSetupStepState(els.settingsStepRegister, connected && path && !blocked ? 'active' : '');
  }

  function applyClientMaintenanceStatus(data) {
    state.clientMaintenance = data || null;
    if (data) state.localNodeLaunchAttempted = false;
    if (!els.settingsClientStatus || !els.settingsClientPaths) {
      updateWorkbenchOnboarding();
      return;
    }
    if (!data) {
      els.settingsClientStatus.textContent = '尚未读取';
      els.settingsClientPaths.textContent = '任务记录、配置和安装目录会显示在这里。';
      renderClientMaintenanceActions(null);
      if (els.settingsCliBridgeStatus) els.settingsCliBridgeStatus.textContent = '读取本机助手后显示会话连续性能力。';
      updateWorkbenchOnboarding();
      return;
    }
    const version = clean(data.version) || '--';
    const install = data.install || {};
    const product = data.product_status || install.product_status || {};
    const overview = data.maintenance_overview || {};
    const startMenu = data.start_menu || install.start_menu || {};
    const installedSha = clean(data.installed_git_sha || install.installed_git_sha);
    const packageVersion = clean(data.installed_package_version || install.installed_package_version);
    const layoutStatus = clean(data.layout_status || install.layout_status);
    const startMenuLine = clientStartMenuLine(product, startMenu);
    const recommendedLine = clientRecommendedActionsLine(product);
    const primaryMaintenanceLine = clientPrimaryMaintenanceActionLine(data.primary_maintenance_action);
    const overviewTitle = clean(overview.title);
    const overviewDetail = clean(overview.detail);
    const installed = data.supported === false
      ? '当前平台不支持 Win 客户端维护'
      : data.installed
        ? '已安装'
        : '未检测到完整安装';
    const running = data.running_from_install_dir ? '安装目录运行中' : '外部启动或未确认';
    const packageLine = installedSha
      ? `包 ${shortHash(installedSha)}${packageVersion ? ` / ${packageVersion}` : ''}`
      : '未读取包版本';
    const updateLine = clientUpdateLine(data, state.clientPackageLatest);
    const recentMaintenance = clientMaintenanceEventsLine(data.maintenance_recent_events);
    const overviewPrefix = overviewTitle ? `${overviewTitle} · ` : '';
    els.settingsClientStatus.textContent = `${overviewPrefix}v${version} · ${installed} · ${running} · ${packageLine} · ${updateLine} · ${clientLayoutLabel(layoutStatus)}`;
    const paths = [
      overviewDetail && `维护说明 ${overviewDetail}`,
      clean(data.install_dir) && `安装 ${clean(data.install_dir)}`,
      clean(data.logs_dir) && `运行日志 ${clean(data.logs_dir)}`,
      clean(data.launcher_logs_dir) && `启动器日志 ${clean(data.launcher_logs_dir)}`,
      clean(data.task_journal_dir) && `任务记录 ${clean(data.task_journal_dir)}`,
      clean(data.config_dir) && `配置 ${clean(data.config_dir)}`,
      recentMaintenance && `最近维护 ${recentMaintenance}`,
      primaryMaintenanceLine && `首要建议 ${primaryMaintenanceLine}`,
      recommendedLine && `建议 ${recommendedLine}`,
      startMenuLine && `开始菜单 ${startMenuLine}`
    ].filter(Boolean);
    els.settingsClientPaths.textContent = paths.join(' · ') || '未读取到本机维护路径';
    if (els.settingsCliBridgeStatus) {
      els.settingsCliBridgeStatus.textContent = cliSessionBridgeLine(data.cli_session_bridge);
    }
    renderClientMaintenanceActions(data.maintenance_actions);
    updateWorkbenchOnboarding();
  }

  function renderClientMaintenanceActions(actions) {
    if (!els.settingsClientActions) return;
    if (clientMaintenanceActions) {
      clientMaintenanceActions.render(els.settingsClientActions, actions);
    } else {
      els.settingsClientActions.textContent = '刷新本机助手后显示每个操作是否可用。';
    }
  }

  async function refreshLatestClientPackageVersion() {
    try {
      const resp = await fetch('/api/node-agent/version', { cache: 'no-store' });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      state.clientPackageLatest = await resp.json();
    } catch (error) {
      state.clientPackageLatest = { error: clean(error && (error.message || error)) || 'version_unavailable' };
    }
    if (state.clientMaintenance) applyClientMaintenanceStatus(state.clientMaintenance);
  }

  function clientUpdateLine(local, latest) {
    const install = (local && local.install) || {};
    const installedSha = clean(local && (local.installed_git_sha || install.installed_git_sha));
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

  function clientMaintenanceEventsLine(events) {
    if (!Array.isArray(events) || !events.length) return '';
    const latest = events[0] || {};
    const action = clientMaintenanceActionLabel(clean(latest.action));
    const ok = latest.ok === false ? '失败' : '成功';
    const detail = clean(latest.detail);
    return [action, ok, detail].filter(Boolean).join(' · ');
  }

  function clientMaintenanceActionLabel(action) {
    if (action === 'open_target') return '打开维护目录';
    if (action === 'update') return '检查更新';
    if (action === 'uninstall') return '卸载';
    if (action === 'export_diagnostics') return '导出诊断';
    return action || '维护动作';
  }

  function shortHash(value) {
    const text = clean(value);
    return text.length > 12 ? text.slice(0, 8) : text;
  }

  function formatBytes(value) {
    const bytes = Number(value || 0);
    if (!bytes) return '';
    if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
    if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
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
    if (folder) return `${folder}${parts.length ? '：' + parts.join(' · ') : ''}`;
    return parts.join(' · ');
  }

  function clientRecommendedActionsLine(product) {
    const actions = Array.isArray(product && product.recommended_actions)
      ? product.recommended_actions.map(clean).filter(Boolean)
      : [];
    return actions.slice(0, 3).join(' / ');
  }

  function clientPrimaryMaintenanceActionLine(action) {
    if (!action || typeof action !== 'object') return '';
    const label = clean(action.label);
    const reason = clean(action.recommendation);
    const enabled = action.enabled === false ? '当前不可用' : '可点击';
    return [label, enabled, reason].filter(Boolean).join(' · ');
  }

  function clientStartMenuStatusLabel(status) {
    const value = clean(status).toLowerCase();
    if (!value || value === 'unknown') return '';
    if (value === 'clean') return '入口完整';
    if (value === 'missing') return '开始菜单文件夹缺失';
    if (value === 'incomplete') return '维护入口不完整';
    return value;
  }

  function cliSessionBridgeLine(bridge) {
    if (!bridge) return '本机助手未返回 CLI 会话桥接状态。';
    const summary = clean(bridge.display_summary || bridge.summary);
    const actions = Array.isArray(bridge.recommended_next_actions)
      ? bridge.recommended_next_actions.map(clean).filter(Boolean).slice(0, 2).join(' / ')
      : '';
    if (summary && actions) return `${summary} 下一步：${actions}`;
    if (summary) return summary;
    const modes = Array.isArray(bridge.continuity_modes)
      ? bridge.continuity_modes.map(clean).filter(Boolean).slice(0, 3).join(' / ')
      : '';
    const tty = bridge.tty_takeover_supported ? '支持原 TTY 接管' : '不接管原 TTY';
    return modes ? `${tty} · ${modes}` : `${tty} · 使用新 CLI 子进程桥接会话`;
  }

  async function refreshClientMaintenance(showResult) {
    setSettingsBusy(els.refreshClientMaintenanceBtn, true, '刷新中…');
    try {
      const data = await localNodeApi('/api/client-maintenance');
      applyClientMaintenanceStatus(data);
      refreshLatestClientPackageVersion();
      if (showResult) setSettingsResult('客户端维护状态已刷新。');
    } catch (error) {
      applyClientMaintenanceStatus(null);
      if (els.settingsClientStatus) els.settingsClientStatus.textContent = '无法连接本机助手';
      if (els.settingsClientPaths) els.settingsClientPaths.textContent = clean(error.message || error);
      if (showResult) {
        state.localNodeLaunchAttempted = true;
        updateWorkbenchOnboarding();
        setSettingsResult('还不能读取电脑文件夹。请点击绿色按钮打开下载和安装页面。', 'note');
      }
    } finally {
      setSettingsBusy(els.refreshClientMaintenanceBtn, false);
    }
  }

  function openNodeSetupFromSettings() {
    closeSettings();
    node.selectNode();
  }

  function startLocalNodeFromSettings() {
    setSettingsResult('');
    state.localNodeLaunchAttempted = true;
    if (els.settingsNodeStatusTitle) els.settingsNodeStatusTitle.textContent = '正在连接这台电脑';
    if (els.settingsNodeStatusDetail) {
      els.settingsNodeStatusDetail.textContent = '如果浏览器弹出确认框，请选择允许；连接成功后会自动进入文件夹选择。';
    }
    launchClientProtocol(CLIENT_PROTOCOL_TARGETS.open);
    setSettingsBusy(els.chooseProjectFolderBtn, true, '等待启动…');
    window.setTimeout(() => refreshClientMaintenance(false), 1200);
    window.setTimeout(() => refreshClientMaintenance(false), 3000);
    window.setTimeout(() => {
      setSettingsBusy(els.chooseProjectFolderBtn, false);
      if (!state.clientMaintenance) {
        setSettingsResult('还不能读取电脑文件夹。请点击绿色按钮打开下载和安装页面。', 'note');
      }
      updateWorkbenchOnboarding();
    }, 3600);
  }

  function clientProtocolUrlForTarget(target) {
    return CLIENT_PROTOCOL_TARGETS[String(target || '').trim()] || '';
  }

  function launchClientProtocol(url) {
    try {
      const frame = document.createElement('iframe');
      frame.style.display = 'none';
      frame.setAttribute('aria-hidden', 'true');
      frame.src = url;
      document.body.appendChild(frame);
      window.setTimeout(() => frame.remove(), 2000);
    } catch (_) {
      window.open(url, '_blank', 'noopener');
    }
  }

  function applyLocalProjectInfo(payload) {
    const project = (payload && payload.project) || {};
    const inspect = (payload && payload.inspect) || {};
    const registration = (payload && payload.registration) || {};
    const registerPayload = registration.register_payload || {};
    const nextAction = registration.next_action || {};
    const previousPath = clean(state.localProjectInfo && state.localProjectInfo.path);
    const path = clean(registerPayload.workspace_path || project.workspace_path || inspect.workspace_path);
    const pathChanged = path && path !== previousPath;
    const name = clean(registerPayload.name || project.name);
    const repo = clean(registerPayload.repo_url || project.repo_url || inspect.git_remote_origin);
    const branch = clean(registerPayload.branch || project.branch || inspect.git_branch);
    const desc = clean(registerPayload.description || project.description);
    const identitySource = clean(project.identity_source);
    const canRegister = registration.can_register !== false;
    const missingFields = Array.isArray(registration.missing_fields)
      ? registration.missing_fields.map(clean).filter(Boolean)
      : [];
    const warnings = Array.isArray(registration.warnings)
      ? registration.warnings.map(clean).filter(Boolean)
      : [];
    const autofillFields = Array.isArray(registration.autofill_fields)
      ? registration.autofill_fields.map(clean).filter(Boolean)
      : [];
    if (path) els.settingsProjectPath.value = path;
    if (name) els.settingsProjectName.value = name;
    els.settingsProjectRepo.value = repo;
    els.settingsProjectBranch.value = branch;
    if (desc && (pathChanged || !clean(els.settingsProjectDesc.value))) els.settingsProjectDesc.value = desc;
    const detectedFiles = Array.isArray(project.detected_files)
      ? project.detected_files.map(clean).filter(Boolean)
      : [];
    const payloadDevProfile = registerPayload.dev_profile || {};
    const devProfile = {
      project_type: clean(payloadDevProfile.project_type || project.project_type) || null,
      package_manager: clean(payloadDevProfile.package_manager || project.package_manager) || null,
      run_command: clean(payloadDevProfile.run_command || project.run_command) || null,
      test_command: clean(payloadDevProfile.test_command || project.test_command) || null,
      build_command: clean(payloadDevProfile.build_command || project.build_command) || null,
      detected_files: Array.isArray(payloadDevProfile.detected_files)
        ? payloadDevProfile.detected_files.map(clean).filter(Boolean)
        : detectedFiles,
      source: 'node_agent_project_picker'
    };
    const hasDevProfile = Boolean(
      devProfile.project_type || devProfile.package_manager || devProfile.run_command ||
      devProfile.test_command || devProfile.build_command || devProfile.detected_files.length
    );
    state.localProjectInfo = path
      ? { path, name, repo, branch, canRegister, missingFields, registerPayload, devProfile: hasDevProfile ? devProfile : null }
      : null;
    const git = inspect.is_git_worktree || project.is_git_worktree
      ? [branch || 'HEAD', clean(project.git_head || inspect.git_head), (project.has_uncommitted_changes || inspect.has_uncommitted_changes) ? '有未提交改动' : '干净']
        .filter(Boolean).join(' · ')
      : '未检测到 Git 工作区';
    const profile = [clean(project.project_type), clean(project.package_manager)]
      .filter(Boolean).join(' · ');
    const commands = [
      clean(project.run_command) && `运行 ${clean(project.run_command)}`,
      clean(project.test_command) && `测试 ${clean(project.test_command)}`,
      clean(project.build_command) && `构建 ${clean(project.build_command)}`
    ].filter(Boolean).join(' / ');
    const detected = (devProfile.detected_files || detectedFiles).slice(0, 4).join('、');
    const agentRuntime = (project && project.agent_runtime) || {};
    const agentRuntimeStatus = clean(agentRuntime.status);
    const agentRuntimeSummary = clean(agentRuntime.summary);
    const agentRuntimeTone = agentRuntimeStatus === 'current' ? 'ok' : (agentRuntimeSummary ? 'warning' : '');
    if (!path) {
      els.settingsProjectMeta.textContent = '尚未选择项目目录';
      updateWorkbenchOnboarding();
      return;
    }
    const summary = clean(registration.summary) || (canRegister ? '已读取目录信息，可以注册。' : '目录信息不足，暂不能注册。');
    const statusTone = !canRegister ? 'error' : warnings.length ? 'warning' : 'ok';
    const nextActionLine = [
      clean(nextAction.label),
      clean(nextAction.detail)
    ].filter(Boolean).join('：');
    const registerPreview = projectRegistrationPreviewLine();
    const rows = [
      registerPreview && ['将注册', registerPreview, canRegister ? 'ok' : 'warning', 'is-register-preview'],
      ['目录', path],
      ['状态', summary, statusTone],
      nextActionLine && ['下一步', nextActionLine, !canRegister ? 'error' : warnings.length ? 'warning' : 'ok'],
      ['Git', git],
      identitySource && ['来源', identitySource],
      profile && ['类型', profile],
      commands && ['命令', commands],
      agentRuntimeSummary && ['Agent Runtime', agentRuntimeSummary, agentRuntimeTone],
      detected && ['识别', `检测到 ${detected}`],
      autofillFields.length && ['自动', `已填写 ${autofillFields.slice(0, 8).join('、')}`],
      missingFields.length && ['缺少', missingFields.join('、'), 'error'],
      warnings.length && ['提醒', warnings.slice(0, 3).join('；'), 'warning']
    ].filter(Boolean);
    els.settingsProjectMeta.innerHTML = rows.map(([label, value, tone, extraClass]) => (
      `<div class="settings-project-meta-row ${tone ? `is-${escapeHtml(tone)}` : ''} ${extraClass ? escapeHtml(extraClass) : ''}"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`
    )).join('');
    updateWorkbenchOnboarding();
  }

  function projectRegistrationPreviewLine() {
    const payload = (state.localProjectInfo && state.localProjectInfo.registerPayload) || {};
    const name = clean(els.settingsProjectName && els.settingsProjectName.value);
    const path = clean(els.settingsProjectPath && els.settingsProjectPath.value);
    const repo = clean(els.settingsProjectRepo && els.settingsProjectRepo.value);
    const branch = clean(els.settingsProjectBranch && els.settingsProjectBranch.value);
    const mode = normalizeRuntimePermission(els.settingsRuntimePermission && els.settingsRuntimePermission.value);
    const modeLabel = runtimePermissionLabel(mode);
    const devProfile = (state.localProjectInfo && state.localProjectInfo.devProfile) || {};
    const commands = [
      clean(devProfile.run_command) && `运行 ${clean(devProfile.run_command)}`,
      clean(devProfile.test_command) && `测试 ${clean(devProfile.test_command)}`,
      clean(devProfile.build_command) && `构建 ${clean(devProfile.build_command)}`
    ].filter(Boolean);
    const gitLine = repo
      ? `Git ${repo}${branch ? ` @ ${branch}` : ''}`
      : (branch ? `分支 ${branch}` : '');
    return [
      (name || clean(payload.name)) && `项目 ${name || clean(payload.name)}`,
      gitLine || ((path || clean(payload.workspace_path)) && '本地目录'),
      `权限 ${modeLabel}`,
      commands.length && `命令 ${commands.slice(0, 2).join(' / ')}`
    ].filter(Boolean).join(' · ');
  }

  function refreshProjectRegistrationPreview() {
    if (!els.settingsProjectMeta) return;
    const preview = els.settingsProjectMeta.querySelector('.settings-project-meta-row.is-register-preview strong');
    if (preview) preview.textContent = projectRegistrationPreviewLine();
  }

  function markLocalProjectPathDirty() {
    const path = clean(els.settingsProjectPath.value);
    state.localProjectInfo = null;
    els.settingsProjectMeta.textContent = path
      ? '注册前会自动读取目录、Git 远端、当前分支和项目命令。'
      : '尚未选择项目目录';
    updateWorkbenchOnboarding();
  }

  function projectInfoMatchesPath(path) {
    return clean(state.localProjectInfo && state.localProjectInfo.path) === clean(path);
  }

  async function inspectProjectPath(path) {
    const data = await localNodeApi('/api/project-folder/inspect', {
      method: 'POST',
      body: JSON.stringify({ workspace_path: path })
    });
    applyLocalProjectInfo(data);
    return data;
  }

  async function ensureProjectInfoBeforeRegister(path) {
    if (projectInfoMatchesPath(path) && clean(els.settingsProjectName.value) && state.localProjectInfo.canRegister !== false) return;
    setSettingsResult('正在自动读取目录信息…');
    await inspectProjectPath(path);
    setSettingsResult('已自动读取目录、Git 远端、当前分支和项目命令，继续注册。');
  }

  function projectRegistrationSummary(data, fallback) {
    const registration = (data && data.registration) || {};
    return clean(registration.summary) || fallback;
  }

  function projectRegistrationResultKind(data) {
    const registration = (data && data.registration) || {};
    if (registration.can_register === false) return 'error';
    return Array.isArray(registration.warnings) && registration.warnings.length ? 'note' : 'ok';
  }

  function normalizeRuntimePermission(value) {
    const mode = clean(value);
    if (mode === 'danger_full_access') return 'danger_full_access';
    return mode === 'full_access' ? 'full_access' : 'project_write';
  }

  function runtimePermissionLabel(mode) {
    if (mode === 'danger_full_access') return '完整本机命令行';
    return mode === 'full_access' ? '完全访问' : '项目内读写';
  }

  function syncSettingsRuntimePermissionHint() {
    if (!els.settingsRuntimePermissionHint || !els.settingsRuntimePermission) return;
    const mode = normalizeRuntimePermission(els.settingsRuntimePermission.value);
    els.settingsRuntimePermissionHint.textContent = mode === 'danger_full_access'
      ? 'Route A/B/C 都允许 AI 使用完整本机命令行、绝对路径文件读写和 cmd/powershell 排障。'
      : (mode === 'full_access'
        ? 'Route A 的 Codex/Copilot 需要本机确认后才会绕过项目沙箱；Route B/C 仍保留项目路径和命令白名单，但 build/test 会执行项目代码。'
        : 'AI 只能读写当前项目目录，并运行开发相关命令。');
  }

  async function saveProjectRuntimePermission(project, requestedMode) {
    const projectId = clean(project && project.id);
    if (!projectId) return;
    const mode = normalizeRuntimePermission(requestedMode);
    const data = await api(`/api/projects/${encodeURIComponent(projectId)}/runtime-permission`, {
      method: 'PATCH',
      body: JSON.stringify({
        mode,
        confirmFullAccess: mode === 'full_access' || mode === 'danger_full_access',
        confirmDangerFullAccess: mode === 'danger_full_access'
      })
    });
    project.runtime_permission = data.mode || mode;
  }

  async function grantLocalProjectFullAccess(project, workspacePath) {
    const projectId = clean(project && project.id);
    if (!projectId) throw new Error('云端项目 ID 缺失，无法写入这台电脑的完全访问授权。');
    await localNodeApi('/api/full-access/grants', {
      method: 'POST',
      body: JSON.stringify({
        project_id: projectId,
        workspace_path: workspacePath,
        confirm_full_access: true
      })
    });
  }

  async function chooseLocalProjectFolder(options) {
    if (activeProjectFolderPickController) {
      cancelLocalProjectFolderPick();
      return;
    }
    if (!state.clientMaintenance) {
      if (state.localNodeLaunchAttempted) {
        openNodeSetupFromSettings();
        return;
      }
      startLocalNodeFromSettings();
      return;
    }
    const autoRegister = !!(options && options.autoRegister);
    const controller = new AbortController();
    activeProjectFolderPickController = controller;
    clearProjectFolderPickHint();
    setSettingsResult('正在打开本机文件夹选择器… 如果没有看到弹窗，请看任务栏或按 Alt+Tab；也可以点击“取消等待”后手动填写项目目录。', 'note');
    setProjectFolderPickBusy(true);
    projectFolderPickHintTimer = window.setTimeout(() => {
      if (activeProjectFolderPickController !== controller) return;
      revealManualProjectPathInput(false);
      setSettingsResult('还没有收到文件夹选择结果。我已展开手动路径输入框；如果系统弹窗在后台，可以先选目录，或者点击“取消等待”后直接填写路径。', 'note');
    }, 6000);
    try {
      const data = await localNodeApi('/api/project-folder/pick', { method: 'POST', signal: controller.signal });
      if (data.cancelled) {
        setSettingsResult('已取消选择。');
        return;
      }
      applyLocalProjectInfo(data);
      const registration = (data && data.registration) || {};
      if (autoRegister && registration.can_register !== false) {
        setSettingsResult(
          projectRegistrationSummary(data, '已读取项目目录、Git 远端和当前分支，正在注册…'),
          projectRegistrationResultKind(data)
        );
        await registerLocalProject({ fromAutoPick: true });
        return;
      }
      setSettingsResult(
        projectRegistrationSummary(data, '已读取项目目录、Git 远端和当前分支。'),
        projectRegistrationResultKind(data)
      );
    } catch (error) {
      if (error && error.name === 'AbortError') {
        revealManualProjectPathInput(true);
        setSettingsResult('已停止等待文件夹选择器。请在“项目目录”里填写本机路径，例如 D:\\my-app，然后点击“重新读取目录信息”。', 'note');
      } else {
        revealManualProjectPathInput(true);
        setSettingsResult(`${escapeHtml(error.message || error)}。你也可以手动填写项目目录后重试。`, 'error');
      }
    } finally {
      if (activeProjectFolderPickController === controller) activeProjectFolderPickController = null;
      clearProjectFolderPickHint();
      setProjectFolderPickBusy(false);
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
      const data = await inspectProjectPath(path);
      setSettingsResult(
        projectRegistrationSummary(data, '已读取项目目录、Git 远端和当前分支。'),
        projectRegistrationResultKind(data)
      );
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
    const registerPayload = (state.localProjectInfo && state.localProjectInfo.registerPayload) || {};
    const targetProjectId = clean(state.workbenchRegistrationProjectId);
    const targetProject = targetProjectId ? projectById(targetProjectId) : null;
    const path = clean(els.settingsProjectPath.value) || clean(registerPayload.workspace_path);
    if (!path) {
      setSettingsResult('请选择项目目录。', 'error');
      return;
    }
    setSettingsBusy(els.registerProjectBtn, true, '自动读取…');
    try {
      await ensureProjectInfoBeforeRegister(path);
    } catch (error) {
      setSettingsResult(escapeHtml(error.message || error), 'error');
      setSettingsBusy(els.registerProjectBtn, false);
      return;
    }
    const name = clean(els.settingsProjectName.value) || clean(registerPayload.name) || (targetProject ? titleOf(targetProject) : '');
    if (!name) {
      setSettingsResult('目录已读取，但没有识别到项目名称，请手动填写。', 'error');
      setSettingsBusy(els.registerProjectBtn, false);
      return;
    }
    if (state.localProjectInfo && state.localProjectInfo.canRegister === false) {
      const missing = (state.localProjectInfo.missingFields || []).join('、') || '必要信息';
      setSettingsResult(`目录信息不足，缺少：${escapeHtml(missing)}。`, 'error');
      setSettingsBusy(els.registerProjectBtn, false);
      return;
    }
    const devProfile = (state.localProjectInfo && (state.localProjectInfo.devProfile || registerPayload.dev_profile)) || null;
    const runtimeMode = normalizeRuntimePermission(els.settingsRuntimePermission && els.settingsRuntimePermission.value);
    if (runtimeMode === 'full_access' || runtimeMode === 'danger_full_access') {
      const dangerText = runtimeMode === 'danger_full_access'
        ? 'Route A/B/C 都可能运行任意 cmd/powershell 命令，并读写项目目录外的本机文件和系统设置；这次确认会写入这台电脑的本机授权记录。'
        : 'Route A 的 Codex/Copilot 可能读取或修改项目目录外的本机文件和系统设置；这次确认会写入这台电脑的本机授权记录。';
      const ok = window.confirm(`确认给项目「${name}」开启${runtimePermissionLabel(runtimeMode)}？${dangerText}`);
      if (!ok) {
        setSettingsResult('已取消授权，项目尚未注册。');
        setSettingsBusy(els.registerProjectBtn, false);
        return;
      }
    }
    setSettingsBusy(els.registerProjectBtn, true, '注册中…');
    try {
      await ensureLocalNodeLogin();
      const data = await localNodeApi('/api/register-project', {
        method: 'POST',
        body: JSON.stringify({
          project_id: targetProjectId || null,
          name,
          workspace_path: path,
          description: clean(els.settingsProjectDesc.value) || clean(registerPayload.description) || null,
          repo_url: clean(els.settingsProjectRepo.value) || clean(registerPayload.repo_url) || null,
          branch: clean(els.settingsProjectBranch.value) || clean(registerPayload.branch) || null,
          dev_profile: devProfile
        })
      });
      const project = (data.cloud && data.cloud.project) || {};
      const reused = data.cloud && data.cloud.reused_existing;
      if (runtimeMode === 'full_access' || runtimeMode === 'danger_full_access') {
        await grantLocalProjectFullAccess(project, path);
      }
      await saveProjectRuntimePermission(project, runtimeMode);
      setSettingsResult(`${reused ? '已复用现有项目' : '注册成功'}：${escapeHtml(project.name || name)}${project.id ? ` · ${escapeHtml(project.id)}` : ''}${runtimeMode === 'project_write' ? '' : ` · ${runtimePermissionLabel(runtimeMode)}已授权`}`);
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
    if (state.activeKind === 'project-conversation' && state.activeProjectId && state.activeConversationId) {
      return selectProjectConversation(state.activeProjectId, state.activeConversationId);
    }
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
    clearDevTaskRefresh();
    setNodeMode(false);
    els.messageList.innerHTML = `<div class="empty-state"><strong>加载失败</strong><p>${escapeHtml(error.message || error)}</p></div>`;
  }

  window.addEventListener('elon:project-task-done', (event) => {
    const detail = event.detail || {};
    if (state.activeKind !== 'project' || !state.activeChannelId) return;
    if (!sameId(detail.projectId || detail.project_id, state.activeProjectId)) return;
    refreshActiveProjectChannel().catch(showError);
  });

  window.addEventListener('storage', () => {
    const latest = readToken();
    if (latest && latest !== state.token) {
      saveToken(latest);
      refreshActive();
    }
  });

  init().catch(showError);
})();
