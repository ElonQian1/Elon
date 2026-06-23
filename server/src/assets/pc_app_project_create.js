(function () {
  function create(ctx) {
    const {
      state, els, clean, escapeHtml, api, loadBaseData, selectProject,
      refreshActive, renderProjectRail, sameId
    } = ctx;
    const STORAGE_NONE = 'none';
    const STORAGE_AUTO = 'auto';

    function setBusy(button, busy, label) {
      if (!button) return;
      if (busy) {
        if (!button.disabled) button.dataset.label = button.textContent;
        button.disabled = true;
        button.textContent = label || '处理中...';
      } else {
        button.disabled = false;
        button.textContent = button.dataset.label || button.textContent;
        delete button.dataset.label;
      }
    }

    function formatBytes(value) {
      const bytes = Number(value || 0);
      if (!bytes) return '';
      if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
      if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
      if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
      return `${bytes} B`;
    }

    function setError(message) {
      if (!els.pcProjectCreateError) return;
      const text = clean(message);
      els.pcProjectCreateError.textContent = text;
      els.pcProjectCreateError.classList.toggle('show', !!text);
    }

    function nodeIdOf(node) {
      return clean(node && (node.node_id || node.agent_id || node.id));
    }

    function nodeCapacityTone(node) {
      const tone = clean(node && (node.capacity_tone || node.capacityTone)).toLowerCase();
      if (tone === 'ok' || tone === 'warn' || tone === 'bad') return tone;
      return '';
    }

    function nodeCapacityLabel(node) {
      return clean(node && (node.capacity_label || node.capacityLabel));
    }

    function nodeCanAcceptProject(node) {
      if (!node) return false;
      const explicit = node.can_accept_project ?? node.canAcceptProject;
      if (explicit === true || explicit === 1 || explicit === '1') return true;
      if (explicit === false || explicit === 0 || explicit === '0') return false;
      const remaining = Number(node.project_slots_remaining ?? node.projectSlotsRemaining);
      if (Number.isFinite(remaining) && remaining <= 0) return false;
      return nodeCapacityTone(node) !== 'bad';
    }

    function nodeProjectSlotsText(node) {
      const count = Number(node && (node.project_count ?? node.projectCount));
      const limit = Number(node && (node.project_limit ?? node.projectLimit));
      const remaining = Number(node && (node.project_slots_remaining ?? node.projectSlotsRemaining));
      if (Number.isFinite(count) && Number.isFinite(limit) && limit > 0) {
        const suffix = Number.isFinite(remaining) ? ` · 剩余 ${Math.max(remaining, 0)}` : '';
        return `项目 ${count}/${limit}${suffix}`;
      }
      if (Number.isFinite(remaining)) return `剩余 ${Math.max(remaining, 0)} 个项目位`;
      return '';
    }

    function nodeDiskText(node) {
      const label = formatBytes(node && (node.disk_free_bytes ?? node.diskFreeBytes));
      return label ? `磁盘 ${label}` : '磁盘未知';
    }

    function nodeHardwareText(node) {
      const direct = clean(node && (node.hardware_summary || node.hardwareSummary));
      if (direct && direct !== '硬件未知') return direct;
      const h = node && (node.hardware || node.hardwareProfile);
      if (!h) return '';
      const parts = [];
      const gpus = Array.isArray(h.gpu_names || h.gpuNames) ? (h.gpu_names || h.gpuNames).filter(Boolean) : [];
      if (gpus.length) parts.push('GPU ' + gpus.slice(0, 2).join(' / '));
      const gpuMem = formatBytes(h.gpu_memory_total_bytes ?? h.gpuMemoryTotalBytes);
      if (gpuMem) parts.push('显存 ' + gpuMem);
      const mem = formatBytes(h.memory_total_bytes ?? h.memoryTotalBytes);
      if (mem) parts.push('内存 ' + mem);
      const cores = Number(h.cpu_cores ?? h.cpuCores);
      if (Number.isFinite(cores) && cores > 0) parts.push(`CPU ${cores} 核`);
      return parts.join(' · ');
    }

    function nodeWorkspaceRuntimeText(node) {
      return node && (node.workspace_provision_ready ?? node.workspaceProvisionReady)
        ? '开发运行时就绪'
        : '开发运行时未就绪';
    }

    function nodeAiCliText(node) {
      const clis = Array.isArray(node && (node.allowed_clis || node.allowedClis))
        ? (node.allowed_clis || node.allowedClis).filter(Boolean).join('/')
        : '';
      return clis ? `AI ${clis}` : '';
    }

    function nodeCapacitySummary(node) {
      return [
        nodeCapacityLabel(node) || '容量未知',
        nodeProjectSlotsText(node),
        nodeHardwareText(node),
        nodeDiskText(node),
        nodeWorkspaceRuntimeText(node),
        nodeAiCliText(node)
      ].filter(Boolean).join(' · ');
    }

    function close() {
      els.pcProjectCreateBackdrop.hidden = true;
      setError('');
    }

    async function loadNodes() {
      const data = await api('/api/nodes');
      const nodes = (Array.isArray(data.nodes) ? data.nodes : [])
        .filter((node) => node && node.online && nodeIdOf(node));
      if (!nodes.length) {
        els.pcProjectNodeSelect.innerHTML = '<option value="">没有在线开发环境</option>';
        els.pcProjectNodeSelect.disabled = true;
        els.pcProjectStorageNodeSelect.innerHTML = '<option value="">没有可用代码存储</option>';
        els.pcProjectStorageNodeSelect.disabled = true;
        els.pcProjectStorageHint.textContent = '暂无可用开发环境，项目暂时不能创建。';
        return;
      }

      const orderedNodes = nodes.slice().sort((left, right) => {
        return Number(!nodeCanAcceptProject(left)) - Number(!nodeCanAcceptProject(right));
      });
      const selectableCount = orderedNodes.filter(nodeCanAcceptProject).length;
      const options = orderedNodes.map((node) => {
        const nodeId = nodeIdOf(node);
        const shortId = clean(node.short_id) || (nodeId.length > 16 ? '...' + nodeId.slice(-14) : nodeId);
        const label = clean(node.display_name || node.label || node.device_name) || shortId;
        const disabled = nodeCanAcceptProject(node) ? '' : ' disabled';
        return `<option value="${escapeHtml(nodeId)}"${disabled}>${escapeHtml(label)} · ${escapeHtml(shortId)} · ${escapeHtml(nodeCapacitySummary(node))}</option>`;
      }).join('');
      els.pcProjectNodeSelect.innerHTML = selectableCount
        ? options
        : '<option value="">暂无可创建项目的开发环境</option>' + options;
      els.pcProjectNodeSelect.disabled = !selectableCount;

      const storageNodes = nodes.filter((node) => (
        node.storage_ready || (node.storage && node.storage.enabled)
      ) && node.storage_repo_url_configured);
      const storageOptions = storageNodes.map((node) => {
        const nodeId = nodeIdOf(node);
        const shortId = clean(node.short_id) || (nodeId.length > 16 ? '...' + nodeId.slice(-14) : nodeId);
        const label = clean(node.display_name || node.label || node.device_name) || shortId;
        const configured = node.storage_repo_url_configured ? '可跨 PC' : '仅同机/需升级';
        return `<option value="${escapeHtml(nodeId)}">${escapeHtml(label)} · ${escapeHtml(shortId)} · ${escapeHtml(configured)}</option>`;
      }).join('');
      els.pcProjectStorageNodeSelect.innerHTML = [
        `<option value="${STORAGE_NONE}">暂不使用代码存储（推荐）</option>`,
        storageNodes.length ? `<option value="${STORAGE_AUTO}">自动选择代码存储（高级）</option>` : '',
        storageOptions
      ].join('');
      els.pcProjectStorageNodeSelect.value = STORAGE_NONE;
      els.pcProjectStorageNodeSelect.disabled = false;
      els.pcProjectStorageHint.textContent = storageNodes.length
        ? '默认先创建在开发环境上；需要跨 PC 迁移时再启用代码存储。'
        : '项目会直接创建在所选开发环境上。';
    }

    function open() {
      if (!state.token) return false;
      setError('');
      setBusy(els.pcProjectCreateSubmitBtn, false);
      els.pcProjectNameInput.value = '';
      els.pcProjectDescInput.value = '';
      els.pcProjectTemplateSelect.value = 'android_kotlin';
      els.pcProjectRepoInput.value = '';
      els.pcProjectBranchInput.value = '';
      els.pcProjectStorageHint.textContent = '';
      els.pcProjectNodeSelect.disabled = true;
      els.pcProjectNodeSelect.innerHTML = '<option value="">正在加载可用开发环境...</option>';
      els.pcProjectStorageNodeSelect.disabled = true;
      els.pcProjectStorageNodeSelect.innerHTML = `<option value="${STORAGE_NONE}">暂不使用代码存储</option>`;
      els.pcProjectCreateBackdrop.hidden = false;
      loadNodes().catch((error) => {
        els.pcProjectNodeSelect.innerHTML = '<option value="">加载开发环境失败</option>';
        els.pcProjectNodeSelect.disabled = true;
        els.pcProjectStorageNodeSelect.innerHTML = `<option value="${STORAGE_NONE}">暂不使用代码存储</option>`;
        els.pcProjectStorageNodeSelect.disabled = true;
        els.pcProjectStorageHint.textContent = '';
        setError(error.message || '加载开发环境失败');
      });
      setTimeout(() => els.pcProjectNameInput.focus(), 0);
      return true;
    }

    async function submit(event) {
      event.preventDefault();
      setError('');
      const name = clean(els.pcProjectNameInput.value);
      if (!name) {
        setError('请输入项目名');
        els.pcProjectNameInput.focus();
        return;
      }
      const nodeId = clean(els.pcProjectNodeSelect.value);
      if (!nodeId) {
        setError('请先选择一个可创建项目的开发环境');
        return;
      }
      const repoUrl = clean(els.pcProjectRepoInput.value);
      const storageChoice = clean(els.pcProjectStorageNodeSelect.value) || STORAGE_NONE;
      const storageNodeId = (!repoUrl
        && storageChoice !== STORAGE_NONE
        && storageChoice !== STORAGE_AUTO)
        ? storageChoice
        : null;
      setBusy(els.pcProjectCreateSubmitBtn, true, '创建中...');
      try {
        const data = await api('/api/projects', {
          method: 'POST',
          body: JSON.stringify({
            name,
            description: clean(els.pcProjectDescInput.value) || null,
            template: clean(els.pcProjectTemplateSelect.value) || 'android_kotlin',
            repo_url: repoUrl || null,
            branch: clean(els.pcProjectBranchInput.value) || null,
            execution_target: 'pc_node',
            node_id: nodeId,
            storage_node_id: repoUrl || storageChoice === STORAGE_AUTO ? null : storageNodeId,
            skip_storage: !!repoUrl || storageChoice === STORAGE_NONE
          })
        });
        const project = (data && data.project) || {};
        close();
        await loadBaseData();
        if (project.id && !state.projects.some((item) => sameId(item && item.id, project.id))) {
          state.projects.unshift(project);
          renderProjectRail();
        }
        if (project.id) await selectProject(project.id, {
          preferredChannelKind: 'ai_development',
          focusComposer: true
        });
        else await refreshActive();
      } catch (error) {
        setError(error.message || '创建失败');
      } finally {
        setBusy(els.pcProjectCreateSubmitBtn, false);
      }
    }

    function bindEvents() {
      els.pcProjectCreateForm.addEventListener('submit', submit);
      els.pcProjectCreateCloseBtn.addEventListener('click', close);
      els.pcProjectCreateCancelBtn.addEventListener('click', close);
      els.pcProjectCreateBackdrop.addEventListener('click', (event) => {
        if (event.target === els.pcProjectCreateBackdrop) close();
      });
    }

    return { bindEvents, close, open };
  }

  window.ElonPcProjectCreate = { create };
})();
