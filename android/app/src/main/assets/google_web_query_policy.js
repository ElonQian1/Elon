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
    return clean(observation.explicitQuery) ||
      clean(observation.rememberedQuery) ||
      clean(observation.urlQuery);
  }

  return Object.freeze({ version: 1, select });
});
