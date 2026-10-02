(function (root) {
  'use strict';
  const node = (tag, text, cls) => { const el = document.createElement(tag); if (text != null) el.textContent = text; if (cls) el.className = cls; return el; };
  const allowed = message => message?.id && !message.recalled_at && !message.recalledAt && !message.send_status;
  const signature = message => JSON.stringify([message?.revision || 1, message?.content, message?.attachments, message?.recalled_at, message?.recalledAt]);
  const summary = message => (message.sender_name ? message.sender_name + '：' : '') + (root.ElonGridShare?.summary(message.content) || message.content || (message.attachments || []).map(a => '[' + (a.kind === 'image' ? '图片' : a.kind === 'voice' ? '语音' : '附件') + '] ' + (a.display_name || a.file_name || '')).join(' '));
  function button(label, action, cls) { const el = node('button', label, cls); el.type = 'button'; el.onclick = action; return el; }
  function create(options) {
    let key = '', owner = '', rows = new Map(), ordered = [], quote = null, selecting = false, menu = null, closeTransfer = null, suppress = 0;
    const selected = new Map(), bindings = new Map();
    const bar = node('div', null, 'social-quote-compose'), preview = node('span'), cover = node('img'); cover.alt = ''; cover.hidden = true;
    const cancel = button('×', () => { quote = null; paintQuote(); options.input.focus(); }, 'social-icon-button'); cancel.title = '取消引用'; cancel.setAttribute('aria-label', '取消引用');
    bar.append(preview, cover, cancel); bar.hidden = true; options.input.closest('.input-panel').append(bar);
    const notice = node('p', null, 'social-message-notice'); notice.setAttribute('role', 'status'); notice.hidden = true; options.list.before(notice);
    const selection = node('div', null, 'social-message-selection'), count = node('span'); selection.hidden = true;
    const forward = button('转发', () => transfer(chosen()));
    const copy = button('复制', () => copyMessages(chosen()));
    selection.append(button('取消多选', () => { selecting = false; selected.clear(); paintSelection(); }), count, copy, forward); options.list.before(selection);
    function report(text) { notice.textContent = text; notice.hidden = !text; }
    function valid(message) { return owner === options.owner() && allowed(rows.get(message?.id)) && signature(rows.get(message.id)) === signature(message); }
    function chosen() { return ordered.filter(message => selected.has(message.id) && valid(selected.get(message.id))); }
    function context(messages) {
      const identity = owner, scope = key;
      return { api: options.api, userId: options.userId(), current: () => identity === options.owner() && key === scope && messages.every(valid), changed: options.changed };
    }
    function paintQuote() {
      bar.hidden = !quote; cover.hidden = true; cover.removeAttribute('src'); if (!quote) return;
      preview.textContent = valid(quote) ? summary(quote) : '原消息已修改或不可用，请取消后重新引用';
      const item = valid(quote) && quote.attachments?.find(a => a.kind === 'image' || a.mime_type?.startsWith('image/'));
      if (item?.url) {
        try { const url = new URL(item.url, location.origin); if (['http:', 'https:'].includes(url.protocol)) { cover.src = url.href; cover.hidden = false; } } catch {}
      }
      cover.onerror = () => { cover.hidden = true; };
    }
    function paintSelection() {
      selection.hidden = !selecting; count.textContent = `已选择 ${selected.size} 条`; forward.disabled = copy.disabled = !selected.size;
      bindings.forEach(({ block, check }, id) => { check.hidden = !selecting; check.checked = selected.has(id); block.classList.toggle('social-message-selecting', selecting); block.classList.toggle('social-message-selected', selecting && selected.has(id)); });
    }
    function toggle(message) {
      if (!valid(message)) return;
      if (selected.has(message.id)) selected.delete(message.id);
      else if (selected.size < 20) selected.set(message.id, message);
      else report('每次最多选择 20 条消息');
      paintSelection();
    }
    function dismiss() { if (menu) { const old = menu; menu = null; old.close(); old.remove(); } }
    async function copyMessages(messages) {
      if (!messages.length || !messages.every(valid)) return;
      const env = context(messages); report('');
      try { await root.ElonSocialMessageTransfer.copy(messages, env); if (env.current()) { report('已复制'); dismiss(); } }
      catch (error) { if (env.current()) { report(error.message || '复制失败，请使用转发'); const status = menu?.querySelector('[role="status"]'); if (status) status.textContent = notice.textContent; } }
    }
    function transfer(messages) { if (!messages.length || !messages.every(valid)) return; dismiss(); closeTransfer?.(); closeTransfer = root.ElonSocialMessageTransfer.forward(messages, context(messages)); }
    function show(message, block) {
      if (!valid(message)) return; dismiss();
      const dialog = node('dialog', null, 'social-action-dialog'); menu = dialog; dialog.setAttribute('aria-label', '消息操作');
      const status = node('p'); status.setAttribute('role', 'status');
      dialog.append(node('h2', '消息操作'), node('p', summary(message), 'social-message-excerpt'));
      dialog.append(button('引用', () => { quote = structuredClone(message); paintQuote(); dismiss(); options.input.focus(); }),
        button('复制', () => copyMessages([message])), button('转发', () => transfer([message])),
        button('多选', () => { selecting = true; selected.set(message.id, message); paintSelection(); dismiss(); }));
      block.querySelectorAll('.chat-message-content > .bubble > .group-revision-actions button').forEach(action => {
        const label = action.textContent.startsWith('已编辑') ? '查看修改记录' : action.textContent;
        dialog.append(button(label, () => { dismiss(); action.click(); }));
      });
      block.querySelectorAll('button').forEach(action => {
        if (action.textContent === '识别二维码') dialog.append(button('识别二维码', () => { dismiss(); action.hidden = false; action.click(); }));
      });
      dialog.append(status, button('关闭', dismiss));
      dialog.onclose = () => { dialog.remove(); if (menu === dialog) menu = null; if (block.isConnected && document.activeElement !== options.input && !document.querySelector('dialog[open]')) block.querySelector('.social-message-more')?.focus({ preventScroll: true }); };
      document.body.append(dialog); dialog.showModal();
    }
    function bind(block, message) {
      if (!allowed(message)) return () => {};
      const more = button('⋯', () => show(message, block), 'social-message-more'); more.title = '消息操作'; more.setAttribute('aria-label', '消息操作');
      const check = node('input'); check.type = 'checkbox'; check.className = 'social-message-check'; check.setAttribute('aria-label', '选择消息'); check.onchange = () => toggle(message);
      block.prepend(check); (block.querySelector('.chat-message-content') || block).append(more); bindings.set(message.id, { block, check }); paintSelection();
      block.classList.add('social-message-compact');
      const edited = [...block.querySelectorAll('.chat-message-content > .bubble > .group-revision-actions button')].some(action => action.textContent.startsWith('已编辑'))
        ? node('span', ' · 已编辑', 'social-message-edited') : null;
      if (edited) { edited.title = '可在消息操作中查看修改记录'; block.querySelector('.chat-sender-name')?.append(edited); }
      let timer, start;
      const cancelHold = () => { clearTimeout(timer); start = null; };
      const open = event => { if (event.target.closest('input,textarea') || !valid(message)) return; event.preventDefault(); event.stopPropagation(); cancelHold(); suppress = Date.now() + 700; show(message, block); };
      const events = {
        contextmenu: open,
        keydown: event => { if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) open(event); },
        pointerdown: event => {
          if (selecting || event.pointerType === 'mouse' || event.target.closest('input,textarea,a,audio,video,.social-message-more')) return;
          cancelHold(); start = [event.clientX, event.clientY]; timer = setTimeout(() => { suppress = Date.now() + 800; show(message, block); }, 550);
        },
        pointermove: event => { if (start && Math.hypot(event.clientX - start[0], event.clientY - start[1]) > 8) cancelHold(); },
        pointerup: cancelHold, pointercancel: cancelHold,
        click: event => {
          if (event.target === check || event.target === more) return;
          if (selecting || Date.now() < suppress) { event.preventDefault(); event.stopImmediatePropagation(); if (selecting) toggle(message); }
        },
      };
      Object.entries(events).forEach(([name, fn]) => block.addEventListener(name, fn, true));
      return () => { cancelHold(); Object.entries(events).forEach(([name, fn]) => block.removeEventListener(name, fn, true)); bindings.delete(message.id); more.remove(); check.remove(); edited?.remove(); block.classList.remove('social-message-compact'); };
    }
    function reset() { dismiss(); closeTransfer?.(); closeTransfer = null; quote = null; selecting = false; selected.clear(); key = ''; rows.clear(); ordered = []; paintQuote(); paintSelection(); report(''); }
    function update(messages, kind, contact) {
      const next = kind + ':' + contact.id;
      if (key !== next || owner !== options.owner()) reset();
      key = next; owner = options.owner(); ordered = messages; rows = new Map(messages.map(m => [m.id, m]));
      for (const [id, message] of selected) if (!valid(message)) selected.delete(id);
      if (menu && menu.open) dismiss(); paintQuote(); paintSelection();
    }
    function sender(kind, contact) {
      const source = quote, scope = key, identity = owner;
      return async (sendKind, sendContact, content, attachments = []) => {
        if (scope !== key || identity !== options.owner() || kind !== sendKind || contact.id !== sendContact.id) throw Error('会话已变化，请重新发送');
        if (source && !valid(source)) throw Error('原消息已修改或不可用，请取消后重新引用');
        await options.send(kind, contact, content, attachments, source ? { message_id: source.id, revision: source.revision || 1 } : null, source ? { ...source, message_id: source.id } : null);
        if (scope === key && identity === options.owner() && source === quote) { quote = null; paintQuote(); }
      };
    }
    return { update, bind, reset, sender, send(kind, contact, text) {
      const send = sender(kind, contact), scope = key, identity = owner;
      return send(kind, contact, text).catch(error => {
        if (key === scope && identity === options.owner() && !options.input.value) { options.input.value = text; options.input.dispatchEvent(new Event('input', { bubbles: true })); }
        throw error;
      });
    } };
  }
  root.ElonSocialMessageActions = { create };
})(globalThis);
