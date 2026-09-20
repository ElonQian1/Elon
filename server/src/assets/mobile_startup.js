// Boot recovery is independent of optional features and never clears account data.
(function (root) {
  'use strict';
  const memory = Object.create(null), removed = new Set();
  const storage = {
    getItem(key) {
      if (removed.has(key)) return null;
      if (Object.prototype.hasOwnProperty.call(memory, key)) return memory[key];
      try { return root.localStorage.getItem(key); } catch { return null; }
    },
    setItem(key, value) {
      memory[key] = String(value); removed.delete(key);
      try { root.localStorage.setItem(key, value); } catch { /* page-only session */ }
    },
    removeItem(key) {
      delete memory[key]; removed.add(key);
      try { root.localStorage.removeItem(key); } catch { /* page-only session */ }
    },
  };
  const safeStorage = new Proxy(storage, {
    ownKeys() {
      let keys = []; try { keys = Object.keys(root.localStorage); } catch {}
      return [...new Set(keys.concat(Object.keys(memory)))].filter(key => !removed.has(key));
    },
    getOwnPropertyDescriptor(_, key) { return { configurable: true, enumerable: true, value: storage.getItem(key) }; },
  });
  let ready = false, failed = false, watchdog;
  function mount() {
    if (!document.body || document.getElementById('mobileStartup') || ready) return;
    const panel = document.createElement('aside');
    panel.id = 'mobileStartup'; panel.setAttribute('role', 'status');
    panel.style.cssText = 'position:fixed;inset:0;z-index:2147483000;display:grid;place-content:center;gap:16px;padding:28px;background:#0b1017;color:#edf3f7;text-align:center;font:16px system-ui';
    const title = document.createElement('strong'); title.textContent = '一龙ai';
    const text = document.createElement('p'); text.id = 'mobileStartupText'; text.textContent = '正在打开聊天…';
    const retry = document.createElement('button'); retry.type = 'button'; retry.textContent = '重新打开';
    retry.style.cssText = 'padding:12px 24px;border:1px solid #667788;border-radius:12px;background:#243345;color:white;font:inherit';
    retry.onclick = () => {
      const target = new URL(location.href);
      target.searchParams.set('tab', 'chat'); target.searchParams.set('pwa_retry', String(Date.now()));
      location.replace(target.href);
    };
    retry.hidden = true; retry.id = 'mobileStartupRetry'; panel.append(title, text, retry); document.body.append(panel);
    if (failed) showFailure();
  }
  function showFailure() {
    if (ready) return;
    failed = true; mount();
    const text = document.getElementById('mobileStartupText'), retry = document.getElementById('mobileStartupRetry');
    if (text) text.textContent = navigator.onLine === false ? '当前没有网络。连接后可以重新打开聊天。' : '聊天暂时未能打开，请重试。账号和聊天缓存会保留。';
    if (retry) retry.hidden = false;
  }
  function complete() {
    ready = true; clearTimeout(watchdog); document.getElementById('mobileStartup')?.remove();
  }
  root.ElonMobileStartup = { storage: safeStorage, complete, fail: showFailure };
  document.addEventListener('DOMContentLoaded', mount, { once: true });
  root.addEventListener('error', event => { if (!ready && event.target === root) showFailure(); });
  root.addEventListener('unhandledrejection', () => { if (!ready) showFailure(); });
  watchdog = setTimeout(showFailure, 15000);
  if (root.isSecureContext && 'serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js').catch(() => {});
  }
})(globalThis);
