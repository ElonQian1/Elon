(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && !root.__elonGoogleWebAnswerCandidatePolicy) {
    root.__elonGoogleWebAnswerCandidatePolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function nonNegative(value) {
    const number = Number(value);
    return Number.isFinite(number) ? Math.max(0, number) : 0;
  }

  function accepts(metrics) {
    const textLength = nonNegative(metrics && metrics.textLength);
    const citations = nonNegative(metrics && metrics.citations);
    const semanticBlocks = nonNegative(metrics && metrics.semanticBlocks);
    const links = nonNegative(metrics && metrics.links);
    const tabControls = nonNegative(metrics && metrics.tabControls);
    const explicit = metrics && metrics.explicit === true;
    if (textLength < 8) return false;
    if (tabControls > 0 && semanticBlocks === 0 && citations === 0) return false;
    if (links >= 3 && semanticBlocks === 0 && citations === 0) return false;
    if (!explicit && semanticBlocks === 0 && citations === 0 && textLength < 80) return false;
    return true;
  }

  function penalty(metrics) {
    const links = nonNegative(metrics && metrics.links);
    const tabControls = nonNegative(metrics && metrics.tabControls);
    return Math.min(links, 20) * 180 + Math.min(tabControls, 10) * 600;
  }

  return Object.freeze({ version: 1, accepts, penalty });
});
