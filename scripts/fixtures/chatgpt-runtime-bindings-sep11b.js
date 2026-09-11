'use strict';
// Public anchor observed on normal APK 1654; reviewed AST/import evidence, not guessed aliases.
const files = {
  shared: '4813494d-c6b4nsqqwi13e6rd.js',
  conversation: 'conversation-small-ft205i7yqa6zc2nj.js',
  composer: '8b34dbc2-fpy4mlfnxc115y6k.js'
};
const expectedExports = {
  shared: { H3: 'D5', R5: 'net', F5: '$9', mq: 'rK', wV: 'pR', SV: 'dR', XM: 'IJ', HM: 'OJ',
    'M$': 'U0', RW: 'dV', uo: 'ws', t4: 'v6', IX: 'XU', t6: 't5', cX: 'cJ',
    Fx: 'GS', Fl: 'td', v7: 'Iet', $3: '$8', Ur: 'Zi', zr: 'qi' },
  conversation: { AGt: 'Bqt', J5t: 'ctn', Nrn: 'Yon', yRt: 'kBt', Grn: 'ssn',
    vRt: 'OBt', p8t: 'D9t', l0: 'b2', M1t: 'o4t', Rdn: 'Fhn', Rrn: 'esn',
    win: 'Bsn', Ein: 'Hsn', ay: 'Qy', iy: 'Zy', ry: 'Xy', Jrn: 'usn', Hrn: 'isn',
    f8t: 'E9t', c0: 'y2', FVt: 'GUt', u1t: 'R2t', l1t: 'L2t', iin: 'bsn' },
  composer: { Ih: 'og', t_: 'T_', AS: 'tC', VS: 'fC', Ng: 'r_', Bg: 'u_' }
};
const extraExports = {
  shared: { attachmentUploadType: 'Gp', conversationStore: 'XJ' },
  conversation: { attachmentBaseLimit: 'g$t', attachmentMaxUploads: 'C$t',
    attachmentPendingCount: 'v$t', attachmentConfiguredLimit: 'h$t' },
  composer: { fh: 'Fh' }
};
const anchor = 'c2675c8c-m4ftlj32vtu9aroq.js';
const temporary = { owner: 'Kqt', action: '()=>{P.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(DV.reset(c),W_()&&Im.delete(n),!o&&!Im(n)&&Wqt(s),u(qqt,{replace:!0})):PP(l,{params:o?void 0:new URLSearchParams({[wh]:`true`})})}' };
module.exports = { files, expectedExports, extraExports, anchor, temporary, id: 'web_20260911_b', toolOwner: 'Pvn' };
