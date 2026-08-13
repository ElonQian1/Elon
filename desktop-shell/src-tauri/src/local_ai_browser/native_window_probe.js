(function () {
  'use strict';
  if (window.__elonNativeWindowProbe) return;
  window.__elonNativeWindowProbe = true;

  function mountStatus(code, title, detail) {
    function render() {
      if (!document.body || document.getElementById('__elon_native_window_status__')) return;
      document.body.insertAdjacentHTML('beforeend', [
        '<div id="__elon_native_window_status__" style="position:fixed;inset:0;z-index:2147483647;background:#080b0a;color:#e9f7f1;display:grid;place-items:center;font-family:Segoe UI,Microsoft YaHei,sans-serif">',
        '<section style="width:min(520px,calc(100vw - 48px));border:1px solid #285a49;border-radius:16px;padding:28px;background:#101714;box-shadow:0 24px 80px rgba(0,0,0,.45)">',
        '<small style="color:#72e2b9;letter-spacing:.12em">' + code + '</small>',
        '<h1 style="font-size:22px;margin:12px 0 8px">' + title + '</h1>',
        '<p style="color:#a9bdb5;line-height:1.7;margin:0">' + detail + '</p>',
        '</section></div>'
      ].join(''));
    }
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', render, { once: true });
    } else {
      render();
    }
  }

  if (location.href === 'about:blank') {
    mountStatus('ELON-NATIVE-BOOT', '正在打开一龙 AI 窗口', '正在连接已登记的一龙聊天页面；若导航失败，本窗口会保留诊断结果。');
    return;
  }
  if (location.pathname !== '/pc/user-browser/native') return;

  function report(phase) {
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var invoke = internalInvoke || publicInvoke;
    if (typeof invoke !== 'function') return;
    var root = document.getElementById('root');
    Promise.resolve(invoke('publish_local_ai_native_window_health', {
      report: {
        phase: String(phase || '').slice(0, 40),
        readyState: String(document.readyState || '').slice(0, 16),
        rootExists: !!root,
        rootChildCount: root ? root.childElementCount : 0,
        route: location.pathname
      }
    })).catch(function () {});
  }

  report('script_started');
  document.addEventListener('DOMContentLoaded', function () { report('dom_content_loaded'); }, { once: true });
  window.addEventListener('load', function () { report('load'); }, { once: true });
  window.addEventListener('error', function () { report('window_error'); });
  window.addEventListener('unhandledrejection', function () { report('promise_rejection'); });
  window.setTimeout(function () {
    report('settled');
    var root = document.getElementById('root');
    if (!root || root.childElementCount === 0) {
      mountStatus('ELON-NATIVE-ROOT-EMPTY', '一龙聊天页面没有完成渲染', '窗口已保留。请在 Codex 控制台查看 native_window.page_health 事件，或关闭后重新打开。');
    }
  }, 8000);
})();
