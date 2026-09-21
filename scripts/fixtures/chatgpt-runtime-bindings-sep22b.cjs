'use strict';

// A second device-observed rollout. All consumed aliases retain their reviewed contracts.
const previous = require('./chatgpt-runtime-bindings-sep22.cjs');
module.exports = {
  ...previous, id: 'web_20260922_b', anchor: 'c2675c8c-irpzu8vf7ngp9dkh.js',
  files: { shared: '4813494d-l0x0s7iuq5eywkh0.js', conversation: 'conversation-small-fisxpb00fx88jy9f.js',
    composer: '8b34dbc2-bhyhfsqvfupfymrt.js', react: previous.files.react },
  hashes: { anchor: 'b6cfd5db68eb9465561f05287f5770f10711fa582ae827d2c1d42f421564009d',
    shared: '344103cb01f7987b190842ddaddb236a5ce6948d2d74c8f38b1933e15cd7cd08',
    conversation: '26dd6e6b5056a387917c2934ad6b152914e9d9f5356f2e4d227067b60375a552',
    composer: 'eb8f66c3d28f25d307de004887b46083cb7d2b1a3129a9f80e80b131d35743a5', react: previous.hashes.react },
  toolOwner: 'Wwn', temporary: { owner: 'Z$t',
    action: '()=>{mc.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(TB.reset(c),Am()&&Lh.delete(n),!o&&!Lh(n)&&Y$t(s),u(Q$t,{replace:!0})):sj(l,{params:o?void 0:new URLSearchParams({[im]:`true`})})}' }
};
