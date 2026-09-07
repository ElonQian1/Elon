(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateModelCatalog = api;
})(typeof window === 'object' ? window : null, function (page, contract) {
  'use strict';
  const MODEL = /^[a-z0-9][a-z0-9._:/-]{0,255}$/i;
  const label = value => typeof value === 'string' && value.trim() && value.length <= 120 && !/[\x00-\x1f]/.test(value);
  const state = live => live && [live.model, live.effort, live.draftServiceTier, live.serviceTier, live.conversationVersion];
  const same = (a, b) => Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((v, i) => v === b[i]);

  function flatten(options, groups) {
    if (!Array.isArray(options) || options.length > 100) throw Error('model_catalog_schema');
    const rows = [], seen = new Set(); let remaining = 4000;
    function visit(items, group, depth) {
      if (!Array.isArray(items) || depth > 8) throw Error('model_catalog_schema');
      for (const item of items) {
        if (--remaining < 0 || !item || typeof item !== 'object' || seen.has(item)) throw Error('model_catalog_schema');
        seen.add(item);
        if ('options' in item) visit(item.options, group, depth + 1);
        else {
          if (!MODEL.test(item.value || '') || !label(item.name)) throw Error('model_catalog_schema');
          rows.push({ group, modelSlug: item.value, label: item.name.trim() });
        }
      }
    }
    for (const group of options) if (group && groups.has(group.categoryId)) visit(group.options, group.categoryId, 0);
    return rows;
  }

  function catalog(binding, modules) {
    const live = contract.readAdvanced(binding, modules), c = modules?.conversation, s = modules?.shared;
    if (!live || !['u1t', 'f8t', 'iin'].every(key => typeof c?.[key] === 'function') ||
        typeof s?.$3 !== 'function' || !c.l1t) return null;
    const menu = live.binding.menu, data = menu.modelsData;
    const groups = new Set(Object.values(c.l1t));
    if (groups.size !== 4 || !['alpha', 'data_campaigns', 'experiments', 'mainline'].every(x => groups.has(x)) ||
        !Array.isArray(data.categories) || data.categories.length > 100 ||
        !Array.isArray(data.groups) || data.groups.length > 100 || data.models.size > 4000 ||
        !data.groups.every(group => Array.isArray(group?.modelIds) && group.modelIds.length <= 4000) ||
        !menu.modelSwitcherDenialsBySlug || typeof menu.modelSwitcherDenialsBySlug !== 'object') return null;
    // Reuse the website's pure category builder. It applies hidden-model rules;
    // never call its React hook, enumerate arbitrary exports, or guess model IDs.
    const allowed = flatten(c.u1t({ modelsData: data }), groups);
    const allowedKeys = new Set(allowed.map(row => row.group + '\n' + row.modelSlug));
    const rows = menu.modelSwitcherCategoryOptions == null ? allowed : flatten(menu.modelSwitcherCategoryOptions, groups);
    const choices = [], seen = new Set();
    for (const row of rows) {
      if (!allowedKeys.has(row.group + '\n' + row.modelSlug) || seen.has(row.modelSlug)) continue;
      seen.add(row.modelSlug);
      if (data.models.get(row.modelSlug)?.id !== row.modelSlug ||
          c.f8t({ modelSlug: row.modelSlug, modelSwitcherDenialsBySlug: menu.modelSwitcherDenialsBySlug }) !== true) continue;
      choices.push({ key: 'model:' + row.modelSlug, label: row.label, semantic: 'model_catalog',
        selected: live.model === row.modelSlug, selection: { kind: 'catalog_model', modelSlug: row.modelSlug } });
    }
    return { live, version: live.conversationVersion, choices };
  }

  function matches(binding, modules, receipt) {
    return same(state(contract.readAdvanced(binding, modules)), receipt);
  }

  async function apply(binding, modules, target, expected, operation) {
    function checked() {
      if (!operation.active()) throw Error('model_selection_cancelled');
      const value = catalog(binding, modules);
      if (target?.kind !== 'catalog_model' || !value?.choices.some(x => x.selection.modelSlug === target.modelSlug) ||
          !same(state(value.live), state(expected))) throw Error('model_selection_changed');
      return value.live;
    }
    let live = checked();
    if (live.model === target.modelSlug) return state(live);
    const gate = modules.shared.$3('3016847258');
    if (typeof gate !== 'boolean') throw Error('model_privacy_unknown');
    if (gate) {
      let timer;
      try {
        const allowed = await Promise.race([
          Promise.resolve(modules.conversation.iin(target.modelSlug)),
          new Promise((_, reject) => {
            operation.cancel = () => { page.clearTimeout(timer); reject(Error('model_selection_cancelled')); };
            timer = page.setTimeout(() => reject(Error('model_privacy_timeout')), 4000);
          })
        ]);
        if (allowed !== true) throw Error('model_privacy_unconfirmed');
      } finally { page.clearTimeout(timer); operation.cancel = null; }
    }
    live = checked();
    // SJn/bqn does not set an effort, tier or version. The official composer
    // action owns model preference persistence and conversation selection.
    modules.composer.Ih({ conversation: binding.conversation, currentModelId: live.model, modelId: target.modelSlug,
      applyModelSelection: modules.conversation.Grn,
      modelPickerSurface: modules.conversation.Rdn.CHATGPT_MODEL_PICKER_SURFACE_COMPOSER });
    const after = contract.readAdvanced(binding, modules);
    if (!after || after.model !== target.modelSlug ||
        !same(state(after).slice(1), state(live).slice(1))) throw Error('model_selection_unconfirmed');
    return state(after);
  }

  return Object.freeze({ version: 1, catalog, apply, matches });
});
