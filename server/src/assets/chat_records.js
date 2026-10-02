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
    const linkDisposers = [];
    const clearLinks = () => { linkDisposers.splice(0).forEach(dispose => dispose()); };
    const path = `/api/me/groups/${encodeURIComponent(card.group_id)}/chat-records/${encodeURIComponent(card.record_id)}`;
    const valid = () => dialog.isConnected && options.current();
    const close = () => { lifetime.abort(); pageController?.abort(); clearLinks(); dialog.querySelectorAll('video,audio').forEach(n => n.pause()); urls.forEach(URL.revokeObjectURL); dialog.close(); dialog.remove(); if (closeActive === close) closeActive = null; };
    closeActive = close;
    const check = setInterval(() => { if (!valid()) close(); }, 1500);
    lifetime.signal.addEventListener('abort', () => clearInterval(check), { once: true });
    async function request(suffix = '', binary = false, method = 'GET', signal = lifetime.signal) {
      if (!valid()) throw Error('会话已变化，请重新打开记录');
      const r = await options.api(path + suffix, { method, signal, cache: method === 'GET' ? 'no-cache' : 'no-store' });
      if (!r.ok) throw Error([403, 404].includes(r.status) ? '记录已撤回，或你已不在此群聊中' : `读取失败（${r.status}）`);
      if (+r.headers.get('content-length') > 12 * 1024 * 1024) throw Error('附件过大');
      const value = binary ? await r.blob() : await r.json();
      if (!valid() || signal.aborted) throw Error('读取已取消'); return value;
    }
    const children = id => view.document.messages.filter(r => (r.parent_id || null) === id);
    function back() { if (raw) { raw = false; render(); } else if (parent) move(view.document.messages.find(r => r.id === parent)?.parent_id || null); else close(); }
    function move(id) { offsets.set(parent || '', feed?.scrollTop || 0); parent = id; raw = false; render(); }
    function render() {
      clearLinks();
      pageController?.abort(); pageController = new AbortController(); const signal = pageController.signal; const current = ++generation;
      dialog.querySelectorAll('video,audio').forEach(n => n.pause()); urls.forEach(URL.revokeObjectURL); urls.clear(); dialog.replaceChildren();
      const bar = el('header'); const backButton = button('‹', back); backButton.title = '返回'; backButton.setAttribute('aria-label', '返回');
      const closeButton = button('×', close); closeButton.title = '关闭'; closeButton.setAttribute('aria-label', '关闭');
      bar.append(backButton, el('h2', parent ? '转发的聊天记录' : view?.document.title || card.title), closeButton); dialog.append(bar);
      if (!view) return;
      const nav = el('nav'), menu = el('details', null, 'chat-record-options'), summary = el('summary', '⋮'); summary.setAttribute('aria-label', '更多');
      menu.append(summary, button(raw ? '返回记录' : '原始文本', () => { raw = !raw; render(); }));
      nav.append(el('small', `微信导出 · ${children(parent).length} 条`), menu);
      if (view.owner_id === options.owner) menu.append(button('撤回分享', async () => {
        if (!confirm('撤回后群成员将不能再读取此记录。确定撤回？')) return;
        try { await request('', false, 'DELETE'); view = null; render(); dialog.append(el('p', '聊天记录已撤回')); } catch (e) { dialog.append(el('p', e.message, 'chat-record-error')); }
      })); dialog.append(nav);
      feed = el('div', null, 'chat-record-feed'); dialog.append(feed);
      if (view.document.warnings?.length) { const details = el('details'); details.append(el('summary', `${view.document.warnings.length} 项导入提示`)); view.document.warnings.forEach(s => details.append(el('p', s))); feed.append(details); }
      if (raw) { feed.append(el('pre', view.document.raw_text)); return; }
      children(parent).forEach(row => {
        const article = el('article'), body = el('div', null, 'chat-record-body');
        const identity = ElonRecordPresentation.identity(row.sender), avatar = el('span', identity.initial, 'chat-record-avatar');
        avatar.style.background = identity.color; avatar.style.color = '#fff'; avatar.setAttribute('aria-hidden', 'true'); article.append(avatar, body);
        const meta = el('div', null, 'chat-record-meta'); meta.append(el('span', row.sender), el('time', row.time)); body.append(meta);
        if (row.kind === 'forward') body.append(button(`聊天记录 · ${children(row.id).length} 条`, () => move(row.id)));
        else {
          const text = el('p', null, 'chat-record-text'), previews = new Map((window.ElonSocialLinks?.links(row.text) || []).map(p => [p.url, p]));
          function drawText() {
            const value = ElonRecordPresentation.text(row, [...previews.values()]); text.replaceChildren(); text.hidden = !value;
            value.split(/(https?:\/\/[^\s]+)/g).forEach(part => {
              if (/^https?:\/\//.test(part)) { const a = el('a', part); a.href = part; a.target = '_blank'; a.rel = 'noopener noreferrer'; text.append(a); } else text.append(document.createTextNode(part));
            });
          }
          drawText(); body.append(text);
          if (window.ElonSocialLinks) {
            const cards = el('div'); body.append(cards);
            linkDisposers.push(ElonSocialLinks.mount(cards, row.text, { owner: options.owner, isCurrent: valid, api: async (route, init) => {
              const response = await options.api(route, init);
              if (!response.ok) throw Error('预览暂不可用'); return response.json();
            }, compact: true, open: p => window.ElonSocialLinkViewer?.open(p), openOriginal: p => window.ElonSocialLinkViewer?.open(p),
              onPreview: p => { previews.set(p.url, p); drawText(); },
              actions: (host, get) => ElonRecordActions.bind(host, get, { current: valid, api: async (route, init) => {
                if (!valid()) throw Error('会话已变化');
                const response = await options.api(route, init); if (!response.ok) throw Error('请求失败'); return response.json();
              } }) }));
          }
          if (row.asset_id) {
            const box = el('div', null, 'chat-record-asset'); body.append(box);
            linkDisposers.push(ElonRecordMedia.mount(box, row, { current: () => valid() && current === generation && !signal.aborted,
              scope: `${location.origin}:${options.owner}:${path}:${row.asset_id}`,
              openImage: (blob, name, trigger) => ElonSocialImageViewer.openBlob(blob, { display_name: name }, trigger),
              load: () => request('/assets/' + encodeURIComponent(row.asset_id), true, 'GET', signal) }));
          } else if (row.filename) body.append(el('small', '导出包未提供可用附件 · ' + row.filename));
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
