(function (root) {
  'use strict';
  const MAX_BYTES = 12 * 1024 * 1024;
  const node = (tag, text) => { const el = document.createElement(tag); if (text != null) el.textContent = text; return el; };
  function button(label, action) { const el = node('button', label); el.type = 'button'; el.onclick = action; return el; }
  function attachmentUrl(item) {
    const url = new URL(item.url, location.origin);
    // The HTTPS PWA ingress and the public HTTP attachment URL use different ports.
    // Reuse only a recognized same-host attachment path, never send auth to its origin.
    if (!['http:', 'https:'].includes(url.protocol) || url.hostname !== location.hostname || !/^\/api\/user\/[^/]+\/chat-attachments\//.test(url.pathname) || url.username || url.password) throw Error('附件地址不受信任，请下载原文件后重新发送');
    return url.pathname + url.search;
  }
  async function file(item, options, signal) {
    if (item.size_bytes > MAX_BYTES) throw Error('单个附件不能超过 12 MB');
    const response = await options.api(attachmentUrl(item), { signal, redirect: 'error' });
    if (!response.ok) throw Error('附件读取失败，请重试');
    if (Number(response.headers.get('content-length')) > MAX_BYTES) throw Error('附件超过大小限制');
    const reader = response.body.getReader(), chunks = []; let size = 0;
    try {
      while (true) {
        const { done, value } = await reader.read(); if (done) break;
        size += value.length; if (size > MAX_BYTES) throw Error('附件超过大小限制'); chunks.push(value);
      }
    } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
    if (!size || !options.current()) throw Error('附件为空或会话已变化');
    return new File(chunks, item.display_name || item.file_name || '附件', { type: item.mime_type || response.headers.get('content-type') || 'application/octet-stream' });
  }
  async function textCopy(value) {
    if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(value); return; }
    const input = node('textarea', value), focus = document.activeElement;
    input.style.cssText = 'position:fixed;width:1px;height:1px;opacity:0';
    (document.querySelector('dialog[open]') || document.body).append(input); input.select();
    try { if (!document.execCommand('copy')) throw Error('浏览器不允许复制，请选择文字复制'); }
    finally { input.remove(); focus?.focus({ preventScroll: true }); }
  }
  async function clipboardImage(item, options, signal, type) {
    const original = await file(item, options, signal);
    if (type === original.type) return original;
    const bitmap = await createImageBitmap(original);
    try {
      if (bitmap.width * bitmap.height > 16000000) throw Error('图片太大，无法复制，请转发原图');
      const canvas = document.createElement('canvas'); canvas.width = bitmap.width; canvas.height = bitmap.height;
      canvas.getContext('2d').drawImage(bitmap, 0, 0);
      const png = await new Promise(resolve => canvas.toBlob(resolve, 'image/png'));
      if (!png || png.size > MAX_BYTES || !options.current()) throw Error('图片无法复制，请转发原图');
      return png;
    } finally { bitmap.close(); }
  }
  async function copy(messages, options) {
    const attachments = messages.flatMap(m => m.attachments || []);
    const text = messages.map(m => m.content || '').filter(Boolean).join('\n\n');
    if (!attachments.length) { await textCopy(text); return; }
    if (attachments.length !== 1 || text) throw Error('图文或多附件请使用转发，不会只复制部分内容');
    const item = attachments[0], mime = item.mime_type || '';
    if (!navigator.clipboard?.write || !root.ClipboardItem || !['image/png', 'image/jpeg', 'image/webp'].includes(mime)) throw Error('当前浏览器不支持复制此图片格式，可转发或下载原图');
    const type = mime === 'image/png' || ClipboardItem.supports?.(mime) ? mime : 'image/png';
    if (ClipboardItem.supports && !ClipboardItem.supports(type)) throw Error('当前浏览器不支持复制此图片格式，可转发或下载原图');
    const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 15000);
    try {
      // Pass the promise during the activation event, including on WebKit.
      const blob = clipboardImage(item, options, controller.signal, type); blob.catch(() => {});
      await navigator.clipboard.write([new ClipboardItem({ [type]: blob })]);
    } finally { clearTimeout(timer); }
  }
  function forward(messages, options) {
    const dialog = node('dialog'); dialog.className = 'social-action-dialog'; dialog.setAttribute('aria-label', '转发消息');
    const focus = document.activeElement, status = node('p', '正在读取会话…'); status.setAttribute('role', 'status');
    const target = node('select'); target.setAttribute('aria-label', '转发到'); target.append(new Option('请选择会话', ''));
    const contacts = new Map(), uploaded = new Map(), controller = new AbortController(); let busy = false, uncertain = false, sent = 0;
    const current = () => dialog.isConnected && options.current() && !controller.signal.aborted;
    const close = () => { controller.abort(); clearInterval(guard); dialog.close(); dialog.remove(); if (options.current()) focus?.focus({ preventScroll: true }); };
    async function request(path, init = {}) {
      if (!current()) throw Error('会话已变化');
      const abort = new AbortController(), stop = () => abort.abort(), timer = setTimeout(stop, 30000);
      controller.signal.addEventListener('abort', stop, { once: true });
      try {
        const response = await options.api(path, { ...init, signal: abort.signal });
        const data = await response.json(); if (!response.ok) throw Error(data.error || '请求失败'); return data;
      } finally { clearTimeout(timer); controller.signal.removeEventListener('abort', stop); }
    }
    const cancel = button('取消', close), send = button('确认转发', async () => {
      if (busy || uncertain || !contacts.has(target.value) || !current()) return;
      const destination = contacts.get(target.value); busy = true; target.disabled = true; send.disabled = true;
      try {
        for (let i = sent; i < messages.length; i++) {
          const message = messages[i], refs = [], attachments = message.attachments || [];
          if (attachments.length > 6) throw Error('每条消息最多 6 个附件');
          for (let j = 0; j < attachments.length; j++) {
            if (!current()) return;
            const key = target.value + ':' + i + ':' + j;
            if (!uploaded.has(key)) {
              status.textContent = `正在准备第 ${i + 1}/${messages.length} 条消息的附件`;
              const timer = setTimeout(() => controller.abort(), 30000);
              let data; try { data = await file(attachments[j], options, controller.signal); } finally { clearTimeout(timer); }
              if (!current()) return;
              const item = attachments[j], params = new URLSearchParams({ file_name: data.name, display_name: data.name, mime_type: data.type,
                kind: item.kind || 'file', conversation_id: `${destination.kind}-${destination.contact.id}` });
              const result = await request(`/api/user/${encodeURIComponent(options.userId)}/chat-attachments?${params}`, { method: 'POST', body: data, headers: { 'Content-Type': data.type } });
              if (!result.attachment?.url) throw Error('附件上传结果无效');
              uploaded.set(key, { ...result.attachment, kind: item.kind, mime_type: data.type,
                ...(item.source_link ? { source_link: item.source_link } : {}), ...(item.duration_seconds ? { duration_seconds: item.duration_seconds } : {}) });
            }
            refs.push(uploaded.get(key));
          }
          if (!current()) return;
          uncertain = true; status.textContent = `正在转发第 ${i + 1}/${messages.length} 条消息`;
          const result = await request(`/api/me/${destination.kind}s/${encodeURIComponent(destination.contact.id)}/messages`, { method: 'POST', body: JSON.stringify({ content: message.content || '', attachments: refs }) });
          if (!result.message?.id) throw Error('发送结果未确认');
          sent++; uncertain = false;
        }
        status.textContent = `已转发 ${sent} 条消息`; uncertain = true; cancel.textContent = '完成'; options.changed?.();
      } catch (error) {
        if (dialog.isConnected) status.textContent = uncertain ? `已确认 ${sent} 条，其余发送结果未确认，请到目标会话核对。本次不会自动重发。` : `${error.message}；已确认 ${sent} 条`;
      } finally { busy = false; send.disabled = uncertain || !current(); target.disabled = sent > 0 || uncertain; cancel.textContent = sent || uncertain ? '关闭' : '取消'; }
    }); send.disabled = true;
    target.onchange = () => { send.disabled = !target.value || busy || uncertain; };
    const guard = setInterval(() => { if (!options.current()) close(); }, 500);
    dialog.oncancel = event => { event.preventDefault(); close(); };
    dialog.append(node('h2', '转发消息'), node('p', `按原顺序逐条转发 ${messages.length} 条消息`), target, status, cancel, send);
    document.body.append(dialog); dialog.showModal();
    (async () => {
      try {
        for (const kind of ['group', 'friend']) {
          const data = await request('/api/me/' + kind + 's'); if (!current()) return;
          (data[kind + 's'] || []).filter(contact => contact.id !== 'usr_elon_ai').forEach(contact => {
            const key = kind + ':' + contact.id; contacts.set(key, { kind, contact });
            target.append(new Option((kind === 'group' ? '群聊 · ' : '好友 · ') + (contact.name || contact.nickname || contact.account || contact.id), key));
          });
        }
        status.textContent = contacts.size ? '' : '没有可转发的会话';
      } catch (error) { if (current()) status.textContent = error.message; }
    })();
    return close;
  }
  root.ElonSocialMessageTransfer = { copy, forward };
})(globalThis);
