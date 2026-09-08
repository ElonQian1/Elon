'use strict';
const bindings = require('../../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
const CDN = 'https://chatgpt.com/cdn/assets/';
const files = {
  shared: '4813494d-o593jrji51wy4azk.js',
  conversation: 'conversation-small-owrec55n6vm0ekcc.js',
  composer: '8b34dbc2-nhot65scqrg20d6p.js'
};
// Independent expected export identities from the inspected public build.
const expectedExports = {
  shared: { H3: 'c6', R5: 'i7', F5: 't7', mq: 'Pq', wV: 'UV', SV: 'VV', XM: 'mN', HM: 'oN',
    'M$': 'Q$', RW: 'rG', uo: 'uo', t4: 'x4', IX: 'nZ', t6: 'x6', cX: 'OX',
    Fx: 'Lx', Fl: 'Il', v7: 'R7', $3: 'y6', Ur: 'Ur', zr: 'zr' },
  conversation: { AGt: 'uKt', J5t: 'O7t', Nrn: 'Cin', yRt: '$Rt', Grn: 'Fin',
    vRt: 'QRt', p8t: 'q8t', l0: 'E0', M1t: 'f0t', Rdn: 'Ofn', Rrn: 'Oin',
    win: 'man', Ein: 'gan', ay: 'Sy', iy: 'xy', ry: 'by', Jrn: 'Rin', Hrn: 'Min',
    f8t: 'K8t', c0: 'T0', FVt: 'hHt', u1t: 'W1t', l1t: 'U1t', iin: 'Jin' },
  composer: { Ih: 'Qh', t_: '__', AS: 'KS', VS: 'rC', Ng: 'Yg', Bg: 'n_' }
};

function attach(page, modules) {
  const loads = [];
  const observed = new Set(Object.values(files).map(name => CDN + name));
  observed.add(CDN + '2340486e-dyt4epctwx2pn2sj.js');
  if (!page.location.origin) Object.defineProperty(page.location, 'origin', {
    get: () => new URL(page.location.href).origin
  });
  page.performance = { ...page.performance, getEntriesByName: url => observed.has(url) ? [{}] : [] };
  page.__elonChatGptPrivateRuntimeBindings = bindings.create(page, { loadRuntime: async url => {
    loads.push(url);
    const role = Object.keys(files).find(role => url === CDN + files[role]);
    if (!role) throw Error('Unexpected module URL');
    return Object.fromEntries(Object.entries(modules[role] || {}).map(([key, value]) => {
      if (!expectedExports[role][key]) throw Error('Unmapped fixture export: ' + key);
      return [expectedExports[role][key], value];
    }));
  } });
  return { loads, observed, api: page.__elonChatGptPrivateRuntimeBindings };
}

module.exports = { attach, CDN, files, expectedExports };
