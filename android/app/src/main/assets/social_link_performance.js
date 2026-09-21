(function () {
  'use strict';
  if (window.top !== window || location.protocol !== 'https:') return;
  const key = '__elonLinkPerformance';
  if (window[key] && window[key].version === 1) return;
  const observers = [], failures = [];
  let errors = 0, rejections = 0, longTasks = 0, longTaskMs = 0, longestTaskMs = 0, lcp = 0, stopped = false;
  const number = value => Number.isFinite(value) ? Math.max(0, Math.min(value, 600000)) : 0;
  const host = value => { try { return new URL(value, location.href).hostname; } catch (_) { return ''; } };
  function error(event) {
    if (stopped) return;
    const target = event.target;
    if (target && target !== window && (target.src || target.href)) {
      const domain = host(target.src || target.href);
      if (domain && failures.length < 8 && !failures.includes(domain)) failures.push(domain);
    } else errors = Math.min(100, errors + 1);
  }
  function rejection() { if (!stopped) rejections = Math.min(100, rejections + 1); }
  function observe(type, callback) {
    try {
      const observer = new PerformanceObserver(list => { if (!stopped) callback(list.getEntries()); });
      observer.observe({ type, buffered: true }); observers.push(observer);
    } catch (_) { /* Older WebView: retain navigation timing and native errors. */ }
  }
  observe('longtask', entries => entries.forEach(entry => {
    longTasks = Math.min(10000, longTasks + 1);
    longTaskMs = number(longTaskMs + entry.duration);
    longestTaskMs = Math.max(longestTaskMs, number(entry.duration));
  }));
  observe('largest-contentful-paint', entries => entries.forEach(entry => { lcp = number(entry.startTime); }));
  window.addEventListener('error', error, true);
  window.addEventListener('unhandledrejection', rejection);
  function stop() {
    if (stopped) return;
    stopped = true; observers.forEach(observer => observer.disconnect());
    window.removeEventListener('error', error, true);
    window.removeEventListener('unhandledrejection', rejection);
    window.removeEventListener('pagehide', stop);
    clearTimeout(expiry);
  }
  function visible(element) {
    if (!element || !element.getClientRects().length) return false;
    const rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return false;
    for (let current = element, depth = 0; current && depth < 8; current = current.parentElement, depth++) {
      const style = getComputedStyle(current);
      if (style.display === 'none' || style.visibility === 'hidden' || style.visibility === 'collapse' || Number(style.opacity) === 0) return false;
    }
    return true;
  }
  function snapshot() {
    const navigation = performance.getEntriesByType('navigation')[0];
    const paints = performance.getEntriesByType('paint');
    const fcp = paints.find(entry => entry.name === 'first-contentful-paint');
    const wechat = location.hostname === 'mp.weixin.qq.com' && /^\/s(?:\/|$)/.test(location.pathname);
    const article = wechat ? document.getElementById('js_content') : null;
    const contentState = wechat ? (!article ? 'absent' : visible(article) ? 'visible' : 'hidden') : (fcp ? 'painted' : 'pending');
    // Read layout for one known article container; never read body text or walk the document.
    const contentVisible = wechat ? contentState === 'visible' : !!fcp || (document.readyState === 'complete' && visible(document.body));
    const result = {
      schema: 1, ready_state: document.readyState, content_state: contentState, content_visible: contentVisible,
      fcp_ms: fcp ? number(fcp.startTime) : null, lcp_ms: lcp || null,
      js_errors: errors, promise_rejections: rejections, long_tasks: longTasks,
      long_task_ms: longTaskMs, longest_task_ms: longestTaskMs, resource_error_hosts: failures.slice(),
      slow_resources: performance.getEntriesByType('resource').slice(-300)
        .filter(entry => entry.duration >= 100).sort((a, b) => b.duration - a.duration).slice(0, 8)
        .map(entry => ({ host: host(entry.name), kind: entry.initiatorType, duration_ms: number(entry.duration) }))
    };
    if (navigation) Object.assign(result, {
      dns_ms: number(navigation.domainLookupEnd - navigation.domainLookupStart),
      connect_ms: number(navigation.connectEnd - navigation.connectStart),
      ttfb_ms: navigation.responseStart ? number(navigation.responseStart - navigation.requestStart) : null,
      response_end_ms: navigation.responseEnd ? number(navigation.responseEnd) : null,
      dom_interactive_ms: navigation.domInteractive || null,
      dcl_ms: navigation.domContentLoadedEventEnd || null, load_ms: navigation.loadEventEnd || null
    });
    return result;
  }
  const expiry = setTimeout(stop, 45000);
  window.addEventListener('pagehide', stop, { once: true });
  Object.defineProperty(window, key, { value: { version: 1, snapshot, stop }, configurable: true });
})();
