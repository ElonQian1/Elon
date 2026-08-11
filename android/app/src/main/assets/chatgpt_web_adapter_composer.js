(function () {
  'use strict';

  if (window.__elonChatGptComposer || location.origin !== 'https://chatgpt.com') return;

  const MAX_OPTIONS = 30;
  const optionPolicy = window.__elonChatGptComposerOptionPolicy;
  const actionTargetPolicy = window.__elonChatGptActionTargetPolicy;
  let lastOptions = { model: [], tools: [] };
  let pendingOptions = { model: null, tools: null };
  let lastAttachments = [];

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function composerScope(composer) {
    return (composer && composer.closest('form')) ||
      (composer && composer.closest('#thread-bottom')) ||
      document.querySelector('#thread-bottom') ||
      document;
  }

  function nodeLabel(node) {
    return cleanText([
      node && node.getAttribute('aria-label'),
      node && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' '));
  }

  function findAttachmentInput() {
    const candidates = [
      document.querySelector('#upload-fast-tools-files'),
      document.querySelector('input[type="file"][data-testid*="upload" i]'),
      document.querySelector('input[type="file"]')
    ];
    return candidates.find((node) => node && !node.disabled) || null;
  }

  function attachmentState(node) {
    const value = nodeLabel(node).toLowerCase();
    if (/failed|error|失败|错误/.test(value)) return 'error';
    if (node.querySelector('[role="progressbar"]') || /uploading|处理中|上传中/.test(value)) {
      return 'uploading';
    }
    return 'ready';
  }

  function attachmentName(node, fallback) {
    const named = node.querySelector(
      '[data-testid*="file-name" i], [data-testid*="filename" i], [title], img[alt]'
    );
    const raw = cleanText([
      named && named.getAttribute('title'),
      named && named.getAttribute('alt'),
      named && named.textContent,
      node.getAttribute('title'),
      node.textContent
    ].filter(Boolean).join(' '));
    const withoutActions = raw.replace(
      /\b(remove|delete|download|preview)\b|移除附件|删除附件|删除文件|下载|预览/gi,
      ' '
    );
    return cleanText(withoutActions).slice(0, 180) || fallback;
  }

  function readAttachments(composer) {
    const scope = composerScope(composer);
    const seenNodes = new Set();
    const candidates = [];
    const removeLabels = /remove (file|attachment)|delete (file|attachment)|移除附件|删除附件|删除文件/i;
    Array.from(scope.querySelectorAll('button')).filter((button) =>
      removeLabels.test(nodeLabel(button))
    ).forEach((button) => {
      const container = button.closest(
        '[data-testid*="attachment" i], [data-testid*="file" i], li, [role="listitem"]'
      ) || button.parentElement;
      if (container && isVisible(container) && !seenNodes.has(container)) {
        seenNodes.add(container);
        candidates.push({ container, remove: button });
      }
    });
    Array.from(scope.querySelectorAll(
      '[data-testid*="attachment" i], [data-testid*="file-pill" i], [data-testid*="upload-preview" i]'
    )).filter(isVisible).forEach((container) => {
      if (!seenNodes.has(container)) {
        seenNodes.add(container);
        const remove = Array.from(container.querySelectorAll('button')).find((button) =>
          removeLabels.test(nodeLabel(button))
        ) || null;
        candidates.push({ container, remove });
      }
    });
    const occurrences = new Map();
    const attachments = candidates.map(({ container, remove }, index) => {
      const name = attachmentName(container, '附件 ' + (index + 1));
      const occurrence = occurrences.get(name) || 0;
      occurrences.set(name, occurrence + 1);
      return {
        id: optionId('attachment', name, occurrence),
        name,
        state: attachmentState(container),
        removable: !!remove,
        node: remove
      };
    });
    const input = findAttachmentInput();
    if (!attachments.length && input && input.files) {
      Array.from(input.files).slice(0, 10).forEach((file, index) => {
        const name = cleanText(file.name).slice(0, 180) || '附件 ' + (index + 1);
        attachments.push({
          id: optionId('attachment', name, index),
          name,
          state: 'uploading',
          removable: false,
          node: null
        });
      });
    }
    lastAttachments = attachments.slice(0, 10);
    return lastAttachments.map(({ id, name, state, removable }) => ({ id, name, state, removable }));
  }

  function findToolsButton(composer) {
    const scope = composerScope(composer);
    const selectors = [
      '#composer-plus-btn',
      '[data-testid="composer-plus-btn"]',
      'button[aria-label*="tool" i]',
      'button[aria-label*="工具"]',
      'button[aria-label*="add" i]'
    ];
    for (const selector of selectors) {
      const node = scope.querySelector(selector) || document.querySelector(selector);
      if (isActionable(node)) return node;
    }
    const layout = window.__elonChatGptLayout;
    const semanticNode = layout && typeof layout.findSemanticNode === 'function'
      ? layout.findSemanticNode('attachment', 'composer')
      : null;
    if (isVisible(semanticNode)) return semanticNode;
    return null;
  }

  function isComposerAction(button) {
    const id = String(button.id || button.getAttribute('data-testid') || '').toLowerCase();
    const label = nodeLabel(button).toLowerCase();
    return id.includes('send') || id.includes('submit') || id.includes('stop') ||
      id.includes('plus') || id.includes('upload') || label.includes('send') ||
      label.includes('发送') || label.includes('dictation') || label.includes('听写') ||
      label.includes('语音');
  }

  function findModelButton(composer) {
    const scope = composerScope(composer);
    const directSelectors = [
      '[data-testid*="model-selector" i]',
      '[aria-label*="model selector" i]',
      '[aria-label*="选择模型"]'
    ];
    for (const selector of directSelectors) {
      const node = scope.querySelector(selector) || document.querySelector(selector);
      if (isVisible(node)) return node;
    }
    const layout = window.__elonChatGptLayout;
    const semanticNode = layout && typeof layout.findSemanticNode === 'function'
      ? layout.findSemanticNode('model', 'composer')
      : null;
    if (isVisible(semanticNode)) return semanticNode;
    const candidates = Array.from(scope.querySelectorAll('button, [role="button"]')).filter((button) => {
      const label = nodeLabel(button);
      return isActionable(button) && !isComposerAction(button) && label.length > 0 && label.length <= 80;
    });
    return candidates.find((button) => button.getAttribute('aria-haspopup') === 'menu') ||
      candidates.find((button) => /gpt|\bo\d|auto|thinking|instant|sol|模型|能力|轻度|重度/i.test(nodeLabel(button))) ||
      null;
  }

  function findDictationSessionButton(kind) {
    const labels = kind === 'cancel'
      ? ['cancel dictation', '取消听写']
      : ['submit dictation', '提交听写'];
    return Array.from(document.querySelectorAll('button, [role="button"]')).find((button) => {
      if (!isActionable(button)) return false;
      const label = nodeLabel(button).toLowerCase();
      return labels.some((candidate) => label.includes(candidate));
    }) || null;
  }

  function findDictationButton(composer) {
    const scope = composerScope(composer);
    return Array.from(scope.querySelectorAll('button')).find((button) => {
      if (!isActionable(button)) return false;
      const label = nodeLabel(button);
      return /dictation|听写|语音输入/i.test(label) &&
        !/cancel dictation|submit dictation|取消听写|提交听写/i.test(label);
    }) || null;
  }

  function dictationActive(composer) {
    if (findDictationSessionButton('cancel') || findDictationSessionButton('submit')) return true;
    const button = findDictationButton(composer);
    if (!button) return false;
    const label = nodeLabel(button);
    return button.getAttribute('aria-pressed') === 'true' || /stop|end|停止|结束/i.test(label);
  }

  function capabilities(composer) {
    const values = [];
    if (findAttachmentInput()) values.push('attachments');
    if (findModelButton(composer)) values.push('model_selector');
    if (findToolsButton(composer)) values.push('composer_tools');
    if (findDictationButton(composer) || dictationActive(composer)) values.push('dictation');
    return values;
  }

  function currentModel(composer) {
    const button = findModelButton(composer);
    const visibleLabel = cleanText(button && button.textContent);
    return (visibleLabel || nodeLabel(button)).slice(0, 80);
  }

  function optionId(section, label, occurrence) {
    let hash = 2166136261;
    const source = section + ':' + label.toLowerCase() + ':' + occurrence;
    for (let index = 0; index < source.length; index += 1) {
      hash ^= source.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return section + '_' + (hash >>> 0).toString(36);
  }

  function optionSemantic(section, node, label) {
    if (section === 'model') return 'model';
    const signal = cleanText([
      node && node.id,
      node && node.getAttribute('data-testid'),
      node && node.getAttribute('aria-label'),
      label
    ].filter(Boolean).join(' ')).toLowerCase();
    if (/deep.?research|深度研究/.test(signal)) return 'deep_research';
    if (/create.?image|image.?generation|生成图片|创建图片/.test(signal)) return 'image_generation';
    if (/canvas|画布/.test(signal)) return 'canvas';
    if (/study|学习/.test(signal)) return 'study';
    if (/agent|代理模式|智能体/.test(signal)) return 'agent';
    if (/camera|take.?photo|相机|拍照/.test(signal)) return 'attachment_camera';
    if (/photos?|gallery|照片|相册/.test(signal)) return 'attachment_photos';
    if (/(^|[\s_/-])(upload|files?)($|[\s_/-])|文件|上传/.test(signal)) {
      return 'attachment_file';
    }
    if (/(^|[\s_-])search($|[\s_-])|web.*search|search.*web|browse|搜索/.test(signal)) {
      return 'web_search';
    }
    return 'tool';
  }

  function visibleOptionNodes() {
    return Array.from(document.querySelectorAll(
      '[role="menuitemradio"], [role="menuitemcheckbox"], [role="menuitem"], [role="option"]'
    )).filter(isOptionVisible);
  }

  function isActionable(node) {
    return !!(actionTargetPolicy && actionTargetPolicy.actionPoint(node));
  }

  function isOptionVisible(node) {
    const actionable = isActionable(node);
    if (actionable || !isVisible(node)) return actionable;
    const rect = node.getBoundingClientRect();
    return rect.bottom > 0 && rect.right > 0 &&
      rect.top < window.innerHeight && rect.left < window.innerWidth;
  }

  function captureOptionBaseline() {
    const baseline = new Map();
    visibleOptionNodes().forEach((node) => baseline.set(node, actionTargetPolicy.signature(node)));
    return baseline;
  }

  function isNewOrChangedOption(node, baseline) {
    return !baseline || !baseline.has(node) ||
      baseline.get(node) !== actionTargetPolicy.signature(node);
  }

  function opensSubmenu(node) {
    const popup = String(node && node.getAttribute('aria-haspopup') || '').toLowerCase();
    return popup === 'menu' || popup === 'listbox' ||
      !!(node && node.hasAttribute('aria-expanded'));
  }

  function collectOptions(section, baseline) {
    const seen = new Map();
    const candidates = visibleOptionNodes().filter((node) => isNewOrChangedOption(node, baseline)).map((node) => {
      const label = nodeLabel(node).slice(0, 120);
      if (!label) return null;
      const occurrence = seen.get(label) || 0;
      seen.set(label, occurrence + 1);
      const role = String(node.getAttribute('role') || 'menuitem').slice(0, 32);
      const selected = node.getAttribute('aria-checked') === 'true' ||
        node.getAttribute('aria-selected') === 'true' ||
        node.getAttribute('data-state') === 'checked';
      return {
        id: optionId(section, label, occurrence),
        label,
        selected,
        kind: role,
        semantic: optionSemantic(section, node, label),
        role,
        opensSubmenu: opensSubmenu(node),
        selectable: selected || node.hasAttribute('aria-checked') || node.hasAttribute('aria-selected'),
        node
      };
    }).filter(Boolean);
    if (!optionPolicy || typeof optionPolicy.filter !== 'function') return [];
    return optionPolicy.filter(section, candidates).slice(0, MAX_OPTIONS);
  }

  function waitForOptions(section, baseline, onReady, onTimeout) {
    const started = Date.now();
    function poll() {
      const options = collectOptions(section, baseline);
      if (options.length > 0) return onReady(options);
      if (Date.now() - started > 1600) return onTimeout();
      window.setTimeout(poll, 60);
    }
    poll();
  }

  function triggerFor(section, composer) {
    return section === 'model' ? findModelButton(composer) : findToolsButton(composer);
  }

  function emitOptions(section, options, composer, emitEvent) {
    lastOptions[section] = options;
    emitEvent({
      type: 'composer_controls_snapshot',
      section,
      currentModel: currentModel(composer),
      options: options.map(({ id, label, selected, kind, semantic, opensSubmenu }) => ({
        id, label, selected, kind, semantic, opensSubmenu
      }))
    });
  }

  function emitTouchRequest(purpose, node, emitEvent) {
    const point = actionTargetPolicy && actionTargetPolicy.actionPoint(node);
    if (!point) return false;
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);
    const xRatio = point.x / width;
    const yRatio = point.y / height;
    if (xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) return false;
    emitEvent({ type: 'web_touch_request', purpose, xRatio, yRatio });
    return true;
  }

  function emitVisibleNodeTouch(purpose, node, emitEvent) {
    if (emitTouchRequest(purpose, node, emitEvent)) return true;
    if (!isOptionVisible(node)) return false;
    const rect = node.getBoundingClientRect();
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);
    const xRatio = (rect.left + rect.width / 2) / width;
    const yRatio = (rect.top + rect.height / 2) / height;
    if (xRatio < 0 || xRatio > 1 || yRatio < 0 || yRatio > 1) return false;
    emitEvent({ type: 'web_touch_request', purpose, xRatio, yRatio });
    return true;
  }

  function emitTriggerTouch(section, purpose, node, emitEvent) {
    return emitVisibleNodeTouch(purpose, node, emitEvent);
  }

  function replacePendingOptions(section, pending) {
    const previous = pendingOptions[section];
    pendingOptions[section] = pending;
    if (previous && typeof previous.complete === 'function') {
      previous.complete(previous.action, false, '菜单请求已被新的操作替代。');
    }
  }

  function settlePendingOptions(section, ok, detail) {
    const pending = pendingOptions[section];
    pendingOptions[section] = null;
    if (pending && typeof pending.complete === 'function') {
      pending.complete(pending.action, ok, detail || '');
    }
  }

  function requestOptions(section, composer, emitEvent, result) {
    const action = section === 'model' ? 'list_model_options' : 'list_composer_tools';
    const reusable = lastOptions[section].filter((option) => isOptionVisible(option.node));
    if (reusable.length > 0) {
      emitOptions(section, reusable, composer, emitEvent);
      return result(action, true, '');
    }
    const alreadyOpen = collectOptions(section, null);
    if (alreadyOpen.length > 0) {
      emitOptions(section, alreadyOpen, composer, emitEvent);
      return result(action, true, '');
    }
    const trigger = triggerFor(section, composer);
    if (!trigger) return result(action, false, '官网当前没有可用入口。');
    replacePendingOptions(section, {
      baseline: captureOptionBaseline(),
      trigger,
      action,
      complete: result
    });
    if (!emitTriggerTouch(section, action, trigger, emitEvent)) {
      return settlePendingOptions(section, false, '官网入口当前不可见。');
    }
  }

  function collectRequestedOptions(section, composer, emitEvent, result) {
    const action = section === 'model' ? 'collect_model_options' : 'collect_composer_tools';
    const pending = pendingOptions[section];
    if (!pending) return result(action, false, '没有等待读取的官网菜单。');
    function collect() {
      waitForOptions(
        section,
        pending.baseline,
        (options) => {
          if (pendingOptions[section] !== pending) return;
          emitOptions(section, options, composer, emitEvent);
          settlePendingOptions(section, true, '');
        },
        () => {
          if (pendingOptions[section] !== pending) return;
          if (
            !pending.syntheticRetried &&
            pending.trigger && pending.trigger.isConnected && isVisible(pending.trigger)
          ) {
            pending.syntheticRetried = true;
            pending.baseline = captureOptionBaseline();
            pending.trigger.click();
            return collect();
          }
          settlePendingOptions(section, false, '官网菜单尚未返回可用选项，请切换官方网页。');
        }
      );
    }
    collect();
  }

  function selectOption(section, id, composer, emitEvent, result, scheduleSnapshot) {
    const action = section === 'model' ? 'select_model_option' : 'select_composer_tool';
    if (!lastOptions[section].some((option) => option.id === id)) {
      return result(action, false, '选项已过期，请重新打开列表。');
    }
    const target = lastOptions[section].find((option) => option.id === id);
    if (!target || !isOptionVisible(target.node)) {
      return result(action, false, '官网菜单已经关闭，请重新选择。');
    }
    const submenuPurpose = section === 'model' ? 'open_model_submenu' : 'open_composer_tools_submenu';
    const purpose = target.opensSubmenu ? submenuPurpose : action;
    if (target.opensSubmenu) {
      replacePendingOptions(section, {
        baseline: captureOptionBaseline(),
        trigger: target.node,
        action,
        complete: result
      });
    }
    if (!emitVisibleNodeTouch(purpose, target.node, emitEvent)) {
      if (target.opensSubmenu) {
        return settlePendingOptions(section, false, '官网选项当前不可见。');
      }
      return result(action, false, '官网选项当前不可见。');
    }
    if (!target.opensSubmenu) {
      result(action, true, '');
      window.setTimeout(scheduleSnapshot, 240);
    }
  }

  function ownsOptionNode(node) {
    return ['model', 'tools'].some((section) =>
      lastOptions[section].some((option) => option.node === node && isOptionVisible(option.node))
    );
  }

  function startDictation(composer, emitEvent, result) {
    const button = findDictationButton(composer);
    if (!button) return result('start_dictation', false, '官网当前没有听写入口。');
    if (!emitTouchRequest('start_dictation', button, emitEvent)) {
      return result('start_dictation', false, '官网听写入口当前不可见。');
    }
    result('start_dictation', true, '');
  }

  function finishDictation(kind, emitEvent, result) {
    const action = kind === 'cancel' ? 'cancel_dictation' : 'submit_dictation';
    const button = findDictationSessionButton(kind);
    if (!button) return result(action, false, '官网当前没有进行中的听写。');
    if (!emitTouchRequest(action, button, emitEvent)) {
      return result(action, false, '官网听写操作当前不可见。');
    }
    result(action, true, '');
  }

  function removeAttachment(id, emitEvent, result) {
    const attachment = lastAttachments.find((item) => item.id === id);
    if (!attachment || !attachment.removable || !isVisible(attachment.node)) {
      return result('remove_attachment', false, '附件状态已变化，请刷新后重试。');
    }
    if (!emitTouchRequest('remove_attachment', attachment.node, emitEvent)) {
      return result('remove_attachment', false, '官网附件移除入口当前不可见。');
    }
    result('remove_attachment', true, '');
  }

  function openOfficial(section, composer, emitEvent, result) {
    const action = section === 'model' ? 'open_model_selector' : 'open_composer_tools';
    const trigger = triggerFor(section, composer);
    if (!trigger) return result(action, false, '官网当前没有可用入口。');
    if (!emitTriggerTouch(section, action, trigger, emitEvent)) {
      return result(action, false, '官网入口当前不可见。');
    }
    result(action, true, '');
  }

  function dismissOpenMenu(result) {
    const target = document.activeElement || document;
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
    target.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', code: 'Escape', bubbles: true }));
    lastOptions = { model: [], tools: [] };
    settlePendingOptions('model', false, '官网菜单已关闭。');
    settlePendingOptions('tools', false, '官网菜单已关闭。');
    result('dismiss_composer_menu', true, '');
  }

  window.__elonChatGptComposer = Object.freeze({
    capabilities,
    currentModel,
    readAttachments,
    dictationActive,
    requestOptions,
    collectRequestedOptions,
    ownsOptionNode,
    selectOption,
    startDictation,
    cancelDictation: (emitEvent, result) => finishDictation('cancel', emitEvent, result),
    submitDictation: (emitEvent, result) => finishDictation('submit', emitEvent, result),
    removeAttachment,
    openOfficial,
    dismissOpenMenu
  });
})();
