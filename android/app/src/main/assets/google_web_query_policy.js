(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebQueryPolicy ||
      Number(root.__elonGoogleWebQueryPolicy.version || 0) < api.version)) {
    root.__elonGoogleWebQueryPolicy = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim();
  }

  function select(observation) {
    if (!observation) return '';
    const remembered = clean(observation.rememberedQuery);
    if (observation.rememberedOwned === true && remembered) return remembered;
    return clean(observation.explicitQuery) ||
      clean(observation.urlQuery) ||
      remembered;
  }

  function isCarryOverAnswer(observation) {
    if (!observation || observation.rememberedOwned !== true) return false;
    const previousQuery = clean(observation.previousQuery);
    const nextQuery = clean(observation.nextQuery);
    const previousAnswer = clean(observation.previousAnswer);
    const nextAnswer = clean(observation.nextAnswer);
    return !!previousQuery && !!nextQuery && previousQuery !== nextQuery &&
      !!previousAnswer && previousAnswer === nextAnswer;
  }

  return Object.freeze({ version: 3, select, isCarryOverAnswer });
});
