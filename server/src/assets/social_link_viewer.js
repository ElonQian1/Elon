(function (root) {
  'use strict';
  let closeCurrent = null;
  function external(url) { root.open(url, '_blank', 'noopener,noreferrer'); }
  function frameSource(embed) {
    const safe = root.ElonSocialLinks.trustedEmbed(embed); if (!safe) return null;
    if (safe.kind !== 'x') return { src: safe.url };
    // Only a validated numeric ID enters this document. No remote oEmbed HTML is injected.
    return { srcdoc: `<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="referrer" content="no-referrer"><style>body{margin:16px;background:#15171b;color:#eceef2;color-scheme:dark}a{color:#a9c9ed}</style></head><body><blockquote class="twitter-tweet" data-theme="dark" data-dnt="true"><a href="https://x.com/i/status/${safe.id}">在 X 查看原帖</a></blockquote><script async src="https://platform.x.com/widgets.js"></script></body></html>` };
  }
  function open(preview) {
    const url = root.ElonSocialLinks.safeUrl(preview.url); if (!url) return;
    const source = frameSource(preview.embed);
    if (!source) { external(url.href); return; }
    closeCurrent?.();
    const focus = document.activeElement;
    const dialog = document.createElement('dialog'); dialog.className = 'social-link-dialog'; dialog.setAttribute('aria-label', preview.title || preview.site);
    const bar = document.createElement('div'); bar.className = 'social-link-toolbar';
    const title = document.createElement('strong'); title.textContent = preview.title || preview.site;
    const original = document.createElement('a'); original.href = url.href; original.target = '_blank'; original.rel = 'noopener noreferrer'; original.textContent = '打开原文';
    const reload = document.createElement('button'); reload.type = 'button'; reload.textContent = '重试';
    const close = document.createElement('button'); close.type = 'button'; close.textContent = '关闭';
    const frame = document.createElement('iframe'); frame.title = preview.title || preview.site;
    frame.referrerPolicy = 'no-referrer'; frame.allow = 'fullscreen; encrypted-media; picture-in-picture';
    frame.setAttribute('sandbox', `allow-scripts ${source.srcdoc ? '' : 'allow-same-origin '}allow-presentation allow-popups allow-popups-to-escape-sandbox`);
    const note = document.createElement('p'); note.className = 'social-link-viewer-note'; note.textContent = '正在打开…'; note.setAttribute('role', 'status');
    let timer;
    function load() {
      clearTimeout(timer); note.textContent = '正在打开…';
      frame.removeAttribute('src'); frame.removeAttribute('srcdoc'); Object.assign(frame, source);
      timer = setTimeout(() => { note.textContent = '若内容未显示，请重试或打开原文。'; }, 8000);
    }
    frame.onload = () => { clearTimeout(timer); note.textContent = '内容由原平台提供；无法播放或需要登录时，可打开原文。'; };
    function finish() { clearTimeout(timer); frame.src = 'about:blank'; dialog.remove(); closeCurrent = null; if (focus?.isConnected) focus.focus({ preventScroll: true }); }
    close.onclick = () => dialog.close(); dialog.onclose = finish;
    dialog.onclick = event => { if (event.target === dialog) dialog.close(); };
    closeCurrent = () => dialog.close(); reload.onclick = load;
    bar.append(title, original, reload, close); dialog.append(bar, note, frame); document.body.append(dialog);
    dialog.showModal(); close.focus(); load();
  }
  root.ElonSocialLinkViewer = { open, close: () => closeCurrent?.(), frameSource };
})(globalThis);
