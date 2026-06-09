(function () {
  const ROOT_ID = 'projectHomeRoot';
  const LONG_PRESS_MS = 520;
  const COLLAPSED_PROJECT_LIMIT = 4;

  const actionMenuState = {
    projectId: '',
    pressTimer: null,
    suppressNextOpen: false
  };
  const sectionExpandedState = {
    personal: false,
    joint: false
  };

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

  function projects() {
    const app = bridge();
    return typeof app.getProjects === 'function' ? app.getProjects() : [];
  }

  function archive() {
    const app = bridge();
    return typeof app.getProjectArchive === 'function' ? app.getProjectArchive() : null;
  }

  function projectById(id) {
    return projects().find((item) => item && String(item.id) === String(id));
  }

  const plazaBannerState = {
    loading: false,
    loaded: false,
    projects: []
  };

  const plazaBannerSlots = [
    { x: -6, y: 15, size: 50 },
    { x: 13, y: 15, size: 50 },
    { x: 32, y: 15, size: 50 },
    { x: 51, y: 15, size: 50 },
    { x: 70, y: 15, size: 50 },
    { x: 89, y: 15, size: 50 },
    { x: 108, y: 15, size: 50 },
    { x: 4, y: 53, size: 50 },
    { x: 23, y: 53, size: 50 },
    { x: 42, y: 53, size: 50 },
    { x: 78, y: 53, size: 50 },
    { x: 97, y: 53, size: 50 },
    { x: 116, y: 53, size: 50 },
    { x: -6, y: 91, size: 50 },
    { x: 13, y: 91, size: 50 },
    { x: 32, y: 91, size: 50 },
    { x: 51, y: 91, size: 50 },
    { x: 70, y: 91, size: 50 },
    { x: 89, y: 91, size: 50 },
    { x: 108, y: 91, size: 50 }
  ];
  const plazaBannerFocusSlot = { x: 60, y: 43, size: 72 };

  function memberCountOf(project) {
    return Number(project.member_count || project.memberCount || project.members || 0) || 0;
  }

  function sortedBannerProjects() {
    return plazaBannerState.projects
      .slice()
      .sort((a, b) => memberCountOf(b) - memberCountOf(a) || String(titleOf(a)).localeCompare(String(titleOf(b))))
      .slice(0, plazaBannerSlots.length + 1);
  }

  function ensurePlazaBannerProjects() {
    if (plazaBannerState.loading || plazaBannerState.loaded) return;
    plazaBannerState.loading = true;
    fetch('/api/store/projects?limit=18&sort=members', { cache: 'no-store' })
      .then((res) => res.json().then((data) => ({ ok: res.ok, data })))
      .then(({ ok, data }) => {
        if (!ok) throw new Error(data && data.error ? data.error : 'load failed');
        plazaBannerState.projects = Array.isArray(data.projects) ? data.projects : [];
        plazaBannerState.loaded = true;
      })
      .catch(() => {
        plazaBannerState.projects = [];
        plazaBannerState.loaded = true;
      })
      .finally(() => {
        plazaBannerState.loading = false;
        render();
      });
  }

  function formatTime(value) {
    const app = bridge();
    if (typeof app.formatTime === 'function') return app.formatTime(value);
    return value ? String(value).replace('T', ' ').slice(0, 16) : '时间';
  }

  function titleOf(project) {
    return project.name || project.title || '未命名项目';
  }

  function projectInitial(project) {
    const title = String(titleOf(project)).trim();
    if (title.startsWith('一龙')) return '龙';
    return Array.from(title)[0] || '项';
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

  function renderProjectThumb(project) {
    const iconUrl = iconUrlOf(project);
    const label = escapeHtml(projectInitial(project));
    return `
      <span class="project-home-thumb" aria-hidden="true">
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
    if (isSystemProject(project)) return '会话归档';
    return project.last_task_status || project.status || project.stage || '待提交需求';
  }

  function workspaceStatusOf(project) {
    const status = project.workspace_status || project.workspaceStatus;
    if (!status || typeof status !== 'object') return '';
    const latest = String(status.latest_execution_status || status.latestExecutionStatus || '').trim().toLowerCase();
    if (latest === 'running') return '运行中';
    const kind = String(status.workspace_kind || status.workspaceKind || '').trim();
    if (kind === 'system_archive') return systemProjectLabel(project) + '归档';
    if (kind === 'pc_node_workspace') {
      const canRun = boolOf(status.can_run_on_pc || status.canRunOnPc);
      const nodeOnline = boolOf(status.node_online || status.nodeOnline);
      const hasNode = Boolean(status.node_id || status.nodeId);
      const warnings = Number(status.warning_count || status.warningCount || 0) || 0;
      if (canRun && warnings <= 0) return 'PC在线';
      if (canRun) return 'PC有提醒';
      if (hasNode && !nodeOnline) return 'PC离线';
      return 'PC需配置';
    }
    if (kind === 'external_workspace') return '外部工作区';
    if (kind === 'server_workspace') return '服务器工作区';
    return '';
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
    const owner = project.owner_account || project.ownerAccount || project.created_by_account || project.owner;
    if (owner) return owner;
    if (isSystemProject(project)) return '一龙';
    return isJointProject(project) ? '未知' : currentDisplayName();
  }

  function cardMemberCount(project) {
    const raw = project.member_count ?? project.memberCount ?? project.members;
    const count = Array.isArray(raw) ? raw.length : Number(raw);
    if (Number.isFinite(count) && count >= 0) return count;
    return isJointProject(project) ? 0 : 1;
  }

  function projectCodeOf(project) {
    return String(project.project_description || project.projectDescription || project.description || '').trim();
  }

  function renderBannerIcon(project, slot, extraClass) {
    const iconUrl = project ? iconUrlOf(project) : '';
    const label = project ? escapeHtml(projectInitial(project)) : '';
    const fontSize = Math.round(slot.size * 0.36);
    return `
      <span class="project-plaza-tile ${extraClass || ''}" style="--x:${slot.x}%;--y:${slot.y}%;--size:${slot.size}px;--font:${fontSize}px">
        ${label}
        ${iconUrl ? `<img src="${escapeHtml(iconUrl)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
  }

  function renderPlazaTiles() {
    const bannerProjects = sortedBannerProjects();
    const focus = bannerProjects[0] || null;
    const rest = bannerProjects.slice(1);
    const tiles = plazaBannerSlots.map((slot, index) => renderBannerIcon(rest[index] || null, slot));
    tiles.push(renderBannerIcon(focus, plazaBannerFocusSlot, 'project-plaza-focus-tile'));
    return tiles.join('');
  }

  function renderPlazaBanner() {
    ensurePlazaBannerProjects();
    return `
      <button class="project-plaza-banner" type="button" data-project-home-action="plaza" aria-label="进入项目广场">
        <span class="project-plaza-pattern" aria-hidden="true">${renderPlazaTiles()}</span>
        <span class="project-plaza-title">项目广场</span>
        <span class="project-plaza-search" aria-hidden="true"></span>
      </button>
    `;
  }

  function renderSection(key, title, items, emptyAction) {
    const expandable = items.length > COLLAPSED_PROJECT_LIMIT;
    const expanded = expandable && sectionExpandedState[key];
    const headAttrs = expandable
      ? ` role="button" tabindex="0" data-project-home-action="toggle-section" data-project-section="${escapeHtml(key)}" aria-expanded="${expanded ? 'true' : 'false'}"`
      : '';
    const cells = sectionCells(sectionItems(items, expanded));
    return `
      <div class="project-home-section" data-project-section="${escapeHtml(key)}">
        <div class="project-home-section-head ${expandable ? 'expandable' : ''}"${headAttrs}>
          <div class="project-home-section-title">${escapeHtml(title)}</div>
          <div class="project-home-section-arrow ${expanded ? 'expanded' : ''}">›</div>
        </div>
        <div class="project-home-grid" data-project-home-grid>
          ${renderGridCells(cells, emptyAction)}
        </div>
      </div>
    `;
  }

  function sectionItems(items, expanded) {
    return expanded || items.length <= COLLAPSED_PROJECT_LIMIT
      ? items
      : items.slice(0, COLLAPSED_PROJECT_LIMIT);
  }

  function sectionCells(items) {
    const cells = items.slice();
    if (!cells.length) {
      cells.push(null, null);
    } else if (cells.length % 2) {
      cells.push(null);
    }
    return cells;
  }

  function renderGridCells(cells, emptyAction) {
    return cells.map((project) => project ? renderCard(project) : renderEmptyCard(emptyAction)).join('');
  }

  function renderCard(project) {
    const system = isSystemProject(project);
    const joint = !system && isJointProject(project);
    const kind = system ? '系统档案' : joint ? '联合开发' : '个人独立';
    const status = workspaceStatusOf(project) || stageOf(project);
    const meta = `${kind} · ${conversationCount(project)}个会话 · ${status}`;
    const app = bridge();
    const active = typeof app.isCurrentProject === 'function' && app.isCurrentProject(project);
    const projectCode = projectCodeOf(project);
    return `
      <div class="project-home-card ${active ? 'active' : ''}" role="button" tabindex="0" data-project-home-action="open" data-project-id="${escapeHtml(project.id)}" aria-label="打开项目 ${escapeHtml(titleOf(project))}">
        <button class="project-home-more" type="button" data-project-home-action="menu" data-project-id="${escapeHtml(project.id)}" aria-label="项目操作" title="项目操作">...</button>
        <span class="project-home-card-body">
          <span class="project-home-card-head">
            ${renderProjectThumb(project)}
            <span class="project-home-card-details">
              <span>创建者：${escapeHtml(ownerOf(project))}</span>
              <span>成员：${escapeHtml(cardMemberCount(project))}</span>
            </span>
          </span>
          ${projectCode ? `
            <span class="project-home-card-divider" aria-hidden="true"></span>
            <span class="project-home-code">项目代号：${escapeHtml(projectCode)}</span>
          ` : ''}
        </span>
        <span class="project-home-info">
          <span class="project-home-title-row">
            <span class="project-home-name">${escapeHtml(titleOf(project))}</span>
            <span class="project-home-time">${escapeHtml(projectTime(project))}</span>
          </span>
          <span class="project-home-meta">${escapeHtml(meta)}</span>
        </span>
      </div>
    `;
  }

  function renderProjectActionThumb(project) {
    const iconUrl = iconUrlOf(project);
    const label = escapeHtml(projectInitial(project));
    return `
      <span class="project-home-action-thumb" aria-hidden="true">
        ${label}
        ${iconUrl ? `<img src="${escapeHtml(iconUrl)}" alt="" loading="lazy" onerror="this.remove()" />` : ''}
      </span>
    `;
  }

  function renderEmptyCard(action) {
    const attr = action ? ` data-project-home-action="${action}" tabindex="0"` : ' tabindex="-1"';
    const plus = action ? '<span class="project-home-empty-plus" aria-hidden="true">+</span>' : '';
    return `<button class="project-home-empty-card" type="button"${attr} aria-label="空项目位">${plus}</button>`;
  }

  function render() {
    const root = document.getElementById(ROOT_ID);
    if (!root) return;
    attachEvents(root);
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
    const personal = serverPersonal || system.concat(owned);
    const joint = Array.isArray(archiveData && archiveData.shared_projects)
      ? archiveData.shared_projects.filter((project) => !isSystemProject(project))
      : all.filter((project) => !isSystemProject(project) && isJointProject(project));
    root.innerHTML = [
      renderPlazaBanner(),
      renderSection('personal', '个人项目', personal, 'create'),
      renderSection('joint', '联合项目', joint, null),
      renderActionMenu()
    ].join('');
  }

  function renderActionMenu() {
    const project = actionMenuState.projectId ? projectById(actionMenuState.projectId) : null;
    if (!project) return '';
    const joint = isJointProject(project);
    const status = projectStatusText(project, joint);
    return `
      <div class="project-home-action-mask" data-project-home-action="close-menu" role="dialog" aria-modal="true" aria-label="项目操作">
        <div class="project-home-action-panel" data-project-home-menu-panel>
          <div class="project-home-action-header">
            ${renderProjectActionThumb(project)}
            <span class="project-home-action-head-text">
              <span class="project-home-action-title">${escapeHtml(titleOf(project))}</span>
              <span class="project-home-action-status ${joint ? 'joint' : 'personal'}">${escapeHtml(status)}</span>
            </span>
          </div>
          <div class="project-home-action-list">
            ${projectActionRows(project, joint).map(renderActionRow).join('')}
          </div>
        </div>
      </div>
    `;
  }

  function projectStatusText(project, joint) {
    if (isSystemProject(project)) {
      return `${systemProjectLabel(project)} · 专属会话归档`;
    }
    if (!joint) return '个人项目 · 开发会话';
    const joinMode = String(project.join_mode || project.joinMode || 'invite').trim();
    if (joinMode === 'open') return '联合项目 · 商城公开';
    if (joinMode === 'readonly') return '联合项目 · 广场只读';
    if (joinMode === 'approval') return '联合项目 · 加入需审批';
    return '联合项目 · 邀请协作';
  }

  function projectActionRows(project, joint) {
    if (isSystemProject(project)) {
      return [
        {
          action: 'open',
          icon: '开',
          title: '打开会话归档',
          subtitle: '进入这个系统入口的历史会话'
        },
        {
          action: 'record',
          icon: '记',
          title: '查看记录',
          subtitle: '查看最近状态和会话进度'
        }
      ];
    }
    const role = roleOf(project);
    const isOwner = !role || role === 'owner';
    const rows = [
      {
        action: 'open',
        icon: '开',
        title: '打开项目空间',
        subtitle: joint ? '进入联合协作空间' : '进入项目开发会话'
      },
      {
        action: 'git',
        icon: 'G',
        title: 'Git 仓库',
        subtitle: '查看或配置项目远端'
      },
      {
        action: 'record',
        icon: '记',
        title: '项目记录',
        subtitle: '查看开发进度和最近状态'
      }
    ];
    if (isOwner) {
      rows.push({
        action: 'visibility',
        icon: '权',
        title: '协作权限 / 商城公开',
        subtitle: '管理加入方式和可见范围'
      });
    }
    if (isOwner && projects().length > 1 && project.id !== 'elon-self') {
      rows.push({
        action: 'delete',
        icon: '删',
        title: '删除项目',
        subtitle: '从服务器和列表移除',
        danger: true
      });
    } else if (!isOwner) {
      rows.push({
        action: 'leave',
        icon: '退',
        title: '退出项目',
        subtitle: '移出我的项目列表',
        danger: true
      });
    }
    return rows;
  }

  function systemProjectLabel(project) {
    const key = systemKeyOf(project);
    if (key === 'phone_control') return '手机控制';
    if (key === 'chat_memory') return '聊天记忆';
    const sourceType = sourceTypeOf(project);
    if (sourceType === 'agent_balloon') return '手机控制';
    if (sourceType === 'chat_memory') return '聊天记忆';
    return '系统档案';
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
      if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
        const card = event.target.closest('.project-home-card[data-project-id]');
        if (!card) return;
        event.preventDefault();
        openActionMenu(card.dataset.projectId);
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
    if (!card || event.target.closest('.project-home-more')) return;
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
    if (action === 'menu') {
      openActionMenu(actionEl.dataset.projectId);
      return;
    }
    if (action === 'create') {
      if (typeof app.openNewProject === 'function') app.openNewProject();
      return;
    }
    if (action === 'plaza') {
      if (typeof app.openProjectPlaza === 'function') app.openProjectPlaza();
      return;
    }
    if (action === 'toggle-section') {
      toggleSection(actionEl.dataset.projectSection || '');
      return;
    }
    if (action === 'open') {
      const id = actionEl.dataset.projectId;
      const project = projectById(id);
      if (actionMenuState.projectId) closeActionMenu();
      if (project && typeof app.openProject === 'function') app.openProject(project);
      return;
    }

    const project = projectById(actionEl.dataset.projectId);
    if (!project) {
      closeActionMenu();
      return;
    }
    closeActionMenu();
    if (action === 'git' && typeof app.openProjectGit === 'function') {
      app.openProjectGit(project);
      return;
    }
    if (action === 'record' && typeof app.openProjectProgress === 'function') {
      app.openProjectProgress(project);
      return;
    }
    if (action === 'visibility' && typeof app.openProjectVisibility === 'function') {
      app.openProjectVisibility(project);
      return;
    }
    if (action === 'delete' && typeof app.deleteProject === 'function') {
      app.deleteProject(project);
      return;
    }
    if (action === 'leave' && typeof app.leaveProject === 'function') {
      app.leaveProject(project);
      return;
    }
  }

  function toggleSection(key) {
    if (key !== 'personal' && key !== 'joint') return;
    const root = document.getElementById(ROOT_ID);
    const section = root && root.querySelector(`.project-home-section[data-project-section="${key}"]`);
    const grid = section && section.querySelector('[data-project-home-grid]');
    const head = section && section.querySelector('.project-home-section-head');
    const arrow = section && section.querySelector('.project-home-section-arrow');
    const items = projectSectionItems(key);
    if (!section || !grid || !head || !arrow || items.length <= COLLAPSED_PROJECT_LIMIT) return;

    const targetExpanded = !sectionExpandedState[key];
    sectionExpandedState[key] = targetExpanded;
    const cells = sectionCells(sectionItems(items, targetExpanded));
    const nextHtml = renderGridCells(cells, key === 'personal' ? 'create' : null);
    const fromHeight = grid.offsetHeight;
    const toHeight = nextGridHeight(grid, nextHtml, targetExpanded);

    head.setAttribute('aria-expanded', targetExpanded ? 'true' : 'false');
    arrow.classList.toggle('expanded', targetExpanded);
    animateGridHeight(grid, fromHeight, toHeight, () => {
      if (!targetExpanded) grid.innerHTML = nextHtml;
    });
  }

  function projectSectionItems(key) {
    const all = projects();
    return key === 'joint' ? all.filter(isJointProject) : all.filter((project) => !isJointProject(project));
  }

  function nextGridHeight(grid, html, applyNow) {
    if (applyNow) {
      grid.innerHTML = html;
      return grid.scrollHeight;
    }
    const clone = grid.cloneNode(false);
    clone.innerHTML = html;
    clone.style.position = 'absolute';
    clone.style.visibility = 'hidden';
    clone.style.pointerEvents = 'none';
    clone.style.height = 'auto';
    clone.style.transition = 'none';
    clone.style.width = `${grid.getBoundingClientRect().width}px`;
    grid.parentNode.appendChild(clone);
    const height = clone.scrollHeight;
    clone.remove();
    return height;
  }

  function animateGridHeight(grid, fromHeight, toHeight, after) {
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      grid.removeEventListener('transitionend', finish);
      grid.style.height = '';
      grid.style.overflow = '';
      grid.style.transition = '';
      after();
    };
    grid.style.overflow = 'hidden';
    grid.style.height = `${fromHeight}px`;
    grid.style.transition = 'height 260ms cubic-bezier(0.2, 0.8, 0.2, 1)';
    window.requestAnimationFrame(() => {
      grid.style.height = `${toHeight}px`;
    });
    grid.addEventListener('transitionend', finish);
    window.setTimeout(finish, 340);
  }

  window.ElonProjectHome = { render };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', render, { once: true });
  } else {
    render();
  }
})();
