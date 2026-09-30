(function (root) {
  'use strict';
  const android = () => /Android/i.test(root.navigator?.userAgent || '');
  function intent(value, url, now = Date.now()) {
    if (value?.schema !== 1 || !root.ElonSocialLinks.channelsId(url) ||
        root.ElonSocialLinks.channelsId(value.source_url) !== root.ElonSocialLinks.channelsId(url) ||
        !Number.isSafeInteger(value.expires_at_ms) || value.expires_at_ms <= now + 1000 ||
        typeof value.launch_url !== 'string' || value.launch_url.length > 8192) return null;
    const prefix = 'weixin://biz/finder/openFinderFeed/';
    if (!value.launch_url.startsWith(prefix)) return null;
    const encoded = value.launch_url.slice(prefix.length);
    // This scene is a single encoded path component, never arbitrary Intent extras.
    if (!/^[A-Za-z0-9_%+.-]+$/.test(encoded)) return null;
    let scene; try { scene = new URLSearchParams(decodeURIComponent(encoded.replace(/\+/g, '%20'))); } catch { return null; }
    if (!/^export\/[A-Za-z0-9_-]{8,2048}$/.test(scene.get('exportId') || '') || scene.get('actionType') !== '0') return null;
    const allowed = ['exportId', 'actionType', 'commentScene', 'entryScene', 'entryCardType', 'requestScene'];
    for (const [key, val] of scene) {
      if (!allowed.includes(key) || scene.getAll(key).length !== 1 ||
          (key !== 'exportId' && key !== 'actionType' && (!/^\d{1,7}$/.test(val) || Number(val) > 1000000))) return null;
    }
    return 'intent://biz/finder/openFinderFeed/' + encoded + '#Intent;scheme=weixin;package=com.tencent.mm;end';
  }
  function create(host, current, options) {
    if (!android()) return null;
    let active = true, controller, timer, pending, ready, generation = 0;
    const bar = document.createElement('div'); bar.className = 'social-link-handoff'; bar.hidden = true;
    const status = document.createElement('span'); status.setAttribute('role', 'status'); status.setAttribute('aria-live', 'polite');
    const launch = document.createElement('button'); launch.type = 'button'; launch.textContent = '在微信打开';
    const copy = document.createElement('button'); copy.type = 'button'; copy.textContent = '复制链接';
    const cancel = document.createElement('button'); cancel.type = 'button'; cancel.textContent = '取消'; cancel.hidden = true;
    bar.append(status, launch, copy, cancel); host.append(bar);
    const valid = () => active && options.isCurrent() && !document.hidden;
    function stop() { generation++; controller?.abort(); clearTimeout(timer); controller = null; pending = null; ready = null; launch.disabled = false; cancel.hidden = true; }
    function jump(url) {
      if (!valid()) return;
      ready = null;
      status.textContent = '已请求打开微信';
      // Stay in this PWA document. No intermediate browser tab, HTTP navigation, or guessed callback.
      try { root.location.assign(url); } catch { status.textContent = '未能打开微信，请重试或复制链接'; }
    }
    async function open() {
      if (!valid() || pending) return;
      bar.hidden = false;
      const url = current().url;
      const cached = ready && intent(ready, url);
      if (cached) { jump(cached); return; }
      ready = null; const ticket = ++generation;
      controller = new AbortController(); const signal = controller.signal;
      timer = setTimeout(() => controller?.abort(), 12000);
      launch.disabled = true; cancel.hidden = false; status.textContent = '正在准备微信视频';
      pending = Promise.race([
        Promise.resolve().then(() => options.api('/api/me/link-preview/wechat-open', { method: 'POST', body: JSON.stringify({ url }), signal })),
        new Promise((_, reject) => signal.addEventListener('abort', () => reject(new Error('cancelled')), { once: true })),
      ]);
      try {
        let value = await pending;
        if (typeof value?.json === 'function') { if (!value.ok) throw new Error('unavailable'); value = await value.json(); }
        if (!valid() || generation !== ticket || signal.aborted) return;
        const target = intent(value, url);
        if (!target) throw new Error('invalid scene');
        ready = value;
        if (root.navigator.userActivation?.isActive) jump(target);
        else status.textContent = '已准备好，请点击“在微信打开”';
      } catch {
        if (valid() && generation === ticket) status.textContent = '暂时无法打开微信，请重试或复制链接';
      } finally {
        if (generation === ticket) { clearTimeout(timer); pending = null; controller = null; launch.disabled = false; cancel.hidden = true; }
      }
    }
    launch.onclick = open;
    cancel.onclick = () => { stop(); bar.hidden = true; };
    copy.onclick = async () => {
      const ticket = generation;
      try { await root.navigator.clipboard.writeText(current().url); if (valid() && ticket === generation) status.textContent = '链接已复制'; }
      catch { if (valid() && ticket === generation) status.textContent = '复制失败，可长按卡片复制链接'; }
    };
    const visibility = () => { if (document.hidden) stop(); };
    const pagehide = () => stop();
    document.addEventListener('visibilitychange', visibility); root.addEventListener('pagehide', pagehide);
    return { open, dispose() { active = false; stop(); document.removeEventListener('visibilitychange', visibility); root.removeEventListener('pagehide', pagehide); bar.remove(); } };
  }
  root.ElonWechatHandoff = { create, intent };
})(globalThis);
