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
      { hint: 'picture_v2', semantic: 'image_generation', label: '创建图片' },
      { hint: 'tatertot', semantic: 'study', label: '学习与研究' },
      { hint: 'canvas', semantic: 'canvas', label: '画布' }
    ];
    let namespace, namespaceDocument, namespaceToken, loading, cooldown = 0, serial = 0, catalog = null, pending = null, receipt = null;

    function capture() {
      const context = page.__elonChatGptPrivateComposerToolContext ||
        (typeof module === 'object' && module.exports ? require('./chatgpt_web_private_composer_tool_context') : null);
      return context?.capture(page, TOOLS) || null;
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
      if (value?.locked !== false || ![null, ...TOOLS.map(tool => tool.hint)].includes(value.activeSystemHintType) ||
          !(value.activeConnectorSystemHintTypes instanceof Set) || value.activeConnectorSystemHintTypes.size ||
          value.activeCustomAgentSystemHintType !== null) return null;
      return value;
    }

    function loaded() {
      if (page.__elonChatGptPrivateRuntimeBindings) return page.__elonChatGptPrivateRuntimeBindings.observed(RUNTIME_URL);
      return page.performance?.getEntriesByName?.(RUNTIME_URL, 'resource')?.length > 0 ||
        !!page.document.querySelector('link[rel="modulepreload"][href="' + RUNTIME_URL + '"]');
    }

    function load() {
      if (namespaceDocument !== page.document || namespaceToken !== page.__elonChatGptDocumentToken) namespace = null;
      if (namespace) return Promise.resolve();
      if (loading) return loading;
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      const importer = options.loadRuntime || (url => bindings ? bindings.load(url) : import(url));
      const document = page.document, token = page.__elonChatGptDocumentToken;
      let timer;
      const imported = Promise.resolve().then(() => importer(RUNTIME_URL));
      loading = (bindings && !options.loadRuntime ? imported : Promise.race([
        imported,
        new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('runtime_timeout')), 1500); })
      ])).then(value => {
        if (document !== page.document || token !== page.__elonChatGptDocumentToken) throw new Error('runtime_context_changed');
        if (typeof value?.Ng !== 'function' || typeof value?.Bg !== 'function') throw new Error('runtime_unknown');
        namespace = value; namespaceDocument = document; namespaceToken = token;
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

    function entries(value, binding) {
      return TOOLS.filter(tool => binding.allowed.split(',').includes(tool.hint)).map(tool => Object.freeze({ id: PREFIX + serial + '_' + tool.semantic,
        label: tool.label, semantic: tool.semantic, selected: value.activeSystemHintType === tool.hint,
        kind: 'toggle', opensSubmenu: false }));
    }

    function requestPrivateOptions(emitOptions, result, fallback) {
      cancelPending();
      catalog = null;
      receipt = null;
      let binding;
      try {
        if (namespaceDocument !== page.document || namespaceToken !== page.__elonChatGptDocumentToken) namespace = null;
        binding = capture();
        if (!binding || !loaded() || !namespace && Date.now() < cooldown ||
            binding.node.getAttribute('aria-expanded') === 'true') return false;
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
        const list = entries(value, binding);
        catalog = { binding, list, active: value.activeSystemHintType, emitOptions };
        emitOptions(list);
        result('list_composer_tools', true, 'official_tool_runtime_v1:accepted');
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
          catalog = { ...owned, active: desired, list: entries({ activeSystemHintType: desired }, owned.binding) };
          owned.emitOptions(catalog.list);
        }
      } catch (_) { /* A possibly applied mutation must not be replayed through DOM. */ }
      if (!ok) { catalog = null; receipt = null; }
      result('select_composer_tool', ok, ok ? 'official_tool_runtime_v1:accepted' : '工具状态未能确认，请重新打开工具选择。');
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

    return Object.freeze({ version: 3, requestPrivateOptions, selectPrivate, dismissPrivateOptions });
  }

  let privateRuntime;
  function runtime() { return privateRuntime || (privateRuntime = createPrivateRuntime(root)); }
  return Object.freeze({ select, createPrivateRuntime,
    requestPrivateOptions: (...args) => runtime().requestPrivateOptions(...args),
    selectPrivate: (...args) => runtime().selectPrivate(...args),
    dismissPrivateOptions: (...args) => runtime().dismissPrivateOptions(...args) });
});
