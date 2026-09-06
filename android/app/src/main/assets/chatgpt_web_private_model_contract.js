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

  function readAdvanced(binding, modules) {
    const live = read(binding, modules), c = modules?.conversation, s = modules?.shared;
    if (!live || !['Rrn', 'win', 'Ein', 'ay', 'iy', 'ry', 'Jrn', 'Hrn', 'f8t', 'c0']
      .every(key => typeof c?.[key] === 'function') || typeof s?.IX !== 'function' ||
      !['AUTO', 'INSTANT'].every(key => typeof s.t4[key] === 'string')) return null;
    const versionStore = c.Rrn(binding.conversation), tierStore = c.l0(binding.conversation);
    if (typeof versionStore?.conversationVersion$ !== 'function' || typeof versionStore.setConversationVersion !== 'function' ||
        typeof tierStore?.conversationServiceTier$ !== 'function' || typeof tierStore.setServiceTier !== 'function') return null;
    const conversationVersion = versionStore.conversationVersion$(), serviceTier = tierStore.conversationServiceTier$();
    if (conversationVersion != null && !SLUG.test(conversationVersion) ||
        serviceTier != null && !['standard', 'fast'].includes(serviceTier)) return null;
    return { ...live, conversationVersion, serviceTier, versionStore, tierStore };
  }

  function selectableModel(menu, modules, slug) {
    const category = modules.shared.IX(menu.modelsData, slug);
    return SLUG.test(slug || '') && menu.modelsData.models.has(slug) && category && !category.disabledByAdmin &&
      modules.conversation.f8t({ modelSlug: slug, modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug }) === true;
  }

  function tierChoices(live, modules) {
    const menu = live.binding.menu, selection = menu.composerIntelligencePickerState.currentSelection;
    if (menu.hideServiceTier === true || menu.lockedUpgradePreview != null || !selection || !matches(live, selection)) return [];
    const options = selection.serviceTierOptions;
    // The inspected normal-chat fast-mode switch requires both standard and fast.
    if (!Array.isArray(options) || options.length > 10 ||
        !['standard', 'fast'].every(tier => options.filter(item => item?.service_tier === tier).length === 1)) return [];
    const selected = modules.conversation.c0({ configuredServiceTier: live.serviceTier,
      defaultServiceTier: selection.defaultServiceTier, serviceTierOptions: options });
    if (!['standard', 'fast'].includes(selected)) return [];
    return ['standard', 'fast'].map(tier => ({ key: 'tier:' + tier,
      label: tier === 'standard' ? '标准响应速度' : '快速响应速度', semantic: 'service_tier',
      selected: selected === tier, selection: { kind: 'service_tier', tier } }));
  }

  function advancedCatalog(binding, modules) {
    const live = readAdvanced(binding, modules);
    if (!live) return null;
    const menu = live.binding.menu, data = menu.modelsData, c = modules.conversation;
    if (!Array.isArray(data.versions) || data.versions.length > 24 ||
        !Array.isArray(data.categories) || data.categories.length > 100 ||
        !menu.modelSwitcherDenialsBySlug || typeof menu.modelSwitcherDenialsBySlug !== 'object') return null;
    const choices = [], seen = new Set(), restricted = menu.composerIntelligencePickerState.isRestrictedModelPickerState === true;
    // Tqn's version list, using the same category and availability helpers.
    for (const version of data.versions) {
      if (!SLUG.test(version?.id || '') || seen.has(version.id) || !Array.isArray(version.slugs) ||
          version.slugs.length > 100 || !version.slugs.every(slug => SLUG.test(slug))) return null;
      seen.add(version.id);
      const categories = c.win(data, version);
      if (!Array.isArray(categories) || categories.length > 100) return null;
      if (!categories.length || version.disabled) continue;
      const available = restricted ? version.slugs.some(slug => selectableModel(menu, modules, slug)) :
        categories.some(category => c.f8t({ modelSlug: category.defaultModel,
          modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug }) === true);
      if (!available) continue;
      const label = c.Ein(version);
      if (typeof label !== 'string' || !label.trim() || label.length > 120) return null;
      choices.push({ key: 'version:' + version.id, label: label.trim(), semantic: 'model_version',
        selected: !restricted && live.conversationVersion === version.id,
        selection: { kind: 'model_version', versionId: version.id } });
    }
    choices.push(...tierChoices(live, modules));
    return choices.length ? { live, version: live.conversationVersion, choices,
      canGoBack: !!catalog(binding, modules) } : null;
  }

  function versionSelection(live, modules, versionId) {
    const menu = live.binding.menu, n = menu.composerIntelligencePickerState;
    const c = modules.conversation, s = modules.shared, data = menu.modelsData;
    if (n.isRestrictedModelPickerState !== true && n.selectedVersionEntry?.id !== live.conversationVersion ||
        n.currentSelection && !matches(live, n.currentSelection)) throw new Error('model_picker_pending');
    const version = data.versions.find(item => item.id === versionId);
    const lane = s.IX(data, live.model)?.modelLane;
    const bucket = n.bucketSelections == null ? undefined : n.currentBucket;
    const proEffort = lane === s.t4.PRO && n.bucketSelections != null
      ? n.currentSelection?.thinkingEffort : c.Jrn()?.juices?.[s.t4.PRO];
    const auto = c.Hrn(live.binding.conversation), presets = c.ay(version);
    if (typeof auto !== 'boolean' || presets != null && (!Array.isArray(presets) || presets.length > 100)) {
      throw new Error('model_version_schema');
    }
    // Dqn selects a matching bucket/lane, then falls back to an available catalog model.
    const buckets = presets ? c.iy({ autoSwitcherEnabled: auto,
      getModelAvailability: slug => c.p8t({ modelSlug: slug, modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug }),
      intelligencePresets: presets, modelsData: data, proThinkingEffort: proEffort, selectedVersionEntry: version }) : undefined;
    if (buckets != null && (!Array.isArray(buckets) || buckets.length > 100)) throw new Error('model_version_schema');
    const chosen = buckets ? c.ry({ bucketSelections: buckets, currentBucket: bucket, currentLane: lane }) : undefined;
    let modelSlug = chosen?.modelSlug, thinkingEffort = chosen?.thinkingEffort;
    if (modelSlug == null) {
      const desiredLane = [s.t4.AUTO, s.t4.INSTANT].includes(lane) ? (auto ? s.t4.AUTO : s.t4.INSTANT) : lane;
      const slugs = version.slugs.filter(slug => selectableModel(menu, modules, slug));
      modelSlug = slugs.find(slug => s.IX(data, slug)?.modelLane === desiredLane) ?? slugs[0];
      thinkingEffort = undefined;
    }
    if (!version.slugs.includes(modelSlug) || !selectableModel(menu, modules, modelSlug) ||
        thinkingEffort != null && (typeof thinkingEffort !== 'string' ||
          !c.vRt(data.models.get(modelSlug))?.includes(thinkingEffort))) throw new Error('model_version_unavailable');
    return { modelSlug, thinkingEffort, setEffort: chosen?.thinkingEffort != null && chosen.thinkingEffortLane != null };
  }

  function advancedState(live) {
    return live && [live.model, live.effort, live.draftServiceTier, live.serviceTier, live.conversationVersion];
  }

  function matchesAdvanced(binding, modules, receipt) {
    const state = advancedState(readAdvanced(binding, modules));
    return !!state && Array.isArray(receipt) && receipt.length === state.length &&
      receipt.every((value, index) => value === state[index]);
  }

  function applyAdvanced(binding, modules, target, expected) {
    if (!['model_version', 'service_tier'].includes(target?.kind)) throw new Error('model_option_unknown');
    const catalog = advancedCatalog(binding, modules);
    const key = target.kind === 'model_version' ? 'version:' + target.versionId : 'tier:' + target.tier;
    const entry = catalog?.choices.find(item => item.key === key);
    if (!entry) throw new Error('model_option_stale');
    const live = catalog.live;
    if (entry.selected) return advancedState(live);
    if (!advancedState(expected)?.every((value, index) => value === advancedState(live)[index])) {
      throw new Error('model_selection_changed');
    }
    const c = modules.conversation;
    if (target.kind === 'service_tier') {
      live.tierStore.setServiceTier(target.tier);
      const after = readAdvanced(binding, modules);
      if (!after || after.model !== live.model || after.effort !== live.effort ||
          after.conversationVersion !== live.conversationVersion || after.serviceTier !== target.tier ||
          after.draftServiceTier !== target.tier) throw new Error('service_tier_unconfirmed');
      return advancedState(after);
    }
    const selection = versionSelection(live, modules, target.versionId);
    modules.shared.M$(() => {
      if (!current(binding)) throw new Error('model_selection_changed');
      live.versionStore.setConversationVersion(target.versionId);
      if (selection.setEffort) live.effortStore.setThinkingEffort(selection.thinkingEffort, selection.modelSlug);
      modules.composer.Ih({ conversation: binding.conversation, currentModelId: live.model, modelId: selection.modelSlug,
        ...(selection.thinkingEffort == null ? {} : { thinkingEffort: selection.thinkingEffort }),
        applyModelSelection: c.Grn, modelPickerSurface: c.Rdn.CHATGPT_MODEL_PICKER_SURFACE_COMPOSER });
    });
    const after = readAdvanced(binding, modules);
    if (!matches(after, selection) || after.conversationVersion !== target.versionId ||
        after.serviceTier !== live.serviceTier || after.draftServiceTier !== live.draftServiceTier) {
      throw new Error('model_version_unconfirmed');
    }
    return advancedState(after);
  }

  return Object.freeze({ version: 1, urls: URLS, capture, current, validate, catalog, read, matches, apply,
    advancedCatalog, applyAdvanced, matchesAdvanced });
});
