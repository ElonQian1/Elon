/* Existing group APIs, explicit confirmation, no automatic write retry. */
(() => {
  'use strict';
  const node = (tag, text) => { const n = document.createElement(tag); if (text != null) n.textContent = text; return n; };
  function button(label, fn) { const n = node('button', label); n.type = 'button'; n.onclick = fn; return n; }
  async function copy(value) {
    try { if (navigator.clipboard?.writeText) { await navigator.clipboard.writeText(value); return true; } } catch { /* HTTP / WebView fallback. */ }
    const input = node('textarea', value); input.style.cssText = 'position:fixed;width:1px;height:1px;opacity:0';
    const focus = document.activeElement;
    ([...document.querySelectorAll('dialog[open]')].at(-1) || document.body).append(input); input.select();
    try { return document.execCommand('copy'); } catch { return false; } finally { input.remove(); focus?.focus(); }
  }
  function forward(url, options) {
    const dialog = node('dialog'); dialog.className = 'chat-record-forward'; dialog.setAttribute('aria-label', '转发到群聊');
    const controller = new AbortController(); let busy = false, sent = false;
    async function request(path, init = {}) {
      const request = new AbortController(), abort = () => request.abort();
      controller.signal.addEventListener('abort', abort, { once: true });
      const timer = setTimeout(abort, 15000);
      try { if (controller.signal.aborted || !options.current()) throw Error('会话已变化'); return await options.api(path, { ...init, signal: request.signal }); }
      finally { clearTimeout(timer); controller.signal.removeEventListener('abort', abort); }
    }
    const close = () => { controller.abort(); dialog.close(); dialog.remove(); };
    const status = node('p', '正在读取群聊…'); status.setAttribute('role', 'status');
    const select = node('select'); select.setAttribute('aria-label', '转发目标'); select.append(new Option('请选择群聊', ''));
    const preview = node('p', url); preview.className = 'chat-record-forward-url';
    const send = button('确认转发', async () => {
      if (busy || sent || !select.value || !options.current()) return;
      busy = true; send.disabled = true; select.disabled = true; cancel.disabled = true; status.textContent = '正在发送…';
      try {
        const result = await request(`/api/me/groups/${encodeURIComponent(select.value)}/messages`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ content: url }) });
        if (!result?.message?.id) throw Error('发送结果未确认，请到群聊核对');
        sent = true; status.textContent = '已发送到群聊'; cancel.textContent = '完成';
      } catch { status.textContent = '发送结果未确认，请到群聊核对。本次不会自动重发。'; sent = true; }
      finally { busy = false; cancel.disabled = false; }
    }); send.disabled = true;
    select.onchange = () => { send.disabled = !select.value || busy || sent; };
    const cancel = button('取消', close);
    dialog.append(node('h2', '转发到群聊'), preview, select, status, cancel, send); document.body.append(dialog); dialog.showModal();
    dialog.oncancel = e => { e.preventDefault(); if (!busy) close(); };
    const check = setInterval(() => { if (!options.current()) close(); }, 500);
    controller.signal.addEventListener('abort', () => clearInterval(check), { once: true });
    request('/api/me/groups').then(value => {
      if (!dialog.isConnected || !options.current()) return;
      (value.groups || []).forEach(g => select.append(new Option(g.name, g.id)));
      status.textContent = value.groups?.length ? ' ' : '没有可转发的群聊';
    }).catch(() => { if (dialog.isConnected) status.textContent = '群聊读取失败，请关闭后重试'; });
    return close;
  }
  function bind(host, getPreview, options) {
    const actions = node('div'); actions.className = 'chat-record-link-actions';
    const status = node('span'); status.setAttribute('role', 'status');
    const doCopy = async () => { const ok = await copy(getPreview().url); if (host.isConnected) status.textContent = ok ? '已复制' : '复制失败，请重试'; };
    const menu = node('details'); menu.className = 'chat-record-link-menu'; const summary = node('summary', '更多'); summary.setAttribute('aria-label', '卡片操作');
    let closeForward;
    menu.append(summary, button('转发到群聊', () => { menu.open = false; closeForward?.(); closeForward = forward(getPreview().url, options); }), button('复制链接', () => { menu.open = false; void doCopy(); }));
    const original = host.querySelector('.social-link-original-action'); if (original) actions.append(original);
    actions.append(button('复制链接', doCopy), menu, status); host.append(actions);
    const card = host.querySelector('.social-link-card');
    let timer, origin;
    const cancel = () => clearTimeout(timer);
    const open = e => { e.preventDefault(); menu.open = true; menu.querySelector('button').focus(); };
    const down = e => { if (e.pointerType === 'mouse') return; origin = [e.clientX, e.clientY]; timer = setTimeout(() => { menu.open = true; }, 550); };
    const move = e => { if (origin && Math.hypot(e.clientX - origin[0], e.clientY - origin[1]) > 8) cancel(); };
    const click = e => { if (menu.open) { e.preventDefault(); e.stopImmediatePropagation(); } };
    const key = e => { if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) open(e); };
    menu.onkeydown = e => { if (e.key === 'Escape' && menu.open) { e.preventDefault(); e.stopPropagation(); menu.open = false; card?.focus(); } };
    const events = { contextmenu: open, pointerdown: down, pointermove: move, pointerup: cancel, pointercancel: cancel, keydown: key };
    Object.entries(events).forEach(([event, fn]) => card?.addEventListener(event, fn)); card?.addEventListener('click', click, true);
    return () => { cancel(); closeForward?.(); Object.entries(events).forEach(([event, fn]) => card?.removeEventListener(event, fn)); card?.removeEventListener('click', click, true); actions.remove(); };
  }
  globalThis.ElonRecordActions = { bind };
})();
