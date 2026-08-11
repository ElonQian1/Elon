(function () {
  'use strict';

  if (window.__elonChatGptLayout || location.origin !== 'https://chatgpt.com') return;

  const formAdapter = window.__elonChatGptFormControls;
  const composerAdapter = window.__elonChatGptComposer;
  const modelLabelPolicy = window.__elonChatGptModelLabelPolicy;
  const controlOwnershipPolicy = window.__elonChatGptControlOwnershipPolicy;
  const formCommands = window.__elonChatGptFormCommands;
  const disclosureAdapter = window.__elonChatGptDisclosureControls;
  let controlsById = new Map();
  let lastFingerprint = '';
  const MAX_DISCOVERED_CONTROLS = 512;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isVisible(node) {
    if (!node || !(node instanceof Element)) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function isInViewport(rect) {
    return rect.bottom > 0 && rect.right > 0 && rect.top < window.innerHeight && rect.left < window.innerWidth;
  }

  function sameOriginPath(node) {
    const href = node && node.getAttribute('href');
    if (!href) return '';
    try {
      const url = new URL(href, location.origin);
      return url.origin === location.origin ? url.pathname : '';
    } catch {
      return '';
    }
  }

  function relatedSameOriginPath(node) {
    const direct = sameOriginPath(node);
    if (direct) return direct;
    let container = node && node.parentElement;
    for (let depth = 0; container && depth < 4; depth += 1, container = container.parentElement) {
      const paths = Array.from(container.querySelectorAll('a[href]'))
        .map(sameOriginPath)
        .filter(Boolean);
      const contextual = paths.filter((path) => /^\/c\//.test(path) || /^\/g\/g-p-/.test(path));
      if (contextual.length === 1) return contextual[0];
      if (contextual.length > 1) return '';
      if (paths.length === 1) return paths[0];
    }
    return '';
  }

  function semanticContext(node) {
    const values = [];
    let current = node && node.parentElement;
    for (let depth = 0; current && depth < 4; depth += 1, current = current.parentElement) {
      values.push(current.id, current.getAttribute('data-testid'), current.getAttribute('aria-label'));
    }
    return cleanText(values.filter(Boolean).join(' '));
  }

  function navigationSection(node) {
    const scope = node && node.closest(
      'aside, nav, [data-testid*="sidebar" i], [role="navigation"], [role="dialog"]'
    );
    if (!scope) return '';
    const targetTop = node.getBoundingClientRect().top;
    const sectionPattern = /^(?:projects?|项目|chats?|聊天|整理聊天|pinned|已置顶)$/i;
    return actionableNodes(scope)
      .map((candidate) => ({
        label: cleanText(candidate.textContent || candidate.getAttribute('aria-label')),
        top: candidate.getBoundingClientRect().top
      }))
      .filter((candidate) => candidate.top <= targetTop + 1 && sectionPattern.test(candidate.label))
      .sort((left, right) => right.top - left.top)[0]?.label || '';
  }

  function labelOf(node, fallback) {
    const form = formAdapter && formAdapter.describe(node);
    const candidates = [
      node.innerText,
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.getAttribute('data-testid'),
      form && form.label
    ];
    const value = candidates.map(cleanText).find(Boolean);
    return (value || fallback || '操作').slice(0, 160);
  }

  function hash(value) {
    let result = 2166136261;
    const text = String(value || '').toLowerCase();
    for (let index = 0; index < text.length; index += 1) {
      result ^= text.charCodeAt(index);
      result = Math.imul(result, 16777619);
    }
    return (result >>> 0).toString(36);
  }

  function roleOf(node) {
    const form = formAdapter && formAdapter.describe(node);
    if (form && form.role) return form.role;
    const role = String(node.getAttribute('role') || '').toLowerCase();
    if ([
      'button', 'link', 'menuitem', 'menuitemcheckbox', 'menuitemradio',
      'option', 'switch', 'tab', 'treeitem'
    ].includes(role)) return role;
    if (node.matches('summary')) return 'button';
    return node.matches('a[href]') ? 'link' : 'button';
  }

  function composerNode() {
    return controlOwnershipPolicy
      ? controlOwnershipPolicy.findVisibleComposer(document, isVisible)
      : Array.from(document.querySelectorAll(
        '[data-testid="prompt-textarea"], #prompt-textarea, form [contenteditable="true"], form textarea'
      )).find(isVisible) || null;
  }

  function composerRoot() {
    const composer = composerNode();
    return composer && (composer.closest('form') || composer.closest('#thread-bottom-container') || composer.parentElement);
  }

  function headerRoot() {
    return document.querySelector('#page-header, header');
  }

  function actionableNodes(root) {
    if (!root) return [];
    const selector = [
      'button', 'a[href]', 'summary', '[role="button"]', '[role="menuitem"]',
      '[role="menuitemcheckbox"]', '[role="menuitemradio"]', '[role="option"]',
      '[role="switch"]', '[role="tab"]', '[role="treeitem"]'
    ].join(', ') +
      (formAdapter ? ', ' + formAdapter.ACTIONABLE_SELECTOR : '');
    return Array.from(root.querySelectorAll(selector))
      .filter(isVisible);
  }

  function semanticFor(node, region, index) {
    const form = formAdapter && formAdapter.describe(node);
    const path = relatedSameOriginPath(node);
    const signal = cleanText([
      node.id,
      node.getAttribute('data-testid'),
      node.getAttribute('aria-label'),
      node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
    const modelSignal = cleanText(signal + ' ' + labelOf(node, ''));
    if (form && (form.inputKind === 'search' || /search|搜索/.test(signal))) return 'search';
    if (form && form.role === 'textbox') return 'text_input';
    if (form && form.role === 'combobox') return 'selection';
    if (form && [
      'checkbox', 'radio', 'menuitemcheckbox', 'menuitemradio'
    ].includes(form.role)) return 'toggle';
    if (form && form.role === 'slider') return 'slider';
    if (
      region === 'composer' && modelLabelPolicy &&
      typeof modelLabelPolicy.isModelLabel === 'function' &&
      modelLabelPolicy.isModelLabel(modelSignal)
    ) return 'model';
    if (/^\/c\/[A-Za-z0-9_-]{1,160}$/.test(path) && node.matches('a[href]')) return 'conversation';
    if (
      region === 'overlay'
      && (/timestamp|消息时间/.test(signal)
        || /(?:today|yesterday|今天|昨天)[,，]?\s*\d{1,2}:\d{2}(?:\s*[ap]\.?m\.?)?/.test(signal))
    ) return 'timestamp';
    if (/search.chat|搜索聊天/.test(signal)) return 'search';
    if (/library|文件库|资料库/.test(signal + ' ' + path)) return 'library';
    if (/scheduled|schedule|已安排|任务/.test(signal + ' ' + path)) return 'tasks';
    if (/project|项目/.test(signal + ' ' + path)) return 'project';
    if (/\bgpt(s)?\b|探索.?gpt|发现.?gpt/.test(signal + ' ' + path)) return 'gpts';
    if (/setting|设置/.test(signal + ' ' + path)) return 'settings';
    if (/create.*(?:file|website)|创建.*(?:文件|网站)/.test(signal)) return 'create_asset';
    if (/view.*files.*chat|在聊天中查看文件/.test(signal)) return 'conversation_files';
    if (/unpin|pin.chat|取消置顶|置顶聊天/.test(signal)) return 'pin';
    if (/unarchive|archive|取消归档|归档/.test(signal)) return 'archive';
    if (/sources?|citations?|文件和来源|查看来源|来源/.test(signal)) return 'sources';
    if (/open.*(?:image|photo|media)|打开.*(?:图片|照片|媒体)/.test(signal)) return 'open_media';
    if (/composer-plus|attach|upload|添加|附件|上传/.test(signal)) return 'attachment';
    if (
      region === 'composer'
      && (/(?:reasoning|thinking).*(?:effort|level)|推理|思考强度/.test(signal)
        || /(^|\s)(?:轻度|标准|中度|重度|极高)($|\s)/.test(signal))
    ) return 'model';
    if (/model|模型|gpt-|sol/.test(signal)) return 'model';
    if (/read.aloud|朗读/.test(signal)) return 'read_aloud';
    if (/dictat|听写|语音输入/.test(signal)) return 'dictation';
    if (/voice.mode|start.voice|microphone|启动语音|语音功能|麦克风/.test(signal)) return 'voice_mode';
    if (/thought.for|reasoning|思考了|思考过程|推理过程/.test(signal)) return 'reasoning_details';
    if (/send|submit|发送/.test(signal)) return 'send';
    if (/stop|停止/.test(signal)) return 'stop';
    if (/copy|复制/.test(signal)) return 'copy';
    if (/regenerate|try.again|重新生成|重试/.test(signal)) return 'regenerate';
    if (/edit|编辑/.test(signal)) return 'edit';
    if (/share|分享/.test(signal)) return 'share';
    if (/branch|分支/.test(signal)) return 'branch';
    if (/feedback|good.response|bad.response|点赞|点踩|反馈/.test(signal)) return 'feedback';
    if (/delete|删除/.test(signal)) return 'delete';
    if (/close|dismiss|关闭|取消/.test(signal)) return 'close';
    if (/confirm|确定|确认/.test(signal)) return 'confirm';
    if (/new.chat|create.new|新建.*会话|新聊天/.test(signal)) return 'new_conversation';
    if (/profile|account|头像|账户/.test(signal)) return 'profile';
    if (/\bmore\b|更多/.test(signal)) return 'more';
    if (region === 'suggestions') return 'suggestion';
    if (region === 'header' && index === 0) return 'navigation';
    if (region === 'header' && /workspace|工作区|(^|\s)工作($|\s)|personal|team|business/.test(signal)) return 'title';
    const pageSemantic = window.__elonChatGptPageSemanticPolicy
      && window.__elonChatGptPageSemanticPolicy.classify({
        pathname: location.pathname,
        path,
        region,
        signal,
        label: labelOf(node, ''),
        context: semanticContext(node),
        section: navigationSection(node),
        isLink: node.matches('a[href]')
      });
    if (pageSemantic) return pageSemantic;
    return 'action';
  }

  function defaultLabel(semantic) {
    return ({
      navigation: '打开导航',
      title: '切换工作区',
      profile: '账户',
      new_conversation: '新建会话',
      attachment: '添加附件',
      model: '选择模型',
      dictation: '开始听写',
      voice_mode: '启动语音功能',
      send: '发送',
      stop: '停止生成',
      suggestion: '使用建议',
      copy: '复制',
      regenerate: '重新生成',
      edit: '编辑',
      share: '分享',
      feedback: '反馈',
      read_aloud: '朗读',
      branch: '创建分支',
      delete: '删除',
      close: '关闭',
      confirm: '确认',
      conversation: '打开会话',
      search: '搜索聊天',
      text_input: '输入内容',
      selection: '选择选项',
      toggle: '切换选项',
      slider: '调整数值',
      library: '文件库',
      apps: '应用',
      tasks: '任务',
      project: '项目',
      gpts: 'GPT',
      settings: '设置',
      create_asset: '创建文件或网站',
      sources: '文件和来源',
      conversation_files: '在聊天中查看文件',
      pin: '置顶聊天',
      archive: '归档',
      more: '更多操作',
      personalization: '个性化',
      help: '帮助',
      logout: '退出登录',
      plan: '套餐',
      open_media: '打开媒体',
      reasoning_details: '查看思考过程',
      timestamp: '消息时间'
    })[semantic] || '操作';
  }

  function controlId(semantic, node, label, region, used, contextId) {
    const fixed = ['navigation', 'title', 'profile', 'new_conversation', 'attachment', 'model', 'dictation', 'send', 'stop'];
    const identity = contextId || [node.id, node.getAttribute('data-testid'), label].join('|');
    const base = fixed.includes(semantic) && !contextId
      ? 'control_' + semantic
      : 'control_' + (contextId && region === 'message' ? 'message_' + hash(contextId) + '_' : '')
        + semantic + '_' + hash(identity);
    if (!used.has(base)) return base;
    return base + '_' + hash(label + '|' + used.size);
  }

  function addRegionControls(target, root, region, used, filter, contextId) {
    const activeComposer = region === 'composer' ? composerNode() : null;
    actionableNodes(root).forEach((node, index) => {
      if (filter && !filter(node)) return;
      if (composerAdapter && composerAdapter.ownsOptionNode(node)) return;
      if (
        controlOwnershipPolicy && controlOwnershipPolicy.isPrimaryComposerTextControl(
          node,
          region,
          activeComposer,
          formAdapter && formAdapter.describe
        )
      ) return;
      const semantic = semanticFor(node, region, index);
      const label = labelOf(node, defaultLabel(semantic));
      const path = relatedSameOriginPath(node);
      const resolvedContextId = contextId || (
        (semantic === 'conversation' || semantic === 'conversation_options') && /^\/c\//.test(path)
          ? path.slice(3)
          : ''
      );
      const id = controlId(semantic, node, label, region, used, resolvedContextId);
      const rect = node.getBoundingClientRect();
      const form = formAdapter && formAdapter.describe(node);
      const disclosure = disclosureAdapter && disclosureAdapter.describe(node);
      used.add(id);
      controlsById.set(id, node);
      target.push({
        id,
        semantic,
        label,
        region,
        role: roleOf(node),
        enabled: !node.matches(':disabled') && node.getAttribute('aria-disabled') !== 'true',
        selected: form ? form.selected :
          node.getAttribute('aria-selected') === 'true' || node.getAttribute('aria-checked') === 'true',
        inputKind: form && form.inputKind || undefined,
        writable: !!(form && form.writable),
        stateSettable: !!(form && form.stateSettable),
        choiceLabels: form && form.choiceLabels.length ? form.choiceLabels : undefined,
        selectedChoiceIndex: form && form.selectedChoiceIndex >= 0
          ? form.selectedChoiceIndex
          : undefined,
        sliderSettable: !!(form && form.sliderSettable),
        sliderMin: form && form.sliderSettable ? form.sliderMin : undefined,
        sliderMax: form && form.sliderSettable ? form.sliderMax : undefined,
        sliderStep: form && form.sliderSettable ? form.sliderStep : undefined,
        sliderValue: form && form.sliderSettable ? form.sliderValue : undefined,
        expanded: disclosure ? disclosure.expanded : undefined,
        expandable: !!(disclosure && disclosure.expandable),
        contextId: resolvedContextId || undefined,
        inViewport: isInViewport(rect),
        xRatio: (rect.left + rect.width / 2) / Math.max(1, window.innerWidth),
        yRatio: (rect.top + rect.height / 2) / Math.max(1, window.innerHeight)
      });
    });
  }

  function messageNodes() {
    const main = document.querySelector('main');
    if (!main) return [];
    const turns = Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]'));
    return turns.length ? turns : Array.from(main.querySelectorAll('[data-message-author-role]'));
  }

  function messageContextId(node, index) {
    return String(
      node.getAttribute('data-message-id')
      || node.getAttribute('data-testid')
      || node.id
      || 'message-' + index
    ).replace(/[^A-Za-z0-9_.:-]/g, '_').slice(0, 160);
  }

  function addMessageControls(target, used) {
    messageNodes().slice(-24).forEach((turn, index) => {
      const content = turn.querySelector('.markdown, [data-message-content], .whitespace-pre-wrap');
      const contextId = messageContextId(turn, index);
      addRegionControls(target, turn, 'message', used, (node) => {
        if (content && content.contains(node) && node.matches('a[href]')) return false;
        return node.matches('button, [role="button"], [role="menuitem"]');
      }, contextId);
    });
  }

  function suggestionRoot() {
    if (pageKind() !== 'home' || messageNodes().length) return null;
    const composer = composerRoot();
    const main = document.querySelector('main, #main');
    if (!main) return null;
    const candidates = actionableNodes(main).filter((node) => {
      if (composer && composer.contains(node)) return false;
      if (headerRoot() && headerRoot().contains(node)) return false;
      const label = cleanText(node.textContent);
      const rect = node.getBoundingClientRect();
      return label.length >= 2 && label.length <= 120 && rect.top > window.innerHeight * 0.45;
    });
    return candidates.length ? main : null;
  }

  function pageContentRoot() {
    if (pageKind() !== 'feature') return null;
    return document.querySelector('main, #main');
  }

  function addPageContentControls(target, used, excludedRoots) {
    const content = pageContentRoot();
    if (!content) return;
    const turns = messageNodes();
    addRegionControls(target, content, 'content', used, (node) => {
      if (excludedRoots.some((root) => root && root.contains(node))) return false;
      if (turns.some((turn) => turn.contains(node))) return false;
      if (node.closest('aside, nav, [role="navigation"]')) return false;
      return true;
    });
  }

  function discover() {
    controlsById = new Map();
    const controls = [];
    const used = new Set();
    const header = headerRoot();
    const composer = composerRoot();
    addRegionControls(controls, header, 'header', used);
    const suggestions = suggestionRoot();
    addRegionControls(controls, suggestions, 'suggestions', used, (node) => {
      if (composer && composer.contains(node)) return false;
      if (header && header.contains(node)) return false;
      const label = cleanText(node.textContent);
      return label.length >= 2 && label.length <= 120 && node.getBoundingClientRect().top > window.innerHeight * 0.45;
    });
    addRegionControls(controls, composer, 'composer', used);
    const overlays = Array.from(document.querySelectorAll('[role="dialog"], [role="menu"]')).filter(isVisible);
    const overlay = overlays[overlays.length - 1];
    addRegionControls(controls, overlay, 'overlay', used);
    addMessageControls(controls, used);
    addPageContentControls(controls, used, [header, composer, suggestions].concat(overlays));
    return {
      controls: controls.slice(0, MAX_DISCOVERED_CONTROLS),
      totalCount: controls.length,
      truncated: controls.length > MAX_DISCOVERED_CONTROLS
    };
  }

  function pageKind() {
    if (/^\/(auth|cdn-cgi)/.test(location.pathname)) return 'auth';
    if (/^\/c\//.test(location.pathname)) return 'conversation';
    if (location.pathname === '/' || location.pathname === '') return 'home';
    return 'feature';
  }

  function pageTitle(controls) {
    const title = controls.find((item) => item.region === 'header' && item.semantic === 'title');
    return title ? title.label : cleanText(document.title.replace(/\s*[-|]\s*ChatGPT.*$/i, '')) || 'ChatGPT';
  }

  function compatibilityFor(controls, kind) {
    const hasHeader = controls.some((item) => item.region === 'header');
    const hasComposer = !!composerNode();
    const hasFeatureContent = kind === 'feature'
      && controls.some((item) => item.region === 'content');
    if ((hasHeader && (hasComposer || kind === 'auth' || kind === 'feature')) || hasFeatureContent) {
      return 'healthy';
    }
    return hasHeader || hasComposer || controls.length > 0 ? 'partial' : 'fallback_required';
  }

  function snapshot() {
    const discovery = discover();
    const controls = discovery.controls;
    const kind = pageKind();
    return {
      type: 'ui_manifest_snapshot',
      version: 8,
      pageKind: kind,
      title: pageTitle(controls),
      compatibility: compatibilityFor(controls, kind),
      discoveredControlCount: discovery.totalCount,
      controlsTruncated: discovery.truncated,
      controls
    };
  }

  function manifestFingerprint(event) {
    return JSON.stringify({
      version: event.version,
      pageKind: event.pageKind,
      title: event.title,
      compatibility: event.compatibility,
      discoveredControlCount: event.discoveredControlCount,
      controlsTruncated: event.controlsTruncated,
      controls: event.controls.map((control) => ({
        id: control.id,
        semantic: control.semantic,
        label: control.label,
        region: control.region,
        role: control.role,
        enabled: control.enabled,
        selected: control.selected,
        inputKind: control.inputKind,
        writable: control.writable,
        stateSettable: control.stateSettable,
        choiceLabels: control.choiceLabels,
        selectedChoiceIndex: control.selectedChoiceIndex,
        sliderSettable: control.sliderSettable,
        sliderMin: control.sliderMin,
        sliderMax: control.sliderMax,
        sliderStep: control.sliderStep,
        sliderValue: control.sliderValue,
        expanded: control.expanded,
        expandable: control.expandable,
        contextId: control.contextId,
        inViewport: control.inViewport
      }))
    });
  }

  function emitSnapshot(emitEvent, force) {
    const event = snapshot();
    const fingerprint = manifestFingerprint(event);
    if (!force && fingerprint === lastFingerprint) return;
    lastFingerprint = fingerprint;
    emitEvent(event);
  }

  function resolveSemanticControl(semantic, region) {
    const discovery = discover();
    const control = discovery.controls.find((candidate) =>
      candidate.semantic === semantic && candidate.enabled && candidate.inViewport &&
        (!region || candidate.region === region)
    );
    const node = control && controlsById.get(control.id);
    return node && isVisible(node) ? { control, node } : null;
  }

  function findSemanticNode(semantic, region) {
    const resolved = resolveSemanticControl(semantic, region);
    return resolved && resolved.node;
  }

  function requestSemanticTouch(semantic, purpose, emitEvent, region) {
    const resolved = resolveSemanticControl(semantic, region);
    if (!resolved) return false;
    const { control } = resolved;
    emitEvent({
      type: 'web_touch_request',
      purpose,
      controlId: control.id,
      xRatio: control.xRatio,
      yRatio: control.yRatio
    });
    return true;
  }

  function invoke(id, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node)) return result('invoke_ui_control', false, '官网控件已变化，请刷新结构后重试。');
    function dispatch() {
      if (!node.isConnected || !isVisible(node)) {
        return result('invoke_ui_control', false, '官网控件已变化，请刷新结构后重试。');
      }
      const rect = node.getBoundingClientRect();
      const xRatio = (rect.left + rect.width / 2) / Math.max(1, window.innerWidth);
      const yRatio = (rect.top + rect.height / 2) / Math.max(1, window.innerHeight);
      if (!isInViewport(rect) || xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) {
        return result('invoke_ui_control', false, '官网控件滚动后仍不在可操作区域。');
      }
      emitEvent({ type: 'web_touch_request', purpose: 'invoke_ui_control', controlId: id, xRatio, yRatio });
      result('invoke_ui_control', true, '');
      window.setTimeout(() => emitSnapshot(emitEvent, true), 180);
    }
    const rect = node.getBoundingClientRect();
    if (isInViewport(rect)) return dispatch();
    node.scrollIntoView({ block: 'center', inline: 'nearest' });
    window.setTimeout(dispatch, 120);
  }

  function setText(id, value, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node)) {
      return result('set_ui_control_text', false, '官网文本框已变化，请刷新结构后重试。');
    }
    if (!formAdapter) return result('set_ui_control_text', false, '官网表单适配器尚未就绪。');
    const update = formAdapter.setText(node, value);
    if (!update.ok) {
      const detail = update.reason === 'sensitive'
        ? '密码和登录凭证只允许在官方网页中输入。'
        : '该官网控件不是可写文本框。';
      return result('set_ui_control_text', false, detail);
    }
    result('set_ui_control_text', true, '');
    window.setTimeout(() => emitSnapshot(emitEvent, true), 180);
  }

  function setSelected(id, selected, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node) || !formCommands) {
      return result('set_ui_control_selected', false, '官网控件已变化，请刷新结构后重试。');
    }
    return formCommands.setSelected(
      node, id, selected, formAdapter, emitEvent, result,
      () => emitSnapshot(emitEvent, true)
    );
  }

  function selectChoice(id, choiceIndex, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node) || !formCommands) {
      return result('select_ui_control_choice', false, '官网控件已变化，请刷新结构后重试。');
    }
    return formCommands.selectChoice(
      node, choiceIndex, formAdapter, result,
      () => emitSnapshot(emitEvent, true)
    );
  }

  function setSliderValue(id, value, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node) || !formCommands) {
      return result('set_ui_control_slider', false, '官网滑块已变化，请刷新结构后重试。');
    }
    return formCommands.setSliderValue(
      node, value, formAdapter, result,
      () => emitSnapshot(emitEvent, true)
    );
  }

  function setExpanded(id, expanded, emitEvent, result) {
    discover();
    const node = controlsById.get(String(id || ''));
    if (!node || !isVisible(node) || !disclosureAdapter) {
      return result('set_ui_control_expanded', false, '官网控件已变化，请刷新结构后重试。');
    }
    return disclosureAdapter.setExpanded(
      node, id, expanded, emitEvent, result,
      () => emitSnapshot(emitEvent, true)
    );
  }

  function setNodeExpanded(node, expanded, emitEvent, result) {
    if (!node || !isVisible(node) || !disclosureAdapter) {
      return result('set_ui_control_expanded', false, '官网控件已变化，请刷新结构后重试。');
    }
    const entry = Array.from(controlsById.entries()).find(([, candidate]) => candidate === node);
    const stableId = entry ? entry[0] : 'control_disclosure_' + hash([
      node.id,
      node.getAttribute('data-testid'),
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      labelOf(node, '展开选项')
    ].filter(Boolean).join('|'));
    return disclosureAdapter.setExpanded(
      node, stableId, expanded, emitEvent, result,
      () => emitSnapshot(emitEvent, true)
    );
  }

  window.__elonChatGptLayout = Object.freeze({
    emitSnapshot,
    findSemanticNode,
    invoke,
    pageKind,
    requestSemanticTouch,
    selectChoice,
    setExpanded,
    setNodeExpanded,
    setSelected,
    setSliderValue,
    setText
  });
})();
