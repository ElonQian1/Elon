'use strict';
// Exact anchor observed on normal 1679; public AST and stop-owner dependency review.
const files = {
  shared: '4813494d-gf2h57w5fiay19bd.js',
  conversation: 'conversation-small-h1dtzoris1y9588z.js',
  composer: '8b34dbc2-fqgb3eqijpn96umi.js'
};
const expectedExports = {
  shared: { H3: 'J5', R5: 'Cet', F5: 'bet', mq: 'bK', wV: 'AR', SV: 'OR', XM: '$J', HM: 'GJ',
    'M$': 'o2', RW: 'AV', uo: 'Fs', t4: 'R6', IX: 'hW', t6: 'S5', cX: 'TJ',
    Fx: 'aC', Fl: 'md', v7: 'rtt', $3: 'b5', Ur: 'ca', zr: 'ia' },
  conversation: { AGt: 'gJt', J5t: 'rnn', Nrn: 'Wsn', yRt: 'iVt', Grn: 'ncn',
    vRt: 'rVt', p8t: 'Sen', l0: 'z2', M1t: 'P4t', Rdn: 'Ggn', Rrn: 'Ysn',
    win: 'Fcn', Ein: 'Lcn', ay: 'ub', iy: 'lb', ry: 'cb', Jrn: 'acn', Hrn: '$sn',
    f8t: 'xen', c0: 'R2', FVt: 'xWt', u1t: 'f4t', l1t: 'd4t', iin: 'hcn' },
  composer: { Ih: 'ig', t_: 'x_', AS: 'ZS', VS: 'cC', Ng: '$g', Bg: 'o_' }
};
const extraExports = {
  shared: { attachmentUploadType: 'am', conversationStore: 'pY', canvasQueryClient: 'h2',
    textBusinessContext: 'JO', textProjectHeaders: 'iK', textLockedProjectId: 'Yr', textLockedChatPin: 'aK',
    textBindConversationId: 'IP', textClientConversation: 'gY', textResolvedConversationId: 'QJ',
    textNavigationKey: 'HK', textNavigate: 'KK',
    textTemporaryPersonalizationEnabled: 'wJ', textTemporaryPersonalization: 'OJ', textReadUntracked: 'f2',
    writingUpdateState: 'sY', writingTreeOwner: 'KJ' },
  conversation: { attachmentBaseLimit: 'W$t', attachmentMaxUploads: 'Z$t',
    attachmentPendingCount: 'K$t', attachmentConfiguredLimit: 'U$t', canvasDirtyInit: 'Dvt', useCanvasDirty: 'Avt',
    textRequestedDefaultModel: 'Jsn', textRememberFirstModel: 'DDn', textNavigateConversation: 'qHt' },
  composer: { fh: 'Nh' }
};
const anchor = 'c2675c8c-o59yc0xo7p9m3q3o.js';
const temporary = { owner: 'JYt', action: '()=>{Bd.logEvent(`Temporary Chat Move: Temporary Chat Button Clicked`),a?(rB.reset(c),ev()&&Cs.delete(n),!o&&!Cs(n)&&KYt(s),u(YYt,{replace:!0})):pw(l,{params:o?void 0:new URLSearchParams({[lo]:`true`})})}' };
module.exports = { files, expectedExports, extraExports, anchor, temporary, id: 'web_20260912', toolOwner: 'myn' };
