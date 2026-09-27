/* Imported records: private group-scoped reads, text nodes only, no public media URLs. */
(() => {
  'use strict';
  const PREFIX = '【一龙聊天记录】\n';
  const el = (tag, text, cls) => { const n = document.createElement(tag); if (text != null) n.textContent = text; if (cls) n.className = cls; return n; };
  const button = (text, fn) => { const n = el('button', text); n.type = 'button'; n.onclick = fn; return n; };
  const reference = text => {
    try { if (!text.startsWith(PREFIX)) return null; const c = JSON.parse(text.slice(PREFIX.length));
      return c.schema === 'chat_record_bundle_v1' && typeof c.record_id === 'string' && typeof c.group_id === 'string' && /^[\w-]{1,128}$/.test(c.record_id) && /^[\w-]{1,128}$/.test(c.group_id) && typeof c.title === 'string' && typeof c.summary === 'string' && Number.isSafeInteger(c.message_count) && c.message_count > 0 && c.message_count <= 2000 ? c : null;
    } catch { return null; }
  };
  let closeActive = null;
  function open(card, options) {
    closeActive?.();
    const dialog = el('dialog', null, 'chat-record-reader'); document.body.append(dialog);
    dialog.setAttribute('aria-label', '聊天记录');
    const lifetime = new AbortController(), urls = new Set(), offsets = new Map();
    let view, parent = null, feed, raw = false, generation = 0, pageController = null;
    const path = `/api/me/groups/${encodeURIComponent(card.group_id)}/chat-records/${encodeURIComponent(card.record_id)}`;
    const valid = () => dialog.isConnected && options.current();
    const close = () => { lifetime.abort(); pageController?.abort(); dialog.querySelectorAll('video,audio').forEach(n => n.pause()); urls.forEach(URL.revokeObjectURL); dialog.close(); dialog.remove(); if (closeActive === close) closeActive = null; };
    closeActive = close;
    const check = setInterval(() => { if (!valid()) close(); }, 1500);
    lifetime.signal.addEventListener('abort', () => clearInterval(check), { once: true });
    async function request(suffix = '', binary = false, method = 'GET', signal = lifetime.signal) {
      if (!valid()) throw Error('会话已变化，请重新打开记录');
      const r = await options.api(path + suffix, { method, signal, cache: 'no-store' });
      if (!r.ok) throw Error([403, 404].includes(r.status) ? '记录已撤回，或你已不在此群聊中' : `读取失败（${r.status}）`);
      if (+r.headers.get('content-length') > 12 * 1024 * 1024) throw Error('附件过大');
      const value = binary ? await r.blob() : await r.json();
      if (!valid() || signal.aborted) throw Error('读取已取消'); return value;
    }
    const children = id => view.document.messages.filter(r => (r.parent_id || null) === id);
    function back() { if (raw) { raw = false; render(); } else if (parent) move(view.document.messages.find(r => r.id === parent)?.parent_id || null); else close(); }
    function move(id) { offsets.set(parent || '', feed?.scrollTop || 0); parent = id; raw = false; render(); }
    function render() {
      pageController?.abort(); pageController = new AbortController(); const signal = pageController.signal; const current = ++generation;
      dialog.querySelectorAll('video,audio').forEach(n => n.pause()); urls.forEach(URL.revokeObjectURL); urls.clear(); dialog.replaceChildren();
      const bar = el('header'); const backButton = button('‹', back); backButton.title = '返回'; backButton.setAttribute('aria-label', '返回');
      const closeButton = button('×', close); closeButton.title = '关闭'; closeButton.setAttribute('aria-label', '关闭');
      bar.append(backButton, el('h2', parent ? '转发的聊天记录' : view?.document.title || card.title), closeButton); dialog.append(bar);
      if (!view) return;
      const nav = el('nav'); nav.append(el('small', `微信导出 · ${children(parent).length} 条`), button(raw ? '返回记录' : '原始文本', () => { raw = !raw; render(); }));
      if (view.owner_id === options.owner) nav.append(button('撤回分享', async () => {
        if (!confirm('撤回后群成员将不能再读取此记录。确定撤回？')) return;
        try { await request('', false, 'DELETE'); view = null; render(); dialog.append(el('p', '聊天记录已撤回')); } catch (e) { dialog.append(el('p', e.message, 'chat-record-error')); }
      })); dialog.append(nav);
      feed = el('div', null, 'chat-record-feed'); dialog.append(feed);
      if (view.document.warnings?.length) { const details = el('details'); details.append(el('summary', `${view.document.warnings.length} 项导入提示`)); view.document.warnings.forEach(s => details.append(el('p', s))); feed.append(details); }
      if (raw) { feed.append(el('pre', view.document.raw_text)); return; }
      children(parent).forEach(row => {
        const article = el('article'), body = el('div', null, 'chat-record-body');
        article.append(el('span', Array.from(row.sender)[0] || '?', 'chat-record-avatar'), body);
        const meta = el('div', null, 'chat-record-meta'); meta.append(el('span', row.sender), el('time', row.time)); body.append(meta);
        if (row.kind === 'forward') body.append(button(`聊天记录 · ${children(row.id).length} 条`, () => move(row.id)));
        else {
          const text = el('p', null, 'chat-record-text'); row.text.split(/(https?:\/\/[^\s]+)/g).forEach(part => {
            if (/^https?:\/\//.test(part)) { const a = el('a', part); a.href = part; a.target = '_blank'; a.rel = 'noopener noreferrer'; text.append(a); } else text.append(document.createTextNode(part));
          }); body.append(text);
          if (row.asset_id) {
            const box = el('div', null, 'chat-record-asset'); body.append(box);
            const load = async () => {
              box.replaceChildren(el('small', '正在读取附件…'));
              try {
                const blob = await request('/assets/' + encodeURIComponent(row.asset_id), true, 'GET', signal);
                if (current !== generation) return;
                const url = URL.createObjectURL(blob); urls.add(url); box.replaceChildren();
                if (row.kind === 'image' && blob.type.startsWith('image/')) { const img = el('img'); img.src = url; img.alt = row.filename; box.append(img); }
                else if (row.kind === 'video' && blob.type.startsWith('video/')) { const video = el('video'); video.src = url; video.controls = true; video.preload = 'metadata'; box.append(video); }
                else { const a = el('a', '下载：' + row.filename); a.href = url; a.download = row.filename || '附件'; box.append(a); }
              } catch (e) { if (current === generation && !signal.aborted) box.replaceChildren(el('small', e.message), button('重试', load)); }
            };
            box.append(button(row.kind === 'image' ? '查看图片' : row.kind === 'video' ? '播放视频' : '读取附件', load));
            if (row.kind === 'image') { const observer = new IntersectionObserver(entries => { if (entries.some(e => e.isIntersecting)) { observer.disconnect(); void load(); } }); observer.observe(box); signal.addEventListener('abort', () => observer.disconnect(), { once: true }); }
          } else if (row.filename) body.append(el('small', '导出包未提供可用附件'));
        }
        feed.append(article);
      }); feed.scrollTop = offsets.get(parent || '') || 0;
    }
    async function load() {
      render(); dialog.append(el('p', '正在读取…'));
      try { view = await request(); render(); }
      catch (e) { if (valid()) { render(); dialog.append(el('p', e.message, 'chat-record-error'), button('重试', load)); } }
    }
    dialog.addEventListener('cancel', e => { e.preventDefault(); back(); }); dialog.showModal(); void load();
  }
  function mount(bubble, text, options) {
    const card = reference(text); if (!card || card.group_id !== options.group) return false;
    bubble.replaceChildren(); const n = button('', () => open(card, options)); n.className = 'chat-record-card';
    n.append(el('strong', card.title), el('span', card.summary), el('small', `聊天记录 · ${card.message_count} 条`)); bubble.append(n); return true;
  }
  window.ElonChatRecords = { reference, mount, open, close: () => closeActive?.() };
})();
