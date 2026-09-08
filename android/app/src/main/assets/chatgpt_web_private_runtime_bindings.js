(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 5, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      !(Number(root.__elonChatGptPrivateRuntimeBindings?.version) >= api.version)) {
    root.__elonChatGptPrivateRuntimeBindings = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const CDN = 'https://chatgpt.com/cdn/assets/';
  const legacy = Object.freeze({
    shared: '4813494d-hrplraurzfyvxb10.js',
    conversation: 'conversation-small-hiw4wce20lu6te81.js',
    composer: '8b34dbc2-kjj15hg4y6iyx13p.js',
    react: '2340486e-dyt4epctwx2pn2sj.js'
  });
  // Exact public source contracts, not wildcard imports or guessed minified names.
  // Evidence and hashes: docs/chatgpt-private-runtime-bindings.md.
  const currentExports = {
    shared: { H3: 'c6', R5: 'i7', F5: 't7', mq: 'Pq', wV: 'UV', SV: 'VV', XM: 'mN', HM: 'oN',
      'M$': 'Q$', RW: 'rG', uo: 'uo', t4: 'x4', IX: 'nZ', t6: 'x6', cX: 'OX',
      Fx: 'Lx', Fl: 'Il', v7: 'R7', $3: 'y6', Ur: 'Ur', zr: 'zr' },
    conversation: { AGt: 'uKt', J5t: 'O7t', Nrn: 'Cin', yRt: '$Rt', Grn: 'Fin',
      vRt: 'QRt', p8t: 'q8t', l0: 'E0', M1t: 'f0t', Rdn: 'Ofn', Rrn: 'Oin',
      win: 'man', Ein: 'gan', ay: 'Sy', iy: 'xy', ry: 'by', Jrn: 'Rin', Hrn: 'Min',
      f8t: 'K8t', c0: 'T0', FVt: 'hHt', u1t: 'W1t', l1t: 'U1t', iin: 'Jin' },
    composer: { Ih: 'Qh', t_: '__', AS: 'KS', VS: 'rC', Ng: 'Yg', Bg: 'n_' }, react: {}
  };
  const profiles = [
    { id: 'web_20260906', anchor: 'c2675c8c-f6cd0ubcb7y7eluj.js', files: legacy,
      temporary: { owner: 'AKt', action: '()=>{cg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(gB.reset(c),qg()&&$p.delete(n),!o&&!$p(n)&&OKt(s),u(jKt,{replace:!0})):oD(l,{params:o?void 0:new URLSearchParams({[zm]:`true`})})}' } },
    { id: 'web_20260907', anchor: 'c2675c8c-kconnwitb9zzv81k.js',
      files: { shared: '4813494d-o593jrji51wy4azk.js',
        conversation: 'conversation-small-owrec55n6vm0ekcc.js',
        composer: '8b34dbc2-nhot65scqrg20d6p.js', react: legacy.react },
      exports: currentExports,
      temporary: { owner: 'vqt', action: '()=>{No.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(SB.reset(c),_t()&&fh.delete(n),!o&&!fh(n)&&gqt(s),u(yqt,{replace:!0})):Dj(l,{params:o?void 0:new URLSearchParams({[xn]:`true`})})}' } }
  ];
  const roles = Object.keys(legacy);
  let document, token, selected, error = '';
  const cache = new Map(), resolved = new Map(), failedUntil = new Map();
  const now = options.now || (() => Date.now());

  function seen(file) {
    const url = CDN + file;
    return !!page.performance?.getEntriesByName?.(url, 'resource')?.length ||
      !!page.document?.querySelector?.('link[rel="modulepreload"][href="' + url + '"],script[type="module"][src="' + url + '"]');
  }

  function profile() {
    if (page.location?.origin !== 'https://chatgpt.com') return null;
    if (document !== page.document || token !== page.__elonChatGptDocumentToken) {
      document = page.document; token = page.__elonChatGptDocumentToken;
      selected = null; cache.clear(); resolved.clear(); failedUntil.clear(); error = '';
    }
    const candidates = profiles.filter(p => seen(p.anchor) ||
      ['shared', 'conversation', 'composer'].some(role => seen(p.files[role])));
    if (candidates.length > 1) { error = 'runtime_build_ambiguous'; return null; }
    if (candidates.length === 1) {
      if (selected && selected !== candidates[0]) {
        error = 'runtime_build_changed'; return null;
      }
      selected = candidates[0];
    }
    return selected || null;
  }

  function roleOf(url) {
    return roles.find(role => url === role || url === CDN + legacy[role]);
  }

  function observed(url) {
    try {
      const role = roleOf(url), p = role && profile();
      return !!p && (cache.has(role) || seen(p.files[role]) || seen(p.anchor));
    } catch (_) { return false; }
  }

  function expose(namespace, p, role) {
    // Never leave aliases unmapped: the same alias can be an unrelated function
    // in another website build. Consumer-specific validators still check shape.
    const map = currentExports[role];
    const result = Object.create(null);
    for (const name of Object.keys(map)) {
      const exported = p.exports ? p.exports[role][name] : name;
      if (namespace != null && Object.prototype.hasOwnProperty.call(namespace, exported)) {
        result[name] = namespace[exported];
      }
    }
    return Object.freeze(result);
  }

  function peek(url) {
    try {
      const role = roleOf(url);
      return role && profile() ? resolved.get(role) || null : null;
    } catch (_) { return null; }
  }

  function load(url) {
    let p, role;
    try { role = roleOf(url); p = role && profile(); } catch (_) {}
    if (!p || !role || role === 'react' || !observed(url)) {
      return Promise.reject(Error('runtime_not_observed'));
    }
    if (cache.has(role)) return cache.get(role);
    if (now() < (failedUntil.get(role) || 0)) return Promise.reject(Error('runtime_cooldown'));
    const owner = { document, token, profile: p };
    const importer = options.loadRuntime || (value => import(value));
    let timer;
    const promise = Promise.race([
      Promise.resolve().then(() => importer(CDN + p.files[role])),
      new Promise((_, reject) => { timer = page.setTimeout(() => reject(Error('runtime_timeout')),
        Math.min(5000, Math.max(100, options.timeoutMs || 1500))); })
    ]).then(namespace => {
      if (owner.document !== page.document || owner.token !== page.__elonChatGptDocumentToken || profile() !== p) {
        throw Error('runtime_context_changed');
      }
      const result = expose(namespace, p, role);
      if (!Object.keys(result).length) throw Error('runtime_exports_unknown');
      resolved.set(role, result);
      error = ''; return result;
    }).catch(reason => {
      if (cache.get(role) === promise) {
        cache.delete(role); failedUntil.set(role, now() + 10000);
        error = ['runtime_timeout', 'runtime_context_changed', 'runtime_exports_unknown'].includes(reason?.message)
          ? reason.message : 'runtime_load_failed';
      }
      throw reason;
    }).finally(() => page.clearTimeout(timer));
    cache.set(role, promise);
    return promise;
  }

  function temporary() {
    try { const p = profile(); return p ? Object.freeze({ ...p.temporary }) : null; }
    catch (_) { return null; }
  }

  function tools() {
    try {
      const p = profile();
      return p ? Object.freeze({ owner: p.id === 'web_20260907' ? 'Kgn' : 'Whn' }) : null;
    } catch (_) { return null; }
  }

  function state() {
    const p = profile();
    return { version: 5, profile_id: p?.id || '', cached_modules: cache.size, error };
  }

  return Object.freeze({ version: 5, observed, load, peek, temporary, tools, state });
});
