(function bootstrapUiTunerPwaAuth() {
  'use strict';
  const params = new URLSearchParams(window.location.search || '');
  if (params.get('ui_tuner_preview') !== '1' || window.parent === window) return;

  let token = '';
  try {
    const raw = window.localStorage.getItem('elon_auth');
    const parsed = raw ? JSON.parse(raw) : null;
    token = String(parsed && (parsed.token || (parsed.state && parsed.state.token)) || '');
  } catch (_) {
    token = '';
  }
  if (token.length > 8192) token = '';

  Object.defineProperty(window, '__ELON_UI_TUNER_PREVIEW_AUTH__', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({ token }),
  });
})();
