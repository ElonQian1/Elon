(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptRspackRuntime) {
    root.__elonChatGptRspackRuntime = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const CDN = 'https://chatgpt.com/cdn/assets/';
  const PROFILE = 'web_20260925_rspack';
  const RUNTIME = '633146.03cad12214.js';
  // Reviewed public assets. Only the existing document's executed module cache is read.
  const modules = {
    auth: ['238022.3eafa0ab02.js', 'OS'], scope: ['238022.3eafa0ab02.js', 'c3'],
    identity: ['586656.107574cbd4.js', 'sAW'], conversation: ['421899.a0eae5f5f4.js', 'LGwv'],
    composer: ['498514.f27755c4fc.js', 'vG'], submit: ['934244.56bcd8ce43.js', 'CUv'],
  };
  const importer = options.importModule || (url => import(url));
  let loader = null, pending = null, owner = null, code = 'not_observed', retryAt = 0;

  function documentKey() {
    const token = page.__elonChatGptDocumentToken;
    return page.location?.origin === 'https://chatgpt.com' && /^doc_[a-z0-9_]{3,80}$/.test(token || '') ? token : null;
  }
  function urls() {
    return new Set((page.performance?.getEntriesByType?.('resource') || []).map(item => item.name));
  }
  function observed() {
    return !!documentKey() && urls().has(CDN + RUNTIME);
  }
  function valid() {
    const names = urls();
    // These exact anchors identify the reviewed deployment, not an arbitrary Rspack build.
    return observed() && names.has(CDN + '908190.44b0fc59dd.js') &&
      names.has(CDN + '238022.3eafa0ab02.js');
  }
  function peek() {
    if (!valid() || !loader || owner !== documentKey()) return null;
    const result = {};
    const names = urls();
    for (const [role, [file, id]] of Object.entries(modules)) {
      const cached = loader.c?.[id];
      if (!names.has(CDN + file) || !cached || cached.error || !cached.exports) {
        code = role + '_pending'; return null;
      }
      result[role] = cached.exports;
    }
    if (typeof result.auth.getBrowserChatGptAuthSnapshot !== 'function' ||
        typeof result.auth.getBrowserChatGptAuthGeneration !== 'function' ||
        typeof result.auth.isBrowserWorkspaceSwitchPending !== 'function' ||
        typeof result.auth.isBrowserAccountSwitchLoading !== 'function' ||
        result.scope.a?.__scopeBrand !== 'AppScope' || typeof result.submit.a !== 'function' ||
        !['d', 'i', 'j'].every(key => result.identity[key]?.scope === result.scope.a) ||
        !['i', 'z', 'w', 'K', 'T'].every(key => result.conversation[key]?.scope === result.scope.a) ||
        !['r', 's', 'u', 'f', 'h'].every(key => result.composer[key]?.scope === result.scope.a)) {
      code = 'contract_mismatch'; return null;
    }
    code = 'ready'; return result;
  }
  function load() {
    const cached = peek();
    if (cached) return Promise.resolve(cached);
    if (pending) return pending;
    if (!valid()) { code = observed() ? 'build_unreviewed' : 'not_observed'; return Promise.resolve(null); }
    if (Date.now() < retryAt) return Promise.resolve(null);
    const token = documentKey();
    let timer;
    code = 'loading';
    pending = Promise.race([
      Promise.resolve().then(() => importer(CDN + RUNTIME)),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(Error('load_timeout')), 2000); }),
    ]).then(module => {
      if (documentKey() !== token || !valid()) { code = 'document_changed'; return null; }
      const value = module?.__webpack_require__;
      if (typeof value !== 'function' || !value.c || !value.m) { code = 'loader_mismatch'; return null; }
      // Do not call require(id): doing so could create another store or execute a write.
      loader = value; owner = token;
      return peek();
    }).catch(() => { code = 'load_failed'; retryAt = Date.now() + 10000; return null; })
      .finally(() => { page.clearTimeout(timer); pending = null; });
    return pending;
  }
  return Object.freeze({ version: 1, profile: PROFILE, observed, load, peek,
    state: () => ({ profile: PROFILE, code, pending: !!pending }) });
});
