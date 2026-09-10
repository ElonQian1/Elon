(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 9, create: factory });
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
    composer: { Ih: 'Qh', t_: '__', AS: 'KS', VS: 'rC', Ng: 'Yg', Bg: 'n_', fh: 'Oh' }, react: {}
  };
  const september9Exports = {
    shared: { H3: 'L8', R5: 'N9', F5: 'A9', mq: 'UG', wV: 'eR', SV: 'QL', XM: 'SJ', HM: 'mJ',
      'M$': 'D0', RW: 'QB', uo: 'ws', t4: 'X3', IX: 'LU', t6: 'X8', cX: 'Jq',
      Fx: 'zS', Fl: 'Xu', v7: 'pet', $3: 'J8', Ur: 'Zi', zr: 'qi' },
    conversation: { AGt: 'uKt', J5t: 'a9t', Nrn: 'ran', yRt: '$Rt', Grn: 'han',
      vRt: 'QRt', p8t: 'w5t', l0: 'Z1', M1t: 's0t', Rdn: 'Fpn', Rrn: 'can',
      win: 'Jan', Ein: 'Xan', ay: 'ty', iy: 'ey', ry: '$v', Jrn: 'van', Hrn: 'fan',
      f8t: 'C5t', c0: 'X1', FVt: 'hHt', u1t: 'z1t', l1t: 'R1t', iin: 'Oan' },
    composer: { Ih: '$h', t_: 'v_', AS: 'qS', VS: 'iC', Ng: 'Xg', Bg: 'r_', fh: 'kh' }, react: {}
  };
  const september9bExports = {
    shared: { ...september9Exports.shared, H3: 'R8', R5: 'P9', F5: 'j9', t4: 'Z3',
      t6: 'Z8', v7: 'met', $3: 'Y8' },
    conversation: { AGt: 'uKt', J5t: 'o9t', Nrn: 'ian', yRt: '$Rt', Grn: 'gan',
      vRt: 'QRt', p8t: 'T5t', l0: 'Z1', M1t: 'c0t', Rdn: 'Lpn', Rrn: 'lan',
      win: 'Yan', Ein: 'Zan', ay: 'ty', iy: 'ey', ry: '$v', Jrn: 'yan', Hrn: 'pan',
      f8t: 'w5t', c0: 'X1', FVt: 'hHt', u1t: 'B1t', l1t: 'z1t', iin: 'kan' },
    composer: september9Exports.composer, react: {}
  };
  const september10Exports = {
    shared: { H3: 'o5', R5: 'F9', F5: 'M9', mq: 'UG', wV: 'tR', SV: '$L', XM: 'SJ', HM: 'mJ',
      'M$': 'D0', RW: '$B', uo: 'ws', t4: 'Z3', IX: 'LU', t6: 'P8', cX: 'Jq',
      Fx: 'BS', Fl: '$u', v7: 'het', $3: 'M8', Ur: 'Zi', zr: 'qi' },
    conversation: { AGt: 'LKt', J5t: 'V9t', Nrn: 'Ban', yRt: 'Ezt', Grn: 'Qan',
      vRt: 'Tzt', p8t: 'o7t', l0: 'C0', M1t: 'U0t', Rdn: 'xmn', Rrn: 'Gan',
      win: 'jon', Ein: 'Non', ay: 'xy', iy: 'by', ry: 'yy', Jrn: 'ton', Hrn: 'Yan',
      f8t: 'a7t', c0: 'S0', FVt: 'HHt', u1t: 'x0t', l1t: 'b0t', iin: 'don' },
    composer: { Ih: 'ig', t_: 'C_', AS: '$S', VS: 'uC', Ng: 't_', Bg: 'c_', fh: 'Ph' }, react: {}
  };
  const profiles = [
    { id: 'web_20260906', anchor: 'c2675c8c-f6cd0ubcb7y7eluj.js', files: legacy,
      tools: { owner: 'Whn' },
      temporary: { owner: 'AKt', action: '()=>{cg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(gB.reset(c),qg()&&$p.delete(n),!o&&!$p(n)&&OKt(s),u(jKt,{replace:!0})):oD(l,{params:o?void 0:new URLSearchParams({[zm]:`true`})})}' } },
    { id: 'web_20260907', anchor: 'c2675c8c-kconnwitb9zzv81k.js',
      files: { shared: '4813494d-o593jrji51wy4azk.js',
        conversation: 'conversation-small-owrec55n6vm0ekcc.js',
        composer: '8b34dbc2-nhot65scqrg20d6p.js', react: legacy.react },
      exports: currentExports,
      tools: { owner: 'Kgn' },
      temporary: { owner: 'vqt', action: '()=>{No.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(SB.reset(c),_t()&&fh.delete(n),!o&&!fh(n)&&gqt(s),u(yqt,{replace:!0})):Dj(l,{params:o?void 0:new URLSearchParams({[xn]:`true`})})}' } },
    { id: 'web_20260909', anchor: 'c2675c8c-k2kd9yafbfvx5mjw.js',
      files: { shared: '4813494d-e0hjx102gn5zjvdh.js',
        conversation: 'conversation-small-fka464yvjn19vebr.js',
        composer: '8b34dbc2-mx35vjavisrk7hwp.js', react: legacy.react },
      exports: september9Exports, tools: { owner: 'V_n' },
      temporary: { owner: 'SJt', action: '()=>{ym.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(dB.reset(c),a_()&&vp.delete(n),!o&&!vp(n)&&bJt(s),u(CJt,{replace:!0})):vx(l,{params:o?void 0:new URLSearchParams({[I_]:`true`})})}' } },
    { id: 'web_20260909_b', anchor: 'c2675c8c-lz0unwv5yke95cwv.js',
      files: { shared: '4813494d-bgyv5408fxme7xxv.js',
        conversation: 'conversation-small-hg48c5uox88r7a00.js',
        composer: '8b34dbc2-cj4kfo18e1ldvw16.js', react: legacy.react },
      exports: september9bExports, tools: { owner: 'H_n' },
      temporary: { owner: 'DJt', action: '()=>{eg.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(sB.reset(c),c_()&&xp.delete(n),!o&&!xp(n)&&TJt(s),u(OJt,{replace:!0})):mS(l,{params:o?void 0:new URLSearchParams({[G_]:`true`})})}' } },
    { id: 'web_20260910', anchor: 'c2675c8c-jb0yl2d3l1jp8c93.js',
      files: { shared: '4813494d-ilaxclpwvg5i0e40.js',
        conversation: 'conversation-small-ng27r04netsz3e4o.js',
        composer: '8b34dbc2-l0q54hwyus1gmzd1.js', react: legacy.react },
      exports: september10Exports, tools: { owner: 'Evn' },
      temporary: { owner: 'nYt', action: '()=>{ig.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(aB.reset(c),d_()&&dp.delete(n),!o&&!dp(n)&&eYt(s),u(rYt,{replace:!0})):Zj(l,{params:o?void 0:new URLSearchParams({[I_]:`true`})})}' } }
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
      return p ? Object.freeze({ ...p.tools }) : null;
    } catch (_) { return null; }
  }

  function state() {
    const p = profile();
    return { version: 9, profile_id: p?.id || '', cached_modules: cache.size, error };
  }

  return Object.freeze({ version: 9, observed, load, peek, temporary, tools, state });
});
