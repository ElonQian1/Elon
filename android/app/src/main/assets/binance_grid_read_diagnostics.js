/* Read-only page facts. Never send text, URLs, identifiers, headers or exception messages. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com' || window.__elonBinanceDiagnosticsV1) return;
  const counts = {list_requests: 0, list_responses: 0, identity_requests: 0, identity_responses: 0,
    detail_requests: 0, detail_responses: 0, legacy_list_requests: 0, script_errors: 0, script_load_errors: 0};
  const errors = new Set(['invalid_field', 'invalid_row', 'response_failed', 'business_failed', 'identity_unverified',
    'list_invalid', 'duplicate', 'account_mismatch', 'detail_mismatch', 'identity_response_failed', 'identity_business_failed']);
  let lastFailure = 'none', lastKind = 'none', lastStatus = 0;
  const bump = key => { if (Object.prototype.hasOwnProperty.call(counts, key)) counts[key] = Math.min(10000, counts[key] + 1); };
  window.addEventListener('error', event => {
    bump(event.target?.tagName === 'SCRIPT' ? 'script_load_errors' : 'script_errors');
  }, true);
  function visibleLabel(pattern) {
    return Array.from(document.querySelectorAll('button,a,[role="tab"],[role="button"]')).slice(0,256)
      .some(node => node.getClientRects().length > 0 && pattern.test((node.textContent || '').trim()));
  }
  window.__elonBinanceDiagnosticsV1 = Object.freeze({
    request(kind) { bump(kind + '_requests'); },
    response(kind, status) { bump(kind + '_responses'); lastStatus = Number.isInteger(status) && status >= 0 && status <= 599 ? status : 0; },
    failure(kind, code) { lastKind = ['list','identity','detail'].includes(kind) ? kind : 'none'; lastFailure = errors.has(code) ? code : 'transport_or_parse_failed'; },
    inspect(token) {
      if (!/^doc_[a-z0-9_]{3,80}$/.test(token)) return false;
      const route = location.pathname.startsWith('/zh-CN/trading-bots/futures/grid/') ? 'grid' :
        /\/(login|register)(\/|$)/.test(location.pathname) ? 'login' : 'other';
      const facts = {schema:'yilong.binance_diagnostic.v1', token, ...counts, last_failure:lastFailure,
        failure_kind:lastKind, last_http_status:lastStatus, route,
        ready_state:['loading','interactive','complete'].includes(document.readyState) ? document.readyState : 'unknown',
        script_count:Math.min(10000, document.scripts.length),
        body_text_length:Math.min(1000000, (document.body?.textContent || '').length),
        login_control:visibleLabel(/^(登录|Log In|Log in)$/),
        running_control:visibleLabel(/^(运行中|Running)(\s*\([0-9]+\))?$/)};
      window.ElonBinanceRead?.postMessage(JSON.stringify(facts));
      return true;
    }
  });
})();
