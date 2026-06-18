(function () {
  function create(deps) {
    const state = deps.state;
    const els = deps.els;
    const api = deps.api;
    const clean = deps.clean || ((value) => String(value || '').trim());
    const escapeHtml = deps.escapeHtml || fallbackEscapeHtml;
    const cacheAgentKey = 'elon_pc_selected_agent_name';
    const cacheLabelKey = 'elon_pc_selected_model_label';
    const model = {
      options: [],
      selectedAgent: clean(localStorage.getItem(cacheAgentKey)),
      label: clean(localStorage.getItem(cacheLabelKey)) || 'AI',
      codexCliOnly: false,
      byokEnabled: false,
      initialized: false,
      loading: false,
      error: ''
    };
    state.selectedAgentName = model.selectedAgent;
    let backdrop = null;
    let popover = null;
    let removeEscapeHandler = null;

    function currentUserId() {
      const user = state.user || {};
      return clean(user.id || user.user_id || user.userId || user.uid);
    }

    function fallbackEscapeHtml(value) {
      return String(value || '').replace(/[&<>"']/g, (ch) => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;'
      }[ch]));
    }

    function selectedAgentForRequest() {
      return clean(model.selectedAgent);
    }

    function reset() {
      model.options = [];
      model.selectedAgent = '';
      model.label = 'AI';
      state.selectedAgentName = '';
      model.initialized = false;
      model.error = '';
      closeModelPicker();
      updateButton();
    }

    function updateButton() {
      if (!els.aiTaskBtn) return;
      const label = shortButtonLabel(model.label);
      els.aiTaskBtn.textContent = label || 'AI';
      const taskHint = state.activeChannelKind === 'ai_development'
        ? '；发送消息会在当前 AI 开发频道发起任务'
        : '';
      els.aiTaskBtn.title = `选择 AI 模型，当前：${model.label || '服务器默认'}${taskHint}`;
    }

    async function loadModelOptions(showErrors) {
      const userId = currentUserId();
      if (!userId) {
        reset();
        return;
      }
      model.loading = true;
      model.error = '';
      updateButton();
      try {
        const data = await api(`/api/user/${encodeURIComponent(userId)}/agent`);
        applyModelPayload(data || {});
      } catch (error) {
        model.error = error.message || '模型列表加载失败';
        if (showErrors) window.alert(model.error);
      } finally {
        model.loading = false;
        updateButton();
        renderPopover();
      }
    }

    function applyModelPayload(data) {
      model.codexCliOnly = !!data.codex_cli_only;
      model.byokEnabled = !!data.user_byok_api_enabled;
      model.options = buildOptions(data);
      const config = data.config || {};
      const configured = clean(config.use_agent);
      const cached = clean(localStorage.getItem(cacheAgentKey));
      const includeDefault = shouldIncludeDefault(data);
      const configuredOption = optionByAgent(configured);
      const cachedOption = optionByAgent(cached);
      const defaultOption = optionByAgent(clean(data.default_agent));
      let selected = '';

      if (configuredOption) selected = configuredOption.agentName;
      else if (cachedOption) selected = cachedOption.agentName;
      else if (!includeDefault && defaultOption) selected = defaultOption.agentName;
      else if (!includeDefault && model.options[0]) selected = model.options[0].agentName;

      const hasCustomConfig = !!(clean(config.api_base) || clean(config.model));
      model.selectedAgent = selected;
      state.selectedAgentName = selected;
      if (hasCustomConfig && !selected) {
        model.label = '自定义模型';
      } else {
        const selectedOption = optionByAgent(selected);
        model.label = selectedOption ? selectedOption.label : '服务器默认';
      }
      cacheSelection(model.selectedAgent, model.label);
      model.initialized = true;
    }

    function buildOptions(data) {
      const options = [];
      if (shouldIncludeDefault(data)) {
        options.push({
          label: '服务器默认',
          agentName: '',
          provider: 'default',
          backend: 'default',
          modelId: '',
          subtitle: '使用服务器当前默认模型'
        });
      }
      const agents = Array.isArray(data.available_agents) ? data.available_agents : [];
      agents.forEach((item) => {
        const option = normalizeAgentOption(item || {});
        if (option) options.push(option);
      });
      return options;
    }

    function shouldIncludeDefault(data) {
      return !data.codex_cli_only;
    }

    function normalizeAgentOption(item) {
      const agentName = clean(item.name);
      if (!agentName) return null;
      const provider = clean(item.provider || item.backend || 'api').toLowerCase();
      const modelId = clean(item.model);
      const displayModel = clean(item.display_model);
      const rawLabel = clean(item.label);
      const reasoningEffort = clean(item.reasoning_effort);
      const reasoningSummary = clean(item.reasoning_summary);
      const verbosity = clean(item.verbosity);
      const baseLabel = displayModel || displayModelLabel(provider, modelId, rawLabel);
      const label = displayModel
        ? displayModel
        : withCodexRunMeta(baseLabel, provider, reasoningEffort, verbosity);
      return {
        label,
        agentName,
        provider,
        backend: clean(item.backend),
        modelId,
        reasoningEffort,
        reasoningSummary,
        verbosity,
        subtitle: codexOptionSubtitle(provider, modelId, reasoningEffort, reasoningSummary, verbosity)
          || clean(item.api_base)
          || providerGroupTitle(provider)
      };
    }

    function displayModelLabel(provider, modelId, rawLabel) {
      if (rawLabel) return rawLabel;
      if (modelId && modelId.toLowerCase() !== 'default') return friendlyModelName(modelId);
      if (provider === 'codex') return 'Codex 默认';
      return providerGroupTitle(provider);
    }

    function withCodexRunMeta(label, provider, reasoningEffort, verbosity) {
      if (provider !== 'codex') return label;
      const parts = [label || 'Codex 默认'];
      if (reasoningEffort) parts.push(`推理 ${reasoningEffort}`);
      if (verbosity) parts.push(`输出 ${verbosity}`);
      return parts.join(' · ');
    }

    function codexOptionSubtitle(provider, modelId, reasoningEffort, reasoningSummary, verbosity) {
      if (provider !== 'codex') return '';
      const parts = [];
      if (modelId && modelId.toLowerCase() !== 'default') parts.push(`模型 ${friendlyModelName(modelId)}`);
      if (reasoningEffort) parts.push(`推理 ${reasoningEffort}`);
      if (verbosity) parts.push(`输出 ${verbosity}`);
      if (reasoningSummary) parts.push(`摘要 ${reasoningSummary}`);
      return parts.join(' · ');
    }

    function friendlyModelName(value) {
      const modelId = clean(value);
      const lower = modelId.toLowerCase();
      if (lower === 'gpt-5.5') return 'GPT-5.5';
      if (lower === 'gpt-5.4') return 'GPT-5.4';
      if (lower === 'gpt-5.4-mini') return 'GPT-5.4 mini';
      if (lower === 'gpt-5.3-codex-spark') return 'GPT-5.3 Codex Spark';
      if (lower === 'gpt-5.3-codex') return 'GPT-5.3 Codex';
      if (lower === 'gpt-5.2') return 'GPT-5.2';
      if (lower === 'gpt-5') return 'GPT-5';
      return modelId;
    }

    function shortButtonLabel(label) {
      const value = clean(label).replace(/^服务器默认$/, '默认');
      if (!value || value === 'AI') return 'AI';
      if (value.includes('GPT-5.5')) return 'GPT-5.5';
      if (value.includes('GPT-5.4 mini')) return '5.4 mini';
      if (value.includes('GPT-5.4')) return 'GPT-5.4';
      if (value.includes('GPT-5.3')) return 'GPT-5.3';
      if (value.includes('Claude')) return 'Claude';
      if (value.includes('Gemini')) return 'Gemini';
      if (value.includes('Copilot')) return 'Copilot';
      if (value.includes('Codex')) return 'Codex';
      return value.length > 8 ? `${value.slice(0, 8)}` : value;
    }

    function optionByAgent(agentName) {
      const key = clean(agentName);
      return model.options.find((option) => option.agentName === key) || null;
    }

    function providerGroupTitle(provider) {
      const value = clean(provider).toLowerCase();
      if (!value || value === 'default') return '默认';
      if (value === 'codex') return 'Codex CLI';
      if (value === 'copilot') return 'GitHub Copilot';
      if (value === 'openai') return 'OpenAI';
      if (value === 'anthropic' || value === 'claude') return 'Claude';
      if (value === 'google' || value === 'gemini') return 'Gemini';
      if (value === 'api') return 'API 模型';
      return value.toUpperCase();
    }

    function cacheSelection(agentName, label) {
      const cleanAgent = clean(agentName);
      if (cleanAgent) localStorage.setItem(cacheAgentKey, cleanAgent);
      else localStorage.removeItem(cacheAgentKey);
      localStorage.setItem(cacheLabelKey, clean(label) || '服务器默认');
    }

    async function openModelPicker() {
      if (!currentUserId()) {
        window.alert('请先登录一龙账号，再选择 AI 模型。');
        return;
      }
      if (!model.initialized && !model.loading) {
        await loadModelOptions(true);
      }
      renderPopover(true);
    }

    function renderPopover(forceOpen) {
      if (!forceOpen && !popover) return;
      closeModelPicker();
      if (!els.aiTaskBtn) return;
      backdrop = document.createElement('button');
      backdrop.type = 'button';
      backdrop.className = 'pc-model-backdrop';
      backdrop.setAttribute('aria-label', '关闭模型选择');
      popover = document.createElement('section');
      popover.className = 'pc-model-popover';
      popover.setAttribute('role', 'dialog');
      popover.setAttribute('aria-label', '选择 AI 模型');
      popover.innerHTML = popoverHtml();
      document.body.append(backdrop, popover);
      positionPopover();
      bindPopoverEvents();
      window.addEventListener('resize', positionPopover);
      removeEscapeHandler = (event) => {
        if (event.key === 'Escape') closeModelPicker();
      };
      document.addEventListener('keydown', removeEscapeHandler);
    }

    function popoverHtml() {
      return `
        <header class="pc-model-header">
          <div>
            <strong>选择 AI 模型</strong>
            <span>${escapeHtml(model.label || '服务器默认')}</span>
          </div>
          <button class="pc-model-close" type="button" data-action="close" aria-label="关闭">×</button>
        </header>
        <div class="pc-model-list">
          ${model.loading ? '<div class="pc-model-empty">正在读取模型列表...</div>' : optionsHtml()}
        </div>
        <footer class="pc-model-footer">
          <button type="button" data-action="refresh">刷新</button>
          ${model.codexCliOnly ? '' : '<button type="button" data-action="settings">完整模型设置</button>'}
        </footer>
      `;
    }

    function optionsHtml() {
      if (model.error) return `<div class="pc-model-error">${escapeHtml(model.error)}</div>`;
      if (!model.options.length) {
        return '<div class="pc-model-empty">当前没有可选模型。请检查服务器 agent 配置或 PC 节点 CLI 配置。</div>';
      }
      const groups = new Map();
      model.options.forEach((option, index) => {
        const title = providerGroupTitle(option.provider);
        if (!groups.has(title)) groups.set(title, []);
        groups.get(title).push({ option, index });
      });
      return Array.from(groups.entries()).map(([title, rows]) => `
        ${title === '默认' ? '' : `<div class="pc-model-section">${escapeHtml(title)}</div>`}
        ${rows.map(({ option, index }) => optionHtml(option, index)).join('')}
      `).join('');
    }

    function optionHtml(option, index) {
      const active = option.agentName === model.selectedAgent;
      return `
        <button class="pc-model-option${active ? ' active' : ''}" type="button" data-index="${index}">
          <span>
            <strong>${escapeHtml(option.label)}</strong>
            ${option.subtitle ? `<span>${escapeHtml(option.subtitle)}</span>` : ''}
          </span>
          <span class="pc-model-check">${active ? '✓' : ''}</span>
        </button>
      `;
    }

    function bindPopoverEvents() {
      if (!popover || !backdrop) return;
      backdrop.addEventListener('click', closeModelPicker);
      popover.querySelectorAll('[data-index]').forEach((button) => {
        button.addEventListener('click', () => {
          const option = model.options[Number(button.dataset.index)];
          if (option) saveModelSelection(option);
        });
      });
      popover.querySelectorAll('[data-action]').forEach((button) => {
        button.addEventListener('click', () => handlePopoverAction(button.dataset.action));
      });
    }

    function handlePopoverAction(action) {
      if (action === 'close') closeModelPicker();
      else if (action === 'refresh') loadModelOptions(true);
      else if (action === 'settings') window.open('/web', '_blank');
    }

    async function saveModelSelection(option) {
      if (!option) return;
      if (model.codexCliOnly && !option.agentName) {
        window.alert('当前已锁定使用 Codex CLI。');
        return;
      }
      const userId = currentUserId();
      if (!userId) return;
      setPopoverBusy(true);
      try {
        await api(`/api/user/${encodeURIComponent(userId)}/agent`, {
          method: 'PUT',
          body: JSON.stringify({
            use_agent: option.agentName || null,
            api_base: null,
            api_key: null,
            model: null
          })
        });
        model.selectedAgent = option.agentName || '';
        state.selectedAgentName = model.selectedAgent;
        model.label = option.label;
        model.initialized = true;
        cacheSelection(model.selectedAgent, model.label);
        updateButton();
        closeModelPicker();
      } catch (error) {
        model.error = error.message || '模型切换失败';
        renderPopover(true);
      } finally {
        setPopoverBusy(false);
      }
    }

    function setPopoverBusy(busy) {
      if (!popover) return;
      popover.querySelectorAll('button').forEach((button) => {
        button.disabled = !!busy;
      });
    }

    function positionPopover() {
      if (!popover || !els.aiTaskBtn) return;
      const rect = els.aiTaskBtn.getBoundingClientRect();
      const width = Math.min(360, window.innerWidth - 24);
      const left = Math.max(12, Math.min(rect.left, window.innerWidth - width - 12));
      const bottom = Math.max(12, window.innerHeight - rect.top + 8);
      popover.style.width = `${width}px`;
      popover.style.left = `${Math.round(left)}px`;
      popover.style.bottom = `${Math.round(bottom)}px`;
    }

    function closeModelPicker() {
      if (backdrop) backdrop.remove();
      if (popover) popover.remove();
      backdrop = null;
      popover = null;
      window.removeEventListener('resize', positionPopover);
      if (removeEscapeHandler) document.removeEventListener('keydown', removeEscapeHandler);
      removeEscapeHandler = null;
    }

    return {
      loadModelOptions,
      openModelPicker,
      selectedAgentForRequest,
      updateButton,
      reset
    };
  }

  window.ElonPcModels = { create };
})();
