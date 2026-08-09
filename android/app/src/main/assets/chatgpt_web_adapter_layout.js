(function () {
  'use strict';

  if (window.__elonChatGptLayout || location.origin !== 'https://chatgpt.com') return;

  let controlsById = new Map();
  let lastFingerprint = '';

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

  function labelOf(node, fallback) {
    const candidates = [
      node.innerText,
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.getAttribute('data-testid')
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
    const role = String(node.getAttribute('role') || '').toLowerCase();
    if (['button', 'link', 'menuitem', 'switch', 'tab'].includes(role)) return role;
    return node.matches('a[href]') ? 'link' : 'button';
  }

  function composerNode() {
    return document.querySelector('[data-testid="prompt-textarea"], #prompt-textarea, form textarea, form [contenteditable="true"]');
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
    return Array.from(root.querySelectorAll('button, a[href], [role="button"], [role="menuitem"], [role="switch"], [role="tab"]'))
      .filter(isVisible);
  }

  function semanticFor(node, region, index) {
    const path = sameOriginPath(node);
    const signal = cleanText([
      node.id,
      node.getAttribute('data-testid'),
      node.getAttribute('aria-label'),
      node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
    if (/^\/c\/[A-Za-z0-9_-]{1,160}$/.test(path)) return 'conversation';
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
    if (/sources?|citations?|文件和来源|查看来源|来源/.test(signal)) return 'sources';
    if (/composer-plus|attach|upload|添加|附件|上传/.test(signal)) return 'attachment';
    if (/model|模型|gpt-|sol/.test(signal)) return 'model';
    if (/read.aloud|朗读/.test(signal)) return 'read_aloud';
    if (/dictat|microphone|voice|听写|麦克风|语音/.test(signal)) return 'dictation';
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
      library: '文件库',
      tasks: '任务',
      project: '项目',
      gpts: 'GPT',
      settings: '设置',
      create_asset: '创建文件或网站',
      sources: '文件和来源',
      more: '更多操作',
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
    actionableNodes(root).forEach((node, index) => {
      if (filter && !filter(node)) return;
      const semantic = semanticFor(node, region, index);
      const label = labelOf(node, defaultLabel(semantic));
      const path = sameOriginPath(node);
      const resolvedContextId = contextId || (semantic === 'conversation' ? path.slice(3) : '');
      const id = controlId(semantic, node, label, region, used, resolvedContextId);
      const rect = node.getBoundingClientRect();
      used.add(id);
      controlsById.set(id, node);
      target.push({
        id,
        semantic,
        label,
        region,
        role: roleOf(node),
        enabled: !node.matches(':disabled') && node.getAttribute('aria-disabled') !== 'true',
        selected: node.getAttribute('aria-selected') === 'true' || node.getAttribute('aria-checked') === 'true',
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
    const overlay = Array.from(document.querySelectorAll('[role="dialog"], [role="menu"]')).find(isVisible);
    addRegionControls(controls, overlay, 'overlay', used);
    addMessageControls(controls, used);
    return controls.slice(0, 160);
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

  function snapshot() {
    const controls = discover();
    const hasHeader = controls.some((item) => item.region === 'header');
    const hasComposer = !!composerNode();
    return {
      type: 'ui_manifest_snapshot',
      version: 3,
      pageKind: pageKind(),
      title: pageTitle(controls),
      compatibility: hasHeader && (hasComposer || pageKind() === 'auth')
        ? 'healthy'
        : hasHeader || hasComposer ? 'partial' : 'fallback_required',
      controls
    };
  }

  function manifestFingerprint(event) {
    return JSON.stringify({
      version: event.version,
      pageKind: event.pageKind,
      title: event.title,
      compatibility: event.compatibility,
      controls: event.controls.map((control) => ({
        id: control.id,
        semantic: control.semantic,
        label: control.label,
        region: control.region,
        role: control.role,
        enabled: control.enabled,
        selected: control.selected,
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

  window.__elonChatGptLayout = Object.freeze({ emitSnapshot, invoke });
})();
