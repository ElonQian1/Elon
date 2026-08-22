(function () {
  'use strict';

  const base = window.__elonGoogleWebRichContent;
  const common = window.__elonRichContentDomAdapter;
  if (!base || !common || window.__elonWinGoogleRichContentInstalled) return;
  window.__elonWinGoogleRichContentInstalled = true;

  function parts(container, fallbackText, query) {
    const prose = typeof base.parts === 'function' ? base.parts(container, fallbackText, query) : [];
    const rich = common.parts(container);
    return prose.concat(rich).slice(0, 16);
  }

  function owns(node) {
    return Boolean(
      (typeof base.owns === 'function' && base.owns(node)) ||
      common.owns(node)
    );
  }

  window.__elonGoogleWebRichContent = Object.freeze(Object.assign({}, base, {
    version: Number(base.version || 0) + 100,
    parts,
    owns,
    fromAuthorizedEnvelope: common.fromAuthorizedEnvelope
  }));
})();
