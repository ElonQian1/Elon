(function () {
  const ROOT_ID = 'projectHomeRoot';
  const LONG_PRESS_MS = 520;

  const actionMenuState = {
    projectId: '',
    pressTimer: null,
    suppressNextOpen: false
  };
  let selectedSegment = 'personal';
  let renderedProjectsById = new Map();

  function bridge() {
    return window.ElonWebApp || {};
  }

  function escapeHtml(value) {
    const app = bridge();
    if (typeof app.escapeHtml === 'function') return app.escapeHtml(value);
    return String(value == null ? '' : value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function cleanText(value) {
    const text = String(value == null ? '' : value).trim();
    return text && text.toLowerCase() !== 'null' ? text : '';
  }

  function projects() {
    const app = bridge();
    return typeof app.getProjects === 'function' ? app.getProjects() : [];
  }

  function archive() {
    const app = bridge();
    return typeof app.getProjectArchive === 'function' ? app.getProjectArchive() : null;
  }

  function projectById(id) {
    return projects().find((item) => item && String(item.id) === String(id)) ||
      renderedProjectsById.get(String(id));
  }

  function formatTime(value) {
    const app = bridge();
    if (typeof app.formatTime === 'function') return app.formatTime(value);
    return value ? String(value).replace('T', ' ').slice(0, 16) : '时间';
  }

  function titleOf(project) {
    return cleanText(project.displayName) ||
      cleanText(project.display_name) ||
      cleanText(project.alias) ||
      cleanText(project.project_alias) ||
      cleanText(project.title) ||
      cleanText(project.name) ||
      '未命名项目';
  }

  function projectInitial(project) {
    return Array.from(String(titleOf(project)).trim())[0] || '项';
  }

  function iconUrlOf(project) {
    return [
      project.iconDataUrl,
      project.icon_data_url,
      project.iconUrl,
      project.icon_url,
      project.icon,
      project.avatar,
      project.logo
    ].find((value) => typeof value === 'string' && value.trim()) || '';
  }

  function renderProjectThumb(project, className) {
    const iconUrl = iconUrlOf(project);
    const label = escapeHtml(projectInitial(project));
    return `
      <span class="${className}" aria-hidden="true">
        ${label}
        ${iconUrl ? `<img src="${escapeHtml(iconUrl)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
  }

  function roleOf(project) {
    return String(project.role || '').trim().toLowerCase();
  }

  function sourceTypeOf(project) {
    return String(project.source_type || project.sourceType || '').trim().toLowerCase();
  }

  function systemKeyOf(project) {
    return String(project.system_key || project.systemKey || '').trim().toLowerCase();
  }

  function isSystemProject(project) {
    const sourceType = sourceTypeOf(project);
    return sourceType === 'agent_balloon' || sourceType === 'chat_memory' || !!systemKeyOf(project);
  }

  function boolOf(value) {
    return value === true || value === 1 || value === '1' || String(value || '').toLowerCase() === 'true';
  }

  function isJointProject(project) {
    if (isSystemProject(project)) return false;
    const role = roleOf(project);
    const memberCount = Number(project.member_count || project.memberCount || 0) || 0;
    return Boolean(
      project.isJointProject ||
      project.is_joint_project ||
      project.collaborationProjectId ||
      project.collaboration_project_id ||
      (role && role !== 'owner') ||
      boolOf(project.is_public || project.isPublic) ||
      memberCount > 1
    );
  }

  function stageOf(project) {
    const value = String(project.last_task_status || project.status || project.stage || '').trim().toLowerCase();
    if (value === 'running') return '运行中';
    if (value === 'done') return '交付完成';
    if (value === 'failed') return '需要处理';
    return project.last_task_status || project.status || project.stage || '待提交需求';
  }

  function workspaceStatusOf(project) {
    const status = project.workspace_status || project.workspaceStatus;
    if (!status || typeof status !== 'object') return '';
    const latest = String(status.latest_execution_status || status.latestExecutionStatus || '').trim().toLowerCase();
    if (latest === 'running') return '运行中';
    return status.health_label || status.healthLabel || '';
  }

  function conversationCount(project) {
    if (Array.isArray(project.conversations)) return project.conversations.length || 1;
    return project.conversation_count || project.conversationCount || project.chat_count || 1;
  }

  function projectTime(project) {
    return formatTime(project.updated_at || project.updatedAt || project.updated_at_ms) || '时间';
  }

  function currentDisplayName() {
    const app = bridge();
    const user = typeof app.getCurrentUser === 'function' ? app.getCurrentUser() : null;
    return (user && (user.nickname || user.account)) || '未登录';
  }

  function ownerOf(project) {
    if (isSystemProject(project)) return '系统';
    const owner = project.owner_account || project.ownerAccount || project.created_by_account || project.owner;
    if (owner) return owner;
    return isJointProject(project) ? '未知' : currentDisplayName();
  }

  function cardMemberCount(project) {
    const raw = project.member_count ?? project.memberCount ?? project.members;
    const count = Array.isArray(raw) ? raw.length : Number(raw);
    if (Number.isFinite(count) && count >= 0) return count;
    return isJointProject(project) ? 0 : 1;
  }

  function projectIntroOf(project) {
    const intro = String(project.project_description || project.projectDescription || project.description || project.subtitle || '').trim();
    if (intro) return intro;
    if (isSystemProject(project)) {
      const key = systemKeyOf(project);
      if (key === 'phone_control' || sourceTypeOf(project) === 'agent_balloon') return '保存悬浮球手机控制的会话记录、自动化脚本和专属记忆。';
      if (key === 'chat_memory' || sourceTypeOf(project) === 'chat_memory') return '保存普通聊天的会话记录、用户偏好和长期记忆。';
    }
    return '暂无简介';
  }

  function projectMeta(project) {
    const kind = isJointProject(project) ? '联合项目' : '个人独立';
    return `${kind} · ${conversationCount(project)}个会话 · ${workspaceStatusOf(project) || stageOf(project)}`;
  }

  function projectSections() {
    const all = projects();
    const archiveData = archive();
    const serverPersonal = Array.isArray(archiveData && archiveData.personal_projects)
      ? archiveData.personal_projects
      : null;
    const system = Array.isArray(archiveData && archiveData.system_projects)
      ? archiveData.system_projects
      : all.filter(isSystemProject);
    const owned = Array.isArray(archiveData && archiveData.owned_projects)
      ? archiveData.owned_projects.filter((project) => !isSystemProject(project))
      : all.filter((project) => !isSystemProject(project) && !isJointProject(project));
    const personal = (serverPersonal || system.concat(owned))
      .slice()
      .sort((a, b) => Number(b.updated_at || b.updatedAt || 0) - Number(a.updated_at || a.updatedAt || 0));
    const joint = (Array.isArray(archiveData && archiveData.shared_projects)
      ? archiveData.shared_projects.filter((project) => !isSystemProject(project))
      : all.filter((project) => !isSystemProject(project) && isJointProject(project)))
      .slice()
      .sort((a, b) => Number(b.updated_at || b.updatedAt || 0) - Number(a.updated_at || a.updatedAt || 0));
    return { personal, joint };
  }

  function projectSectionItems() {
    const sections = projectSections();
    return selectedSegment === 'joint' ? sections.joint : sections.personal;
  }

  function renderCard(project) {
    const app = bridge();
    const active = typeof app.isCurrentProject === 'function' && app.isCurrentProject(project);
    return `
      <button class="project-home-card ${active ? 'active' : ''}" type="button" data-project-home-action="open" data-project-id="${escapeHtml(project.id)}" aria-label="打开项目 ${escapeHtml(titleOf(project))}">
        ${renderProjectThumb(project, 'project-home-thumb')}
        <span class="project-home-copy">
          <span class="project-home-name">${escapeHtml(titleOf(project))}</span>
          <span class="project-home-desc">${escapeHtml(projectIntroOf(project))}</span>
          <span class="project-home-meta-row">
            <span>创建者：${escapeHtml(ownerOf(project))}</span>
            <span>成员：${escapeHtml(cardMemberCount(project))}</span>
          </span>
        </span>
        <span class="project-home-chevron" aria-hidden="true">›</span>
      </button>
    `;
  }

  function renderEmptyCard() {
    return `
      <button class="project-home-empty-card" type="button" data-project-home-action="create" aria-label="新建项目">
        <span class="project-home-empty-plus">${selectedSegment === 'joint' ? '暂无联合项目' : '还没有项目，点击 + 创建'}</span>
      </button>
    `;
  }

  function renderActionMenu() {
    const project = actionMenuState.projectId ? projectById(actionMenuState.projectId) : null;
    if (!project) return '';
    return `
      <div class="project-home-action-mask" data-project-home-action="close-menu" role="dialog" aria-modal="true" aria-label="项目操作">
        <div class="project-home-action-panel" data-project-home-menu-panel>
          <div class="project-home-action-header">
            ${renderProjectThumb(project, 'project-home-action-thumb')}
            <span class="project-home-action-head-text">
              <span class="project-home-action-title">${escapeHtml(titleOf(project))}</span>
              <span class="project-home-action-status">${escapeHtml(projectMeta(project))}</span>
            </span>
          </div>
          <div class="project-home-action-list">
            ${projectActionRows(project).map(renderActionRow).join('')}
          </div>
        </div>
      </div>
    `;
  }

  function projectActionRows(project) {
    const role = roleOf(project);
    const isOwner = !role || role === 'owner';
    const rows = [
      { action: 'open', icon: '开', title: '打开项目空间', subtitle: '进入项目开发会话或协作空间' },
      { action: 'git', icon: 'G', title: 'Git 仓库', subtitle: '查看或配置项目远端' },
      { action: 'record', icon: '记', title: '项目记录', subtitle: '查看开发进度和最近状态' }
    ];
    if (isOwner && !isSystemProject(project)) {
      rows.push({ action: 'visibility', icon: '权', title: '协作权限 / 商城公开', subtitle: '管理加入方式和可见范围' });
    }
    if (isOwner && projects().length > 1 && project.id !== 'elon-self') {
      rows.push({ action: 'delete', icon: '删', title: '删除项目', subtitle: '从服务器和列表移除', danger: true });
    } else if (!isOwner) {
      rows.push({ action: 'leave', icon: '退', title: '退出项目', subtitle: '移出我的项目列表', danger: true });
    }
    return rows;
  }

  function renderActionRow(row) {
    return `
      <button class="project-home-action-row ${row.danger ? 'danger' : ''}" type="button" data-project-home-action="${escapeHtml(row.action)}" data-project-id="${escapeHtml(actionMenuState.projectId)}">
        <span class="project-home-action-icon" aria-hidden="true">${escapeHtml(row.icon)}</span>
        <span class="project-home-action-copy">
          <span class="project-home-action-name">${escapeHtml(row.title)}</span>
          <span class="project-home-action-subtitle">${escapeHtml(row.subtitle)}</span>
        </span>
      </button>
    `;
  }

  function render() {
    const root = document.getElementById(ROOT_ID);
    if (!root) return;
    attachEvents(root);
    const items = projectSectionItems();
    renderedProjectsById = new Map(items.map((project) => [String(project.id), project]));
    root.innerHTML = `
      <div class="project-home-segments" role="tablist" aria-label="项目类型">
        <button class="project-home-segment ${selectedSegment === 'personal' ? 'active' : ''}" type="button" data-project-home-action="segment-personal">独立</button>
        <button class="project-home-segment ${selectedSegment === 'joint' ? 'active' : ''}" type="button" data-project-home-action="segment-joint">联合</button>
      </div>
      <div class="project-home-list">
        ${items.length ? items.map(renderCard).join('') : renderEmptyCard()}
      </div>
      ${renderActionMenu()}
    `;
  }

  function attachEvents(root) {
    if (root.dataset.projectHomeReady === 'true') return;
    root.dataset.projectHomeReady = 'true';
    root.addEventListener('click', handleAction);
    root.addEventListener('contextmenu', handleContextMenu);
    root.addEventListener('pointerdown', handlePointerDown);
    root.addEventListener('pointerup', clearLongPress);
    root.addEventListener('pointercancel', clearLongPress);
    root.addEventListener('pointerleave', clearLongPress);
    root.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && actionMenuState.projectId) {
        event.preventDefault();
        closeActionMenu();
        return;
      }
      if (event.key !== 'Enter' && event.key !== ' ') return;
      const actionEl = event.target.closest('[data-project-home-action]');
      if (!actionEl) return;
      event.preventDefault();
      runAction(actionEl);
    });
  }

  function handleAction(event) {
    const actionEl = event.target.closest('[data-project-home-action]');
    if (!actionEl) return;
    if (
      actionMenuState.suppressNextOpen &&
      actionEl.dataset.projectHomeAction === 'open' &&
      actionEl.classList.contains('project-home-card')
    ) {
      actionMenuState.suppressNextOpen = false;
      event.preventDefault();
      return;
    }
    if (
      actionEl.dataset.projectHomeAction === 'close-menu' &&
      event.target.closest('[data-project-home-menu-panel]')
    ) {
      return;
    }
    event.preventDefault();
    runAction(actionEl);
  }

  function handleContextMenu(event) {
    const card = event.target.closest('.project-home-card[data-project-id]');
    if (!card) return;
    event.preventDefault();
    clearLongPress();
    openActionMenu(card.dataset.projectId);
  }

  function handlePointerDown(event) {
    if (event.pointerType === 'mouse' && event.button !== 0) return;
    const card = event.target.closest('.project-home-card[data-project-id]');
    if (!card) return;
    clearLongPress();
    actionMenuState.pressTimer = window.setTimeout(() => {
      actionMenuState.pressTimer = null;
      actionMenuState.suppressNextOpen = true;
      openActionMenu(card.dataset.projectId);
    }, LONG_PRESS_MS);
  }

  function clearLongPress() {
    if (!actionMenuState.pressTimer) return;
    window.clearTimeout(actionMenuState.pressTimer);
    actionMenuState.pressTimer = null;
  }

  function openActionMenu(projectId) {
    if (!projectById(projectId)) return;
    actionMenuState.projectId = String(projectId);
    render();
  }

  function closeActionMenu() {
    actionMenuState.projectId = '';
    render();
  }

  function runAction(actionEl) {
    const action = actionEl.dataset.projectHomeAction;
    const app = bridge();
    if (action === 'close-menu') {
      closeActionMenu();
      return;
    }
    if (action === 'create') {
      if (typeof app.openNewProject === 'function') app.openNewProject();
      return;
    }
    if (action === 'segment-personal') {
      selectedSegment = 'personal';
      closeActionMenu();
      return;
    }
    if (action === 'segment-joint') {
      selectedSegment = 'joint';
      closeActionMenu();
      return;
    }
    const project = projectById(actionEl.dataset.projectId);
    if (!project) {
      closeActionMenu();
      return;
    }
    if (action === 'open') {
      if (actionMenuState.projectId) closeActionMenu();
      if (typeof app.openProject === 'function') app.openProject(project);
      return;
    }
    closeActionMenu();
    if (action === 'git' && typeof app.openProjectGit === 'function') app.openProjectGit(project);
    else if (action === 'record' && typeof app.openProjectProgress === 'function') app.openProjectProgress(project);
    else if (action === 'visibility' && typeof app.openProjectVisibility === 'function') app.openProjectVisibility(project);
    else if (action === 'delete' && typeof app.deleteProject === 'function') app.deleteProject(project);
    else if (action === 'leave' && typeof app.leaveProject === 'function') app.leaveProject(project);
  }

  window.ElonProjectHome = { render };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', render, { once: true });
  } else {
    render();
  }
})();
