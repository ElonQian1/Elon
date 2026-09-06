(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateModelState = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const contract = (options.contract || page.__elonChatGptPrivateModelContract).create(page);
  const PREFIX = 'private_model_';
  let modules, loading, cooldown = 0, serial = 0, owned = null, pending = null, receipt = null;

  function load() {
    if (modules) return Promise.resolve();
    if (loading) return loading;
    const importer = options.loadRuntime || (url => import(url));
    let timer;
    loading = Promise.race([
      Promise.all(Object.entries(contract.urls).map(async ([key, url]) => [key, await importer(url)])),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(new Error('model_runtime_timeout')), 1500); })
    ]).then(entries => {
      const candidate = Object.fromEntries(entries);
      if (!contract.validate(candidate)) throw new Error('model_runtime_unknown');
      modules = candidate;
    }).catch(() => { cooldown = Date.now() + 10000; }).finally(() => {
      page.clearTimeout(timer); loading = null;
    });
    return loading;
  }

  function cancel() {
    if (!pending) return;
    const previous = pending; pending = null;
    previous.result('list_model_options', false, '模型请求已被新的操作替代。');
  }

  function emitCatalog(binding, value, emit) {
    serial += 1;
    const choices = value.choices.map((item, index) => ({ ...item, id: PREFIX + serial + '_' + index }));
    const advanced = PREFIX + serial + '_advanced';
    owned = { binding, ...value, choices, advanced, emit };
    emit([...choices.map(item => ({ id: item.id, label: item.label, selected: item.selected,
      semantic: 'model', kind: 'menuitemradio', opensSubmenu: false })),
    { id: advanced, label: '高级', selected: false, semantic: 'model', kind: 'menuitem', opensSubmenu: true }]);
  }

  function request(getTrigger, emit, result, fallback) {
    cancel(); owned = null; receipt = null;
    let binding;
    try {
      binding = contract.capture(getTrigger);
      if (!binding || !modules && Date.now() < cooldown || getTrigger()?.getAttribute('aria-expanded') === 'true' ||
          !Object.values(contract.urls).every(url => page.performance?.getEntriesByName?.(url, 'resource')?.length > 0 ||
            page.document.querySelector('link[rel="modulepreload"][href="' + url + '"]'))) return false;
    } catch (_) { return false; }
    const request = { result }; pending = request;
    function complete() {
      if (pending !== request) return;
      pending = null;
      if (!contract.current(binding)) return result('list_model_options', false, '会话已经变化，请重新选择模型。');
      let value;
      try { value = contract.catalog(binding, modules); } catch (_) { /* Unknown is not unavailable. */ }
      if (!value) return fallback();
      emitCatalog(binding, value, emit);
      result('list_model_options', true, '');
    }
    if (modules) complete(); else void load().then(complete);
    return true;
  }

  function select(id, result, snapshot, advanced) {
    if (typeof id !== 'string' || !id.startsWith(PREFIX)) return false;
    const menu = owned;
    if (id === menu?.advanced && contract.current(menu.binding)) {
      dismiss(); advanced(); return true;
    }
    const target = menu?.choices.find(item => item.id === id);
    let ok = false;
    try {
      if (!target && receipt?.id === id) ok = contract.matches(contract.read(receipt.binding, modules), receipt.selection);
      else if (target) {
        ok = contract.apply(menu.binding, modules, target.selection, menu.live, menu.version);
        if (ok) {
          receipt = { id, binding: menu.binding, selection: target.selection };
          const updated = contract.catalog(menu.binding, modules);
          if (updated) emitCatalog(menu.binding, updated, menu.emit);
        }
      }
    } catch (_) { /* Never replay a possibly applied preference/model mutation through DOM. */ }
    if (!ok) { owned = null; receipt = null; }
    result('select_model_option', ok, ok ? '' : '模型或档位状态未能确认，请重新打开选择。');
    snapshot();
    return true;
  }

  function dismiss() {
    const handled = !!owned || !!pending;
    cancel(); owned = null; receipt = null;
    return handled;
  }

  return Object.freeze({ version: 1, request, select, dismiss });
});
