(function () {
  'use strict';

  if (window.__elonChatGptComposer || location.origin !== 'https://chatgpt.com') return;

  const MAX_OPTIONS = 30;
  let lastOptions = { model: [], tools: [] };

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
      if (isVisible(node)) return node;
    }
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
    const candidates = Array.from(scope.querySelectorAll('button')).filter((button) => {
      const label = nodeLabel(button);
      return isVisible(button) && !isComposerAction(button) && label.length > 0 && label.length <= 80;
    });
    return candidates.find((button) => button.getAttribute('aria-haspopup') === 'menu') ||
      candidates.find((button) => /gpt|\bo\d|auto|thinking|instant|sol|模型|能力|轻度|重度/i.test(nodeLabel(button))) ||
      null;
  }

  function findDictationButton(composer) {
    const scope = composerScope(composer);
    return Array.from(scope.querySelectorAll('button')).find((button) => {
      if (!isVisible(button)) return false;
      return /start dictation|dictation|开始听写|语音输入/i.test(nodeLabel(button));
    }) || null;
  }

  function capabilities(composer) {
    const values = [];
    if (findAttachmentInput()) values.push('attachments');
    if (findModelButton(composer)) values.push('model_selector');
    if (findToolsButton(composer)) values.push('composer_tools');
    if (findDictationButton(composer)) values.push('dictation');
    return values;
  }

  function currentModel(composer) {
    return nodeLabel(findModelButton(composer)).slice(0, 80);
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

  function visibleOptionNodes() {
    return Array.from(document.querySelectorAll(
      '[role="menuitemradio"], [role="menuitemcheckbox"], [role="menuitem"], [role="option"]'
    )).filter(isVisible);
  }

  function collectOptions(section, baseline) {
    const seen = new Map();
    return visibleOptionNodes().filter((node) => !baseline || !baseline.has(node)).map((node) => {
      const label = nodeLabel(node).slice(0, 120);
      if (!label) return null;
      const occurrence = seen.get(label) || 0;
      seen.set(label, occurrence + 1);
      return {
        id: optionId(section, label, occurrence),
        label,
        selected: node.getAttribute('aria-checked') === 'true' ||
          node.getAttribute('aria-selected') === 'true' ||
          node.getAttribute('data-state') === 'checked',
        kind: String(node.getAttribute('role') || 'menuitem').slice(0, 32),
        node
      };
    }).filter(Boolean).slice(0, MAX_OPTIONS);
  }

  function dismissMenu(trigger, openedOptions) {
    const target = document.activeElement || document;
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true }));
    target.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', code: 'Escape', bubbles: true }));
    window.setTimeout(() => {
      const stillOpen = openedOptions.some((option) => isVisible(option.node));
      if (stillOpen && trigger && isVisible(trigger)) trigger.click();
    }, 80);
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
      options: options.map(({ id, label, selected, kind }) => ({ id, label, selected, kind }))
    });
  }

  function requestOptions(section, composer, emitEvent, result) {
    const action = section === 'model' ? 'list_model_options' : 'list_composer_tools';
    const trigger = triggerFor(section, composer);
    if (!trigger) return result(action, false, '官网当前没有可用入口。');
    const baseline = new Set(visibleOptionNodes());
    trigger.click();
    waitForOptions(
      section,
      baseline,
      (options) => {
        emitOptions(section, options, composer, emitEvent);
        result(action, true, '');
        dismissMenu(trigger, options);
      },
      () => {
        dismissMenu(trigger, []);
        result(action, false, '官网菜单尚未返回可用选项，请切换官方网页。');
      }
    );
  }

  function selectOption(section, id, composer, emitEvent, result, scheduleSnapshot) {
    const action = section === 'model' ? 'select_model_option' : 'select_composer_tool';
    if (!lastOptions[section].some((option) => option.id === id)) {
      return result(action, false, '选项已过期，请重新打开列表。');
    }
    const trigger = triggerFor(section, composer);
    if (!trigger) return result(action, false, '官网当前没有可用入口。');
    const baseline = new Set(visibleOptionNodes());
    trigger.click();
    waitForOptions(
      section,
      baseline,
      (options) => {
        const target = options.find((option) => option.id === id);
        if (!target) {
          dismissMenu(trigger, options);
          return result(action, false, '官网选项已经变化，请重新选择。');
        }
        target.node.click();
        result(action, true, '');
        window.setTimeout(scheduleSnapshot, 180);
      },
      () => result(action, false, '官网菜单尚未就绪，请切换官方网页。')
    );
  }

  function chooseAttachments(result) {
    const input = findAttachmentInput();
    if (!input) return result('choose_attachments', false, '官网当前没有附件入口。');
    input.click();
    result('choose_attachments', true, '');
  }

  function startDictation(composer, result) {
    const button = findDictationButton(composer);
    if (!button) return result('start_dictation', false, '官网当前没有听写入口。');
    button.click();
    result('start_dictation', true, '');
  }

  function openOfficial(section, composer, result) {
    const action = section === 'model' ? 'open_model_selector' : 'open_composer_tools';
    const trigger = triggerFor(section, composer);
    if (!trigger) return result(action, false, '官网当前没有可用入口。');
    trigger.click();
    result(action, true, '');
  }

  window.__elonChatGptComposer = Object.freeze({
    capabilities,
    currentModel,
    chooseAttachments,
    requestOptions,
    selectOption,
    startDictation,
    openOfficial
  });
})();
