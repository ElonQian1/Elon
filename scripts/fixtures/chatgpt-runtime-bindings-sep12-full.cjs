'use strict';

// Research baseline for every currently consumed export, including React and
// independent dispatch. This fixture does not enable a new production profile.
const base = require('./chatgpt-runtime-bindings-sep12');
module.exports = {
  ...base,
  files: { ...base.files, react: '2340486e-dyt4epctwx2pn2sj.js' },
  expectedExports: {
    ...base.expectedExports,
    react: { reactApi: 'zn', reactDom: 'Wt', reactRoot: 'Ut', intlInit: 'In', intlProvider: 'An' }
  },
  extraExports: {
    ...base.extraExports,
    shared: { ...base.extraExports.shared,
      canvasConversations: 'MP', useCanvasSendBlocked: 'tP',
      textApi: 'b4', textSecurityHeaders: 'ac', textHistoryDisabled: 'xJ',
      textModelOverride: 'Pc', textTopic: 'ej' },
    conversation: { ...base.extraExports.conversation,
      textSecurity: 'VKt', textStream: 'jGt', textPrepareEnabled: 'FKt',
      textReviewAck: 'MKt', textHydrateHistory: 'BEn' }
  }
};
