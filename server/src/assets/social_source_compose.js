(function (root) {
  'use strict';
  let active = false;
  function button(text, action) { const b = document.createElement('button'); b.type = 'button'; b.textContent = text; b.style.cssText = 'min-height:48px;padding:8px 18px;border:1px solid #252b33;border-radius:8px;background:#171c22;color:#c2cbd6;margin-right:8px'; b.onclick = action; return b; }
  function open(options) {
    if (active) return;
    const input = document.createElement('input'); input.type = 'file'; input.multiple = true; input.accept = 'image/*,video/*,application/pdf';
    input.onchange = () => { const files = Array.from(input.files || []); if (files.length) preview(options, files); };
    input.click();
  }
  async function preview(options, files) {
    if (active) return; active = true;
    const dialog = document.createElement('dialog'); dialog.setAttribute('aria-label', '分享预览');
    dialog.style.cssText = 'box-sizing:border-box;background:#0e1116;color:#d0d0d0;border:1px solid #252b33;border-radius:16px;width:min(94vw,560px);max-height:86dvh;padding:18px;padding-bottom:max(18px,env(safe-area-inset-bottom));overflow:auto';
    const heading = document.createElement('h3'); heading.textContent = '发送给 ' + (options.contact.name || options.contact.nickname || options.contact.account || '当前会话');
    const notice = document.createElement('p'); notice.setAttribute('role', 'status');
    const text = document.createElement('textarea'); text.placeholder = '添加文字或文章链接'; text.maxLength = 4000; text.style.cssText = 'width:100%;box-sizing:border-box;min-height:88px;padding:12px;background:#171c22;color:#d0d0d0;border:1px solid #252b33;border-radius:8px;font:inherit';
    const owner = options.owner(), items = [], urls = []; let busy = true, uncertain = false;
    function close() { if (!busy) dialog.close(); }
    const cancel = button('取消', close), send = button('发送', submit); send.disabled = true;
    send.style.background = 'linear-gradient(#c2cbd6,#95a6b9,#71879f)'; send.style.color = '#0b1118';
    dialog.addEventListener('cancel', event => { if (busy) event.preventDefault(); });
    dialog.addEventListener('close', () => { urls.forEach(url => URL.revokeObjectURL(url)); dialog.remove(); active = false; });
    dialog.append(heading, text, notice, cancel, send); document.body.append(dialog); dialog.showModal();
    try {
      if (files.length > 6 || files.some(f => f.size > 8 * 1024 * 1024 || !f.size)) throw new Error('每次最多 6 个附件，单个附件不超过 8 MB 且不能为空');
      for (const file of files) {
        const item = { file, source: null }; items.push(item);
        const name = document.createElement('p'); name.textContent = file.name; dialog.insertBefore(name, notice);
        if (file.type.startsWith('image/')) {
          const img = document.createElement('img'); img.alt = '待发送：' + file.name; img.src = URL.createObjectURL(file); urls.push(img.src); img.style.cssText = 'display:block;max-width:100%;max-height:180px;border-radius:8px'; dialog.insertBefore(img, notice);
          notice.textContent = '正在本地识别图片链接…';
          const links = await root.ElonSourceLinks.scan(file).catch(() => []);
          if (links.length) {
            const select = document.createElement('select'); select.setAttribute('aria-label', '选择图片原文链接'); select.style.cssText = 'width:100%;min-height:48px;margin-top:10px;padding:8px;background:#171c22;color:#d0d0d0;border:1px solid #252b33;border-radius:8px';
            select.add(new Option('不附加链接', '')); links.forEach(link => select.add(new Option(link.url, link.url)));
            if (links.length === 1) { item.source = links[0]; select.value = links[0].url; }
            select.onchange = () => { item.source = links.find(link => link.url === select.value) || null; }; dialog.insertBefore(select, notice);
          }
        }
      }
      notice.textContent = '请核对图片与链接后发送'; send.disabled = false;
    } catch (error) { notice.textContent = error.message; } finally { busy = false; }
    async function submit() {
      if (busy || uncertain || send.disabled) return;
      busy = true; send.disabled = true; cancel.disabled = true;
      const refs = [];
      try {
        for (const item of items) {
          if (owner !== options.owner()) throw new Error('账号已变化，请重新选择附件');
          const f = item.file, params = new URLSearchParams({ file_name: f.name, display_name: f.name, mime_type: f.type || 'application/octet-stream', kind: f.type.startsWith('image/') ? 'image' : 'file', conversation_id: `${options.kind}-${options.contact.id}` });
          notice.textContent = '正在上传附件…';
          const response = await options.api(`/api/user/${encodeURIComponent(options.userId)}/chat-attachments?${params}`, { method: 'POST', body: f, signal: AbortSignal.timeout(60000) });
          if (!response.ok) throw new Error('附件上传失败，请重试');
          const ref = (await response.json()).attachment; if (!ref?.url) throw new Error('附件上传响应异常');
          if (item.source) ref.source_link = item.source; refs.push(ref);
        }
        if (owner !== options.owner()) throw new Error('账号已变化，请重新选择附件');
        uncertain = true; notice.textContent = '正在发送…';
        await options.send(options.kind, options.contact, text.value.trim(), refs); busy = false; dialog.close();
      } catch (error) {
        notice.textContent = uncertain ? '发送结果未确认，请到聊天核对，避免重复发送。' : error.message;
        send.disabled = uncertain;
      } finally { busy = false; cancel.disabled = false; }
    }
  }
  root.ElonSourceCompose = { open };
})(globalThis);
