(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 9, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateModelContract = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const MODEL_ID = /^[a-z0-9][a-z0-9._:/-]{0,255}$/i;
  let code = 'not_observed';
  const fail = value => { code = value; return null; };
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
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

  function picker(node, allowDisabled = false) {
    if (!node?.isConnected) return fail('trigger_detached');
    const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
    const ancestors = ownerPath?.resolve(node[key])?.ancestors || [];
    if (!ancestors.length) return fail('owner_unavailable');
    const candidates = new Set();
    for (const fiber of ancestors) {
      const props = fiber.memoizedProps;
      const menu = props?.dropdownContent?.props;
      if (!menu?.composerIntelligencePickerState || !menu.conversation ||
          !(menu.modelsData?.models instanceof Map)) continue;
      if ((props.ariaDisabled !== false && !(allowDisabled && props.ariaDisabled === true)) ||
          typeof props.dropdownOpen !== 'boolean') return fail('picker_disabled');
      candidates.add(menu);
    }
    return candidates.size === 1 ? candidates.values().next().value :
      fail(candidates.size ? 'picker_ambiguous' : 'picker_missing');
  }

  function identityContext() {
    const url = new URL(page.location.href);
    const cid = /^(?:\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?)?\/c\/([a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12})$/i.exec(url.pathname)?.[1] || null;
    const project = /^\/g\/g-p-[a-f0-9]{32}(?:-[A-Za-z0-9_-]{1,124})?\/project$/i.test(url.pathname);
    if (url.origin !== 'https://chatgpt.com' || url.username || url.password || url.hash ||
        url.search && url.search !== '?temporary-chat=true' || url.search && url.pathname.startsWith('/g/') ||
        url.pathname !== '/' && !cid && !project) return fail('route_unsupported');
    const token = page.__elonChatGptDocumentToken, account = identity();
    if (!/^doc_[a-z0-9_]{3,80}$/.test(token || '')) return fail('document_unavailable');
    if (!account) return fail('identity_unavailable');
    return { href: url.href, cid, token, account };
  }

  function capture(getTrigger, allowDisabled = false) {
    const owner = identityContext();
    if (!owner) return null;
    const menu = picker(getTrigger(), allowDisabled);
    if (!menu) return null;
    if (typeof menu.conversation.serverId$ !== 'function' ||
        (menu.conversation.serverId$() || null) !== owner.cid) return fail('conversation_mismatch');
    code = 'bound';
    return { ...owner, getTrigger, conversation: menu.conversation, menu };
  }

  function matchingOwner(binding, allowDisabled) {
    try {
      const now = binding && capture(binding.getTrigger, allowDisabled);
      return now && ['href', 'token', 'account', 'conversation'].every(key => now[key] === binding[key]) ? now : null;
    } catch (_) { return null; }
  }

  function current(binding) { return matchingOwner(binding, false); }

  // Post-dispatch identity is independent of picker rendering. The reply owner
  // still verifies the new message, selected branch and original user parent.
  function ownerState(binding) {
    try {
      if (!binding) return 'owner_missing';
      const now = identityContext();
      if (!now) return ({ route_unsupported: 'owner_route', document_unavailable: 'owner_document',
        identity_unavailable: 'owner_identity' })[code] || 'owner_unavailable';
      if (now.href !== binding.href || now.cid !== binding.cid) return 'owner_route';
      if (now.token !== binding.token) return 'owner_document';
      if (now.account !== binding.account) {
        const before = JSON.parse(binding.account), after = JSON.parse(now.account);
        if (before[0] !== after[0]) return 'owner_auth';
        if (before[1] !== after[1]) return 'owner_account';
        return 'owner_device';
      }
      if (binding.menu?.conversation !== binding.conversation) return 'owner_object';
      if (typeof binding.conversation?.serverId$ !== 'function' ||
          (binding.conversation.serverId$() || null) !== now.cid) return 'owner_server';
      return 'owner_current';
    } catch (_) { return 'owner_unavailable'; }
  }

  function ownerCurrent(binding) { return ownerState(binding) === 'owner_current'; }

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
    if (!MODEL_ID.test(model?.id || '') || typeof effortStore?.conversationThinkingEffort$ !== 'function' ||
        typeof effortStore.setThinkingEffort !== 'function' ||
        typeof tierStore?.getDraftServiceTier !== 'function') return null;
    return { model: model.id, effort: effortStore.conversationThinkingEffort$(),
      draftServiceTier: tierStore.getDraftServiceTier(), effortStore, binding: now };
  }

  function selectionKey(item) { return JSON.stringify([item.modelSlug, item.thinkingEffort ?? null, item.bucket]); }

  function restrictedSelections(live, modules) {
    const menu = live.binding.menu, state = menu.composerIntelligencePickerState;
    const capability = state.restrictedModelCapability, model = capability?.modelConfig;
    if (state.bucketSelections != null || capability?.kind !== 'thinking-effort' ||
        model?.id !== live.model || menu.modelsData.models.get(live.model)?.id !== live.model ||
        !SLUG.test(capability.thinkingEffortLane || '') ||
        !Array.isArray(menu.modelsData.categories) || menu.modelsData.categories.length > 100 ||
        !Array.isArray(model.thinkingEfforts) || !model.thinkingEfforts.length || model.thinkingEfforts.length > 30 ||
        typeof model.title !== 'string' || !model.title.trim() || model.title.length > 120 ||
        modules.conversation.p8t({ modelSlug: live.model,
          modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug })?.status !== 'available') return null;
    const allowed = modules.conversation.vRt(model);
    if (!Array.isArray(allowed) || !allowed.length || allowed.length > 30 || !allowed.every(value => SLUG.test(value))) return null;
    const seen = new Set();
    for (const effort of model.thinkingEfforts) {
      if (!SLUG.test(effort?.thinking_effort || '') || seen.has(effort.thinking_effort)) return null;
      seen.add(effort.thinking_effort);
    }
    const efforts = model.thinkingEfforts.filter(item => allowed.includes(item.thinking_effort));
    // fqn resolves the effective display value without persisting a default.
    const selected = efforts.find(item => item.thinking_effort === live.effort) ??
      efforts.find(item => item.thinking_effort === model.defaultThinkingEffort) ?? efforts[0];
    const category = menu.modelsData.categories.find(item => item?.modelLane === capability.thinkingEffortLane) ??
      menu.modelsData.categories[0];
    const label = item => item?.short_label ?? item?.full_label;
    if (!category || typeof label(selected) !== 'string' || !label(selected).trim()) return null;
    const base = { availability: { status: 'available' },
      bucket: state.selectedVersionEntry?.intelligencePresets?.[0]?.id ?? 0,
      category: { ...category, categoryId: live.model, defaultModel: live.model,
        label: model.title, modelLane: capability.thinkingEffortLane,
        shortLabel: model.title, shorterLabel: model.title, supportedModels: [live.model] },
      modelConfig: model, modelSlug: live.model, thinkingEffortLane: capability.thinkingEffortLane,
      defaultServiceTier: model.defaultServiceTier, serviceTierOptions: model.serviceTierOptions };
    const selections = efforts.filter(item => label(item) != null).map(item => ({ ...base,
      thinkingEffort: item.thinking_effort, title: label(item),
      selectedDisplayTitle: model.title + ' ' + label(item) }));
    return { selections, currentSelection: selections.find(item => item.thinkingEffort === selected.thinking_effort) };
  }

  function catalog(binding, modules) {
    const live = read(binding, modules);
    if (!live) return null;
    const menu = live.binding.menu, pickerState = menu.composerIntelligencePickerState;
    const restricted = pickerState.bucketSelections == null ? restrictedSelections(live, modules) : null;
    const selections = pickerState.bucketSelections ?? restricted?.selections, version = pickerState.selectedVersionEntry?.id;
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
        selected: live.model === item.modelSlug && (item.thinkingEffort == null ||
          (restricted?.currentSelection?.thinkingEffort ?? live.effort) === item.thinkingEffort) });
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
    const updatePro = item.category.modelLane === s.t4.PRO && item.thinkingEffort != null;
    const updateDefault = item.modelSlug === live.model && item.thinkingEffort != null && item.thinkingEffort !== live.effort;
    // Acquire every required writer before changing any preference or conversation store.
    const proPreference = updatePro ? c.M1t() : null;
    const defaultPreference = updateDefault ? s.RW() : null;
    if (updatePro && typeof proPreference?.mutate !== 'function' ||
        updateDefault && typeof defaultPreference?.mutate !== 'function') throw new Error('model_runtime_unavailable');
    const confirmed = catalog(binding, modules);
    if (confirmed?.version !== version || !confirmed.choices.some(choice => choice.key === entry.key) ||
        confirmed.live.model !== live.model || confirmed.live.effort !== live.effort ||
        confirmed.live.draftServiceTier !== live.draftServiceTier) throw new Error('model_selection_changed');
    // Mirror fqn's public chat preset action using its existing stores and mutators.
    // Work/service-tier changes and unknown advanced model contracts are not authorized here.
    if (updatePro) proPreference.mutate({ juices: { [s.t4.PRO]: item.thinkingEffort } });
    s.M$(() => {
      if (!current(binding)) throw new Error('model_selection_changed');
      if (item.thinkingEffort != null) live.effortStore.setThinkingEffort(item.thinkingEffort, item.modelSlug);
      if (item.modelSlug === live.model) {
        c.Grn(conversation, item.modelSlug);
        if (updateDefault) defaultPreference.mutate({ modelSlug: item.modelSlug, thinkingEffort: item.thinkingEffort });
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
    const menu = live.binding.menu, pickerState = menu.composerIntelligencePickerState;
    const restricted = pickerState.bucketSelections == null ? restrictedSelections(live, modules) : null;
    const selection = pickerState.bucketSelections == null ? restricted?.currentSelection : pickerState.currentSelection;
    if (menu.hideServiceTier === true || menu.lockedUpgradePreview != null || !selection ||
        !restricted && !matches(live, selection)) return [];
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

  return Object.freeze({ version: 9, state: () => code, urls: URLS, capture, current, ownerCurrent, ownerState, validate, catalog, read, matches, apply,
    readAdvanced, advancedCatalog, applyAdvanced, matchesAdvanced });
});
