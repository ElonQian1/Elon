(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptPageSemanticPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function classify(input) {
    const pathname = clean(input && input.pathname);
    const region = clean(input && input.region);
    const signal = clean(input && input.signal);

    if (/open[-\s]?sidebar|open sidebar|打开(?:侧边栏|边栏)/.test(signal)) {
      return 'navigation';
    }
    if (region === 'content' && /^\/scheduled(?:\/|$)/.test(pathname)) {
      return 'tasks';
    }
    return '';
  }

  return Object.freeze({ classify });
});
