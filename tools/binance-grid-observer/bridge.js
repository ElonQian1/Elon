(function () {
  'use strict';
  if (globalThis.__binanceGridBridgeV1) return;
  const sanitizer = globalThis.BinanceGridSanitizer;
  const origin = 'https://www.binance.com';
  let current = null;
  const validPage = () => location.origin === origin && window.top === window &&
    /^(?:\/[a-z]{2}(?:-[a-zA-Z]{2})?)?\/(?:trading-bots\/futures\/grid(?:\/|$)|futures(?:\/|$))/.test(location.pathname);
  function stop() { current = null; return { active: false }; }
  function start(sessionId) {
    if (!validPage() || !/^[a-zA-Z0-9_-]{16,128}$/.test(sessionId || '')) return stop();
    current = { sessionId, path: location.pathname, expiresAt: Date.now() + 900000, minute: Date.now(), count: 0 };
    return { active: true };
  }
  window.addEventListener('pagehide', stop);
  window.addEventListener('message', (event) => {
    const capture = current;
    if (!capture) return;
    if (!validPage() || capture.path !== location.pathname || Date.now() >= capture.expiresAt) { stop(); return; }
    if (event.source !== window || event.origin !== origin || event.data?.channel !== sanitizer.CHANNEL ||
        event.data?.sessionId !== capture.sessionId) return;
    if (Date.now() - capture.minute >= 60000) { capture.minute = Date.now(); capture.count = 0; }
    if (++capture.count > 120) return;
    try {
      if (JSON.stringify(event.data).length > 16384) return;
      const observation = sanitizer.sanitizeObservation(event.data.observation);
      if (!observation) return;
      chrome.runtime.sendMessage({ type: 'observer-sample', sessionId: capture.sessionId, observation }).catch(() => {});
    } catch { /* A malformed observation never reaches extension storage. */ }
  });
  globalThis.__binanceGridBridgeV1 = Object.freeze({ start, stop });
})();
