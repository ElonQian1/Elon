(function (root, factory) {
  'use strict';

  const adapter = factory(root);
  if (typeof module === 'object' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptComposerToolSelection = Object.freeze(adapter);
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';

  const MAX_OBSERVATION_ATTEMPTS = 24;
  const REQUIRED_CONFIRMATIONS = 4;
  const OBSERVATION_INTERVAL_MS = 120;
  const MAX_TOUCH_ATTEMPTS = 2;

  function toolLabel(context) {
    return String(context && context.toolLabel || '官网工具');
  }

  function completeWhenObserved(context, attempt, confirmations, touchAttempt) {
    const menuSettled = context.menuSettled();
    const optionSelection = context.directSelection(context.optionNode);
    const composerSelection = context.composerSelection();
    const observedSelection = !menuSettled && optionSelection.known
      ? optionSelection
      : composerSelection;
    const observed = observedSelection.known &&
      observedSelection.selected === context.desiredSelected;
    const nextConfirmations = observed ? confirmations + 1 : 0;
    if (nextConfirmations >= REQUIRED_CONFIRMATIONS) {
      return context.complete(true, '');
    }
    if (
      !menuSettled && optionSelection.known && !observed &&
      attempt >= REQUIRED_CONFIRMATIONS
    ) {
      if (touchAttempt >= MAX_TOUCH_ATTEMPTS || !context.retryTouch(context.optionNode)) {
        return context.complete(false, toolLabel(context) + '状态未发生预期变化。');
      }
      window.setTimeout(
        () => completeWhenObserved(context, 1, 0, touchAttempt + 1),
        OBSERVATION_INTERVAL_MS
      );
      return;
    }
    if (
      (menuSettled && !composerSelection.known && attempt >= REQUIRED_CONFIRMATIONS) ||
      attempt >= MAX_OBSERVATION_ATTEMPTS
    ) {
      return verifyInMenu(context, touchAttempt);
    }
    window.setTimeout(
      () => completeWhenObserved(context, attempt + 1, nextConfirmations, touchAttempt),
      OBSERVATION_INTERVAL_MS
    );
  }

  function verifyInMenu(context, touchAttempt) {
    context.openVerificationMenu((options) => {
      const target = options.find((option) => option.semantic === context.semantic);
      if (!target || !target.directStateKnown) {
        return context.complete(false, '官网没有提供可验证的' + toolLabel(context) + '状态。');
      }
      if (target.selected === context.desiredSelected) return context.complete(true, '');
      if (touchAttempt >= MAX_TOUCH_ATTEMPTS || !context.retryTouch(target.node)) {
        return context.complete(false, toolLabel(context) + '状态未发生预期变化。');
      }
      const retryContext = Object.assign({}, context, {
        optionNode: target.node,
        menuSettled: () => context.menuSettledFor(target.node)
      });
      window.setTimeout(
        () => completeWhenObserved(retryContext, 1, 0, touchAttempt + 1),
        OBSERVATION_INTERVAL_MS
      );
    }, () => context.complete(false, toolLabel(context) + '状态无法复核。'));
  }

  function select(context) {
    completeWhenObserved(context, 1, 0, 1);
  }

  function createPrivateRuntime(page, options = {}) {
    const RUNTIME_URL = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
    const PREFIX = 'private_tool_';
    const TOOLS = [
      { hint: 'search', semantic: 'web_search', label: '网页搜索' },
      { hint: 'picture_v2', semantic: 'image_generation', label: '创建图片' }
    ];
    let namespace, loading, cooldown = 0, serial = 0, catalog = null, pending = null, receipt = null;

    function identity() {
      const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
      const normalized = {};
      for (const [key, value] of Object.entries(headers || {})) normalized[key.toLowerCase()] = value;
      if (!/^Bearer\s+\S{8,65536}$/.test(normalized.authorization || '')) return null;
      return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => normalized[key] || ''));
    }

    function capture() {
      const url = new URL(page.location.href);
      const conversationId = /^(?:\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname)?.[1] || null;
      const project = /^\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?\/project$/i.test(url.pathname);
      const temporary = url.search === '?temporary-chat=true';
      if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
          url.search && !temporary || url.pathname.startsWith('/g/') && temporary ||
          url.pathname !== '/' && !conversationId && !project) return null;
      const token = page.__elonChatGptDocumentToken, account = identity();
      if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '') || !account) return null;
      const node = page.document.querySelector('#composer-plus-btn');
      if (!node?.isConnected) return null;
      const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
      const candidates = new Map();
      // Follow only the committed composer ancestors, never a stale alternate or hooks.
      for (const start of [node[key], node[key]?.alternate]) {
        const ancestors = [];
        for (let fiber = start; fiber && ancestors.length < 90; fiber = fiber.return) ancestors.push(fiber);
        const top = ancestors.at(-1);
        if (!top || top.return || top.stateNode?.current !== top) continue;
        for (const fiber of ancestors) {
          const props = fiber.memoizedProps;
          if (!props?.conversation || !props.composerController ||
              typeof props.composerDisabled !== 'boolean' ||
              typeof props.selectModelId !== 'function' || typeof props.clearModelSelection !== 'function' ||
              !Array.isArray(props.availableSystemHints)) continue;
          const controller = props.composerController, conversation = props.conversation;
          const model = props.currentModelId ?? props.currentModelConfig?.id;
          if (controller.conversation !== conversation || typeof conversation.serverId$ !== 'function' ||
              (conversation.serverId$() || null) !== conversationId ||
              props.isTemporaryChat !== temporary || props.composerDisabled ||
              props.composerToolAvailability && props.composerToolAvailability !== 'default' ||
              props.loginModalGate?.shouldGateToLoginModal ||
              typeof model !== 'string' || !/^[a-z0-9][a-z0-9._-]{0,127}$/i.test(model)) return null;
          const hints = TOOLS.map(tool => props.availableSystemHints.filter(h => h?.systemHint === tool.hint));
          if (hints.some(matches => matches.length !== 1 || matches[0].isLoggedOutUpsell ||
              matches[0].isConnector || matches[0].isDangerous || matches[0].hideFromInitialSelection)) return null;
          if (candidates.has(controller) && candidates.get(controller).model !== model) return null;
          candidates.set(controller, { controller, conversation, model, href: url.href, token, account });
        }
      }
      return candidates.size === 1 ? candidates.values().next().value : null;
    }

    function current(binding) {
      try {
        const now = capture();
        return !!now && Object.keys(binding).every(key => binding[key] === now[key]);
      } catch (_) { return false; }
    }

    function state(binding) {
      if (!current(binding) || typeof namespace?.Ng !== 'function' || typeof namespace?.Bg !== 'function') return null;
      const value = namespace.Ng(binding.controller);
      if (value?.locked !== false || ![null, 'search', 'picture_v2'].includes(value.activeSystemHintType) ||
          !(value.activeConnectorSystemHintTypes instanceof Set) || value.activeConnectorSystemHintTypes.size ||
          value.activeCustomAgentSystemHintType !== null) return null;
      return value;
    }

    function loaded() {
      return page.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
        !!page.document.querySelector('link[rel="modulepreload"][href="' + RUNTIME_URL + '"]');
    }

    function load() {
      if (namespace) return Promise.resolve();
      if (loading) return loading;
      const importer = options.loadRuntime || (url => import(url));
      let timer;
      loading = Promise.race([
        Promise.resolve().then(() => importer(RUNTIME_URL)),
        new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('runtime_timeout')), 1500); })
      ]).then(value => {
        if (typeof value?.Ng !== 'function' || typeof value?.Bg !== 'function') throw new Error('runtime_unknown');
        namespace = value;
      }).catch(() => { cooldown = Date.now() + 10000; }).finally(() => {
        page.clearTimeout(timer);
        loading = null;
      });
      return loading;
    }

    function cancelPending() {
      if (!pending) return;
      const previous = pending;
      pending = null;
      previous.result('list_composer_tools', false, '菜单请求已被新的操作替代。');
    }

    function entries(value) {
      return TOOLS.map(tool => Object.freeze({ id: PREFIX + serial + '_' + tool.semantic,
        label: tool.label, semantic: tool.semantic, selected: value.activeSystemHintType === tool.hint,
        kind: 'toggle', opensSubmenu: false }));
    }

    function requestPrivateOptions(emitOptions, result, fallback) {
      cancelPending();
      catalog = null;
      receipt = null;
      let binding;
      try {
        binding = capture();
        if (!binding || !loaded() || !namespace && Date.now() < cooldown ||
            page.document.querySelector('#composer-plus-btn')?.getAttribute('aria-expanded') === 'true') return false;
      } catch (_) { return false; }
      const request = { binding, result };
      pending = request;
      function complete() {
        if (pending !== request) return;
        pending = null;
        if (!current(binding)) return result('list_composer_tools', false, '会话或工具状态已变化，请重新选择。');
        let value;
        try { value = state(binding); } catch (_) { /* Unknown runtime stays on the existing path. */ }
        if (!value) return fallback();
        serial += 1;
        const list = entries(value);
        catalog = { binding, list, active: value.activeSystemHintType, emitOptions };
        emitOptions(list);
        result('list_composer_tools', true, '');
      }
      if (namespace) complete();
      else void load().then(complete);
      return true;
    }

    function selectPrivate(id, result, scheduleSnapshot) {
      if (typeof id !== 'string' || !id.startsWith(PREFIX)) return false;
      const owned = catalog, entry = owned?.list.find(item => item.id === id);
      let ok = false;
      try {
        if (!entry && receipt?.id === id) {
          ok = state(receipt.binding)?.activeSystemHintType === receipt.desired;
          result('select_composer_tool', ok, ok ? '' : '工具状态已变化，请重新选择。');
          return true;
        }
        const value = entry && state(owned.binding);
        if (!value) throw new Error('selection_stale');
        const hint = TOOLS.find(tool => tool.semantic === entry.semantic).hint;
        const desired = entry.selected ? null : hint;
        if (value.activeSystemHintType !== desired) {
          if (value.activeSystemHintType !== owned.active) throw new Error('selection_stale');
          // This is the official sender's live signal. Do not manufacture a parallel store.
          namespace.Bg(owned.binding.controller, desired, {
            skipComposerAutofocus: true, ifPrevSystemHint: value.activeSystemHintType
          });
        }
        ok = state(owned.binding)?.activeSystemHintType === desired;
        if (ok) {
          receipt = { id, binding: owned.binding, desired };
          serial += 1;
          catalog = { ...owned, active: desired, list: entries({ activeSystemHintType: desired }) };
          owned.emitOptions(catalog.list);
        }
      } catch (_) { /* A possibly applied mutation must not be replayed through DOM. */ }
      if (!ok) { catalog = null; receipt = null; }
      result('select_composer_tool', ok, ok ? '' : '工具状态未能确认，请重新打开工具选择。');
      scheduleSnapshot();
      return true;
    }

    function dismissPrivateOptions() {
      const owned = !!catalog || !!pending;
      cancelPending();
      catalog = null;
      receipt = null;
      return owned;
    }

    return Object.freeze({ version: 1, requestPrivateOptions, selectPrivate, dismissPrivateOptions });
  }

  let privateRuntime;
  function runtime() { return privateRuntime || (privateRuntime = createPrivateRuntime(root)); }
  return Object.freeze({ select, createPrivateRuntime,
    requestPrivateOptions: (...args) => runtime().requestPrivateOptions(...args),
    selectPrivate: (...args) => runtime().selectPrivate(...args),
    dismissPrivateOptions: (...args) => runtime().dismissPrivateOptions(...args) });
});
