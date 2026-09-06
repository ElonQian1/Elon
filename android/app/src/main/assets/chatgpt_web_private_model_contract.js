(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateModelContract = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const URLS = Object.freeze({
    shared: 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js',
    conversation: 'https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js',
    composer: 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js'
  });

  function identity() {
    const headers = page.__elonChatGptPrivateTransport?.copySameOriginRequestHeaders?.();
    const values = {};
    for (const [key, value] of Object.entries(headers || {})) values[key.toLowerCase()] = value;
    if (!/^Bearer\s+\S{8,65536}$/.test(values.authorization || '')) return null;
    return JSON.stringify(['authorization', 'chatgpt-account-id', 'oai-device-id'].map(key => values[key] || ''));
  }

  function picker(node) {
    if (!node?.isConnected) return null;
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    const candidates = new Set();
    for (const start of [node[key], node[key]?.alternate]) {
      const chain = [];
      for (let fiber = start; fiber && chain.length < 90; fiber = fiber.return) chain.push(fiber);
      const top = chain.at(-1);
      if (!top || top.return || top.stateNode?.current !== top) continue;
      for (const fiber of chain) {
        const props = fiber.memoizedProps;
        const menu = props?.dropdownContent?.props;
        if (!menu?.composerIntelligencePickerState || !menu.conversation ||
            !(menu.modelsData?.models instanceof Map)) continue;
        if (props.ariaDisabled !== false || typeof props.dropdownOpen !== 'boolean') return null;
        candidates.add(menu);
      }
    }
    return candidates.size === 1 ? candidates.values().next().value : null;
  }

  function capture(getTrigger) {
    const url = new URL(page.location.href);
    const cid = /^(?:\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname)?.[1] || null;
    const project = /^\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?\/project$/i.test(url.pathname);
    if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
        url.search && url.search !== '?temporary-chat=true' || url.search && url.pathname.startsWith('/g/') ||
        url.pathname !== '/' && !cid && !project) return null;
    const token = page.__elonChatGptDocumentToken, account = identity(), menu = picker(getTrigger());
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '') || !account || !menu ||
        typeof menu.conversation.serverId$ !== 'function' || (menu.conversation.serverId$() || null) !== cid) return null;
    return { getTrigger, href: url.href, token, account, conversation: menu.conversation, menu };
  }

  function current(binding) {
    try {
      const now = binding && capture(binding.getTrigger);
      return now && ['href', 'token', 'account', 'conversation'].every(key => now[key] === binding[key]) ? now : null;
    } catch (_) { return null; }
  }

  function validate(modules) {
    const { shared: s, conversation: c, composer: b } = modules || {};
    return ['M$', 'RW', 'uo'].every(key => typeof s?.[key] === 'function') && typeof s?.t4?.PRO === 'string' &&
      ['Nrn', 'yRt', 'Grn', 'vRt', 'p8t', 'l0', 'M1t'].every(key => typeof c?.[key] === 'function') &&
      typeof c?.Rdn?.CHATGPT_MODEL_PICKER_SURFACE_COMPOSER === 'string' && typeof b?.Ih === 'function';
  }

  function read(binding, modules) {
    const now = current(binding);
    if (!now || !validate(modules) || modules.shared.uo(now.conversation) !== false) return null;
    const c = modules.conversation, model = c.Nrn(now.conversation);
    const effortStore = c.yRt(now.conversation), tierStore = c.l0(now.conversation);
    if (!SLUG.test(model?.id || '') || typeof effortStore?.conversationThinkingEffort$ !== 'function' ||
        typeof effortStore.setThinkingEffort !== 'function' ||
        typeof tierStore?.getDraftServiceTier !== 'function') return null;
    return { model: model.id, effort: effortStore.conversationThinkingEffort$(),
      draftServiceTier: tierStore.getDraftServiceTier(), effortStore, binding: now };
  }

  function selectionKey(item) { return JSON.stringify([item.modelSlug, item.thinkingEffort ?? null, item.bucket]); }

  function catalog(binding, modules) {
    const live = read(binding, modules);
    if (!live) return null;
    const menu = live.binding.menu, pickerState = menu.composerIntelligencePickerState;
    const selections = pickerState.bucketSelections, version = pickerState.selectedVersionEntry?.id;
    if (!Array.isArray(selections) || selections.length < 1 || selections.length > 30 ||
        typeof version !== 'string' || !version || !menu.modelSwitcherDenialsBySlug ||
        typeof menu.modelSwitcherDenialsBySlug !== 'object') return null;
    const choices = [], seen = new Set();
    for (const item of selections) {
      if (!['available', 'unavailable', 'rate_limited'].includes(item?.availability?.status)) return null;
      if (item.availability.status !== 'available') continue;
      if (!SLUG.test(item.modelSlug || '') || item.modelConfig?.id !== item.modelSlug ||
          menu.modelsData.models.get(item.modelSlug)?.id !== item.modelSlug ||
          !Number.isInteger(item.bucket) || !item.category || typeof item.category.modelLane !== 'string') return null;
      if (modules.conversation.p8t({ modelSlug: item.modelSlug,
        modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug })?.status !== 'available') continue;
      if (item.thinkingEffort != null && (typeof item.thinkingEffort !== 'string' ||
          !modules.conversation.vRt(item.modelConfig)?.includes(item.thinkingEffort))) continue;
      const label = item.title;
      if (typeof label !== 'string' || !label.trim() || label.length > 120) return null;
      const key = selectionKey(item);
      if (seen.has(key)) return null;
      seen.add(key);
      choices.push({ key, label: label.trim(), selection: item,
        selected: live.model === item.modelSlug && (item.thinkingEffort == null || live.effort === item.thinkingEffort) });
    }
    return choices.length && choices.filter(item => item.selected).length <= 1 ? { live, version, choices } : null;
  }

  function matches(live, target) {
    return !!live && live.model === target.modelSlug && (target.thinkingEffort == null || live.effort === target.thinkingEffort);
  }

  function apply(binding, modules, target, expected, version) {
    const latest = catalog(binding, modules);
    const entry = latest?.version === version && latest.choices.find(item => item.key === selectionKey(target));
    if (!entry) throw new Error('model_selection_stale');
    const live = latest.live;
    if (matches(live, entry.selection)) return true;
    if (live.model !== expected.model || live.effort !== expected.effort ||
        live.draftServiceTier !== expected.draftServiceTier) throw new Error('model_selection_changed');
    const item = entry.selection, c = modules.conversation, s = modules.shared, conversation = binding.conversation;
    // Mirror fqn's public chat preset action using its existing stores and mutators.
    // Work/service-tier changes and unknown advanced model contracts are not authorized here.
    if (item.category.modelLane === s.t4.PRO && item.thinkingEffort != null) {
      const mutation = c.M1t();
      if (typeof mutation?.mutate !== 'function') throw new Error('model_runtime_unavailable');
      mutation.mutate({ juices: { [s.t4.PRO]: item.thinkingEffort } });
    }
    s.M$(() => {
      if (!current(binding)) throw new Error('model_selection_changed');
      if (item.thinkingEffort != null) live.effortStore.setThinkingEffort(item.thinkingEffort, item.modelSlug);
      if (item.modelSlug === live.model) {
        c.Grn(conversation, item.modelSlug);
        if (item.thinkingEffort != null && item.thinkingEffort !== live.effort) {
          const mutation = s.RW();
          if (typeof mutation?.mutate !== 'function') throw new Error('model_runtime_unavailable');
          mutation.mutate({ modelSlug: item.modelSlug, thinkingEffort: item.thinkingEffort });
        }
      } else modules.composer.Ih({ conversation, currentModelId: live.model, modelId: item.modelSlug,
        ...(item.thinkingEffort == null ? {} : { thinkingEffort: item.thinkingEffort }),
        applyModelSelection: c.Grn, modelPickerSurface: c.Rdn.CHATGPT_MODEL_PICKER_SURFACE_COMPOSER });
    });
    const after = read(binding, modules);
    return matches(after, item) && after.draftServiceTier === live.draftServiceTier;
  }

  return Object.freeze({ version: 1, urls: URLS, capture, current, validate, catalog, read, matches, apply });
});
