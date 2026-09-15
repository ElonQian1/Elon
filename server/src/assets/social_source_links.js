(function (root) {
  'use strict';
  function webUrl(value) {
    if (typeof value !== 'string' || !/^https?:\/\//i.test(value) || new TextEncoder().encode(value).length > 4096 || /[\s\u0000-\u001f\u007f\\]/.test(value)) return null;
    try { const u = new URL(value); return ['http:', 'https:'].includes(u.protocol) && u.host && !u.username && !u.password ? value : null; } catch { return null; }
  }
  function source(value) { return value?.version === 1 && ['qr', 'share'].includes(value.method) && webUrl(value.url) ? value : null; }
  function label(url) { const u = new URL(url); return u.hostname === 'mp.weixin.qq.com' && (u.pathname === '/s' || u.pathname.startsWith('/s/')) ? '阅读原文' : '打开链接'; }
  function link(host, value) {
    const item = source(value); if (!item) return;
    const a = document.createElement('a'); a.href = item.url; a.target = '_blank'; a.rel = 'noopener noreferrer'; a.textContent = label(item.url) + ' · ' + new URL(item.url).hostname;
    a.style.cssText = 'display:block;color:#8ea7d5;min-height:44px;padding:10px 0;overflow-wrap:anywhere'; host.append(a);
  }
  async function scan(blob) {
    if (!blob.type.startsWith('image/') || blob.size > 12 * 1024 * 1024) return [];
    const bitmap = await createImageBitmap(blob);
    let worker;
    try {
      const scale = Math.min(1, 1800 / Math.max(bitmap.width, bitmap.height));
      const canvas = document.createElement('canvas'); canvas.width = Math.max(1, Math.round(bitmap.width * scale)); canvas.height = Math.max(1, Math.round(bitmap.height * scale));
      const ctx = canvas.getContext('2d', { willReadFrequently: true }); ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const frame = ctx.getImageData(0, 0, canvas.width, canvas.height);
      worker = new Worker('/assets/social_source_worker.js');
      const values = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('二维码识别超时，可稍后手动识别')), 8000);
        worker.onmessage = event => { clearTimeout(timer); resolve(event.data); };
        worker.onerror = () => { clearTimeout(timer); reject(new Error('二维码识别暂不可用')); };
        worker.postMessage({ pixels: frame.data, width: frame.width, height: frame.height }, [frame.data.buffer]);
      });
      return [...new Set(values)].filter(webUrl).map(url => ({ version: 1, url, method: 'qr' }));
    } finally { bitmap.close(); worker?.terminate(); }
  }
  function image(host, attachment, url) {
    link(host, attachment.source_link);
    const action = document.createElement('button'); action.type = 'button'; action.textContent = '识别二维码'; action.style.minHeight = '44px';
    action.hidden = true;
    const picture = host.querySelector('img');
    if (picture) {
      picture.tabIndex = 0; picture.setAttribute('aria-label', '图片，长按或按菜单键识别二维码');
      picture.oncontextmenu = event => { event.preventDefault(); action.hidden = false; action.focus(); };
      picture.onkeydown = event => { if (event.key === 'Enter' || event.key === 'ContextMenu') { event.preventDefault(); action.hidden = false; action.focus(); } };
    }
    const result = document.createElement('div'); result.setAttribute('role', 'status');
    action.onclick = async () => {
      action.disabled = true; result.textContent = '正在本地识别…';
      try {
        const response = await fetch(url, { signal: AbortSignal.timeout(15000) }); if (!response.ok) throw new Error('图片无法读取，请尝试原图');
        const found = await scan(await response.blob()); result.textContent = found.length ? '选择要打开的链接' : '未识别到网页二维码，可尝试原图或复制链接';
        found.forEach(item => link(result, item));
      } catch (error) { result.textContent = error.message; } finally { action.disabled = false; }
    };
    host.append(action, result);
  }
  root.ElonSourceLinks = { webUrl, source, label, link, scan, image, text(host, text) { const url = webUrl(text.trim()); if (url) link(host, { version: 1, url, method: 'share' }); } };
})(globalThis);
