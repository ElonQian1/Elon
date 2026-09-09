(function (root, factory) {
  'use strict';
  if (Number(root?.__elonChatGptPrivateModelState?.version) >= 5) return;
  const states = new WeakMap();
  const api = Object.freeze({ version: 5, create(page, options) {
    const instance = factory(page, options);
    states.set(page, instance.state);
    return instance;
  }, state: page => states.get(page)?.() || 'not_observed' });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateModelState = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const contract = (options.contract || page.__elonChatGptPrivateModelContract).create(page);
  const extra = page.__elonChatGptPrivateModelCatalog?.create(page, contract);
  const PREFIX = 'private_model_';
  const PAGE_SIZE = 20;
  let modules, loading, cooldown = 0, serial = 0, owned = null, pending = null, receipt = null, mutation = null;
  let code = 'not_observed', diagnosticToken;
  const state = () => diagnosticToken === page.__elonChatGptDocumentToken ? code : 'not_observed';

  function load() {
    if (modules) return Promise.resolve();
    if (loading) return loading;
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    const importer = options.loadRuntime || (url => bindings ? bindings.load(url) : import(url));
    let timer;
    loading = Promise.race([
      Promise.all(Object.entries(contract.urls).map(async ([key, url]) => [key, await importer(url)])),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('model_runtime_timeout')), 1500); })
    ]).then(entries => {
      const candidate = Object.fromEntries(entries);
      if (!contract.validate(candidate)) throw new Error('model_runtime_unknown');
      modules = candidate;
    }).catch(error => {
      code = ['model_runtime_timeout', 'model_runtime_unknown'].includes(error?.message)
        ? error.message.slice(6) : 'runtime_unavailable';
      cooldown = Date.now() + 10000;
    }).finally(() => {
      page.clearTimeout(timer); loading = null;
    });
    return loading;
  }

  function cancelMutation() {
    const previous = mutation; mutation = null;
    if (!previous) return;
    previous.cancel?.();
    previous.waiters.forEach(result => result('select_model_option', false, '模型选择已被新的操作替代。'));
  }

  function cancel() {
    cancelMutation();
    if (!pending) return;
    const previous = pending; pending = null;
    previous.result('list_model_options', false, '模型请求已被新的操作替代。');
  }

  function emitCatalog(binding, value, emit, view = 'presets', page = 0) {
    serial += 1;
    const lastPage = Math.max(0, Math.ceil(value.choices.length / PAGE_SIZE) - 1);
    page = Math.max(0, Math.min(page, lastPage));
    const visible = view === 'models' ? value.choices.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE) : value.choices;
    const choices = visible.map((item, index) => ({ ...item, id: PREFIX + serial + '_' + index }));
    const advanced = PREFIX + serial + '_advanced';
    const back = PREFIX + serial + '_back', official = PREFIX + serial + '_official';
    const previous = PREFIX + serial + '_previous', next = PREFIX + serial + '_next';
    let showOther = true;
    if (view === 'advanced') {
      try { showOther = extra?.catalog(binding, modules)?.choices.length !== 0; }
      catch (_) { /* Unknown catalog remains reachable through the existing reader. */ }
    }
    owned = { binding, ...value, choices, advanced, back, official, previous, next, page, lastPage, emit, view };
    const navigation = (id, label) => ({ id, label, selected: false, semantic: 'model', kind: 'menuitem', opensSubmenu: true });
    emit([...(view === 'models' ? [navigation(back, '返回高级')] :
      view === 'advanced' && value.canGoBack !== false ? [navigation(back, '返回档位')] : []),
      ...choices.map(item => ({ id: item.id, label: item.label, selected: item.selected,
        semantic: item.semantic || 'model', kind: 'menuitemradio', opensSubmenu: false })),
      ...(view === 'models' ? [...(page > 0 ? [navigation(previous, '上一页')] : []),
        ...(page < lastPage ? [navigation(next, '下一页')] : [])] :
        view === 'advanced' ? (showOther ? [navigation(official, '其他官网模型')] : []) : [navigation(advanced, '高级')])]);
  }

  function readCatalog(menu, view) {
    return view === 'models' ? extra?.catalog(menu.binding, modules) : view === 'advanced'
      ? contract.advancedCatalog(menu.binding, modules) : contract.catalog(menu.binding, modules);
  }

  function selectCatalog(menu, target, result, snapshot) {
    if (mutation?.id === target.id) { mutation.waiters.push(result); return; }
    cancelMutation();
    const operation = { id: target.id, waiters: [result], active: () => mutation === operation };
    mutation = operation;
    void extra.apply(menu.binding, modules, target.selection, menu.live, operation).then(catalogState => {
      if (mutation !== operation) return;
      receipt = { id: target.id, binding: menu.binding, catalogState };
      const updated = extra.catalog(menu.binding, modules);
      if (updated) emitCatalog(menu.binding, updated, menu.emit, 'models', menu.page);
      else owned = null;
      mutation = null;
      operation.waiters.forEach(done => done('select_model_option', true, ''));
      snapshot();
    }).catch(() => {
      if (mutation !== operation) return;
      mutation = null; owned = null; receipt = null;
      operation.waiters.forEach(done => done('select_model_option', false, '模型状态或隐私资格未能确认，请重新选择。'));
      snapshot();
    });
  }

  function request(getTrigger, emit, result, fallback) {
    cancel(); owned = null; receipt = null;
    diagnosticToken = page.__elonChatGptDocumentToken;
    let binding;
    try {
      binding = contract.capture(getTrigger);
      if (!binding) { code = contract.state?.() || 'capture_error'; return false; }
      if (!modules && Date.now() < cooldown) { code = 'cooldown'; return false; }
      if (getTrigger()?.getAttribute('aria-expanded') === 'true') { code = 'menu_open'; return false; }
      if (!Object.values(contract.urls).every(url => page.__elonChatGptPrivateRuntimeBindings
            ? page.__elonChatGptPrivateRuntimeBindings.observed(url) :
            page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
            page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]'))) {
        code = 'runtime_not_observed'; return false;
      }
    } catch (_) { code = 'capture_error'; return false; }
    code = 'loading';
    const request = { result }; pending = request;
    function complete() {
      if (pending !== request) return;
      pending = null;
      if (!contract.current(binding)) {
        code = 'context_changed'; return result('list_model_options', false, '会话已经变化，请重新选择模型。');
      }
      if (!modules) return fallback();
      let value, view = 'presets';
      try {
        value = contract.catalog(binding, modules);
        if (!value) { value = contract.advancedCatalog(binding, modules); view = 'advanced'; }
      } catch (_) { /* Unknown is not unavailable. */ }
      if (!value) { code = 'catalog_unavailable'; return fallback(); }
      emitCatalog(binding, value, emit, view);
      code = 'ready';
      result('list_model_options', true, '');
    }
    if (modules) complete(); else void load().then(complete);
    return true;
  }

  function select(id, result, snapshot, advanced) {
    if (typeof id !== 'string' || !id.startsWith(PREFIX)) return false;
    const menu = owned;
    const destination = menu && (menu.view === 'presets' ? id === menu.advanced && 'advanced' :
      menu.view === 'advanced' ? id === menu.official ? 'models' : menu.canGoBack !== false && id === menu.back && 'presets' :
        id === menu.back ? 'advanced' : (id === menu.previous && menu.page > 0 || id === menu.next && menu.page < menu.lastPage) && 'models');
    if (destination && contract.current(menu.binding)) {
      cancelMutation();
      receipt = null;
      let value;
      try {
        value = readCatalog(menu, destination);
      } catch (_) { /* An unknown extended contract does not replace existing models. */ }
      if (!value && destination !== 'presets') { dismiss(); advanced(); return true; }
      const page = menu.view !== 'models' ? 0 : menu.page + (id === menu.next ? 1 : id === menu.previous ? -1 : 0);
      if (value) emitCatalog(menu.binding, value, menu.emit, destination, page);
      else { owned = null; receipt = null; }
      result('select_model_option', !!value, value ? '' : '档位状态仍在更新，请重新打开选择。');
      snapshot(); return true;
    }
    const target = menu?.choices.find(item => item.id === id);
    if (target && menu.view === 'models') { selectCatalog(menu, target, result, snapshot); return true; }
    cancelMutation();
    let ok = false;
    try {
      if (!target && receipt?.id === id) ok = receipt.catalogState
        ? extra.matches(receipt.binding, modules, receipt.catalogState) : receipt.advancedState
        ? contract.matchesAdvanced(receipt.binding, modules, receipt.advancedState)
        : contract.matches(contract.read(receipt.binding, modules), receipt.selection);
      else if (target) {
        const advancedState = menu.view === 'advanced'
          ? contract.applyAdvanced(menu.binding, modules, target.selection, menu.live) : null;
        ok = menu.view === 'advanced' ? !!advancedState
          : contract.apply(menu.binding, modules, target.selection, menu.live, menu.version);
        if (ok) {
          receipt = { id, binding: menu.binding, selection: target.selection, advancedState };
          const updated = menu.view === 'advanced' ? contract.advancedCatalog(menu.binding, modules)
            : contract.catalog(menu.binding, modules);
          if (updated) emitCatalog(menu.binding, updated, menu.emit, menu.view);
          else owned = null;
        }
      }
    } catch (_) { /* Never replay a possibly applied preference/model mutation through DOM. */ }
    if (!ok) { owned = null; receipt = null; }
    result('select_model_option', ok, ok ? '' : '模型或档位状态未能确认，请重新打开选择。');
    snapshot();
    return true;
  }

  function dismiss() {
    const handled = !!owned || !!pending || !!mutation;
    cancel(); owned = null; receipt = null;
    return handled;
  }

  return Object.freeze({ version: 5, state, request, select, dismiss });
});
