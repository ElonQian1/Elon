/* Win host bridge for the shared read-only Binance adapters. Requests stay in the page;
 * only adapter-validated observation JSON crosses to the native side. */
(function () {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com') return;
  if (window.__elonBinanceWinBridge) return;

  function invoke(command, args) {
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call !== 'function') return;
    Promise.resolve(call(command, args)).catch(function () {});
  }
  function post(payload) {
    if (typeof payload !== 'string' || payload.length > 1048576) return;
    invoke('publish_local_ai_web_event', { payload: payload });
  }

  function documentToken() {
    var words = new Uint32Array(4);
    window.crypto.getRandomValues(words);
    return 'doc_win_' + Array.from(words, function (word) {
      return word.toString(16).padStart(8, '0');
    }).join('');
  }
  var token = documentToken();

  // Same surface the Android WebView installs through addWebMessageListener.
  window.ElonBinanceRead = Object.freeze({ postMessage: post });

  function result(action, ok, detail, requestId) {
    post(JSON.stringify({
      type: 'command_result', action: action, ok: ok === true,
      detail: String(detail || '').slice(0, 240), requestId: requestId || null
    }));
  }
  function parseValue(value) {
    try { return JSON.parse(String(value || '')); } catch (_) { return null; }
  }

  function bind() {
    var read = window.__elonBinanceReadV1;
    if (read && typeof read.bind === 'function') {
      read.bind(token);
      post(JSON.stringify({ type: 'adapter_ready', token: token }));
    } else {
      post(JSON.stringify({ type: 'browser_diagnostic', kind: 'adapter_bootstrap_failed', detail: 'Binance 读取适配器未加载。' }));
    }
  }

  window.__elonBinanceWinBridge = Object.freeze({
    version: __ADAPTER_VERSION__,
    // Called by the host bootstrap once the shared adapters above it have been evaluated.
    bind: function () {
      if (document.readyState === 'loading') {
        window.addEventListener('DOMContentLoaded', bind, { once: true });
      } else {
        bind();
      }
    },
    command: function (raw) {
      var command = parseValue(raw);
      if (!command || typeof command.action !== 'string') return;
      var read = window.__elonBinanceReadV1;
      var requestId = typeof command.requestId === 'string' ? command.requestId : null;
      if (!read) { result(command.action, false, 'adapter_missing', requestId); return; }
      var ok = false;
      switch (command.action) {
        case 'refresh': ok = read.refresh() === true; break;
        case 'detail': ok = read.detail(String(command.value || '')) === true; break;
        case 'report': ok = read.report(parseValue(command.value)) === true; break;
        case 'wallet': ok = read.wallet(parseValue(command.value)) === true; break;
        case 'inspect': {
          var diagnostics = window.__elonBinanceDiagnosticsV1;
          var facts = diagnostics && diagnostics.inspect(token);
          ok = !!facts;
          if (facts) post(JSON.stringify(facts));
          break;
        }
        default: result(command.action, false, 'unsupported_action', requestId); return;
      }
      result(command.action, ok, ok ? '' : 'adapter_rejected', requestId);
    }
  });
})();
