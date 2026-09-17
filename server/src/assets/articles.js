/* Group articles: scoped mobile UI; documents never contain executable markup. */
(() => {
  'use strict';
  const PREFIX = '【一龙文章】\n';
  let cacheUser; const previews = new Map();
  const el = (tag, text, cls) => { const n = document.createElement(tag); if (text != null) n.textContent = text; if (cls) n.className = cls; return n; };
  const button = (text, action) => { const n = el('button', text); n.type = 'button'; n.onclick = action; return n; };
  const reference = text => { try { if (!text.startsWith(PREFIX)) return null; const r = JSON.parse(text.slice(PREFIX.length)); return r.schema === 1 && /^article_[\w-]+$/.test(r.article_id) && Number.isSafeInteger(r.revision) && r.revision > 0 && typeof r.title === 'string' && typeof r.summary === 'string' ? r : null; } catch { return null; } };
  const image = (src, cover = false) => { const n = el('img', null, cover ? 'article-hero' : ''); if (/^data:image\/(png|jpeg|webp);base64,/.test(src || '')) n.src = src; n.alt = cover ? '文章封面' : '正文图片'; return n; };
  function card(a, action) {
    const n = button('', action); n.className = 'article-card';
    if (a.cover_data_url) n.append(image(a.cover_data_url, true));
    const info = el('span', null, 'article-card-info'); info.append(el('small', `文章${a.author_name ? ' · ' + a.author_name : ''}`), el('strong', a.title || '未命名文章'), el('span', a.summary || ''), el('small', a.status === 'withdrawn' ? '已撤下' : '阅读全文 ›')); n.append(info); return n;
  }
  function body(a) {
    const n = el('article', null, 'article-reading'), d = a.document, media = a.media || {};
    if (d.cover && media[d.cover]) n.append(image(media[d.cover], true));
    const prose = el('div', null, 'article-prose'); prose.append(el('h1', d.title || '未命名文章'), el('small', `${a.author_name} · ${(a.updated_at || '').slice(0, 10)}`));
    if (d.summary) prose.append(el('p', d.summary, 'article-abstract'));
    d.blocks.forEach(b => { if (b.type === 'image') { const figure = el('figure'); figure.append(image(media[b.media_id]), el('figcaption', b.caption)); prose.append(figure); } else prose.append(el(b.type === 'heading' ? 'h2' : b.type === 'quote' ? 'blockquote' : 'p', b.text)); }); n.append(prose); return n;
  }
  async function upload(file, request) {
    if (!/^image\/(png|jpeg|webp)$/.test(file.type) || file.size > 20 * 1024 * 1024) throw Error('请选择小于20MB的PNG、JPEG或WebP图片');
    const url = URL.createObjectURL(file);
    try {
      const img = new Image(); img.src = url; await img.decode(); const scale = Math.min(1, 1600 / Math.max(img.width, img.height));
      const canvas = document.createElement('canvas'); canvas.width = Math.round(img.width * scale); canvas.height = Math.round(img.height * scale); const ctx = canvas.getContext('2d');
      ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, canvas.width, canvas.height); ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      let data; for (const q of [.85, .7, .55, .4]) { data = canvas.toDataURL('image/jpeg', q); if (data.length < 699000) break; }
      if (data.length >= 699000) throw Error('图片仍然过大，请裁剪后重试'); return await request('/api/me/articles/media', 'POST', { base64: data.split(',')[1] });
    } finally { URL.revokeObjectURL(url); }
  }
  function open(api, groupId, ref) {
    const dialog = el('dialog', null, 'article-dialog'), panel = el('section', null, 'article-panel'); dialog.append(panel); document.body.append(dialog); dialog.showModal();
    let current = null, dirty = false, busy = false, mine = false, mode = 'library';
    const request = async (path, method = 'GET', value) => { const r = await api(path, { method, ...(value === undefined ? {} : { body: JSON.stringify(value) }) }); const data = await r.json(); if (!r.ok) throw Error(data.error || `请求失败（${r.status}）`); return data; };
    const status = el('p', '', 'article-status'); status.setAttribute('role', 'status');
    const run = async fn => { if (busy) return; busy = true; status.textContent = '正在处理…'; const controls = [...dialog.querySelectorAll('button,input,textarea,select')]; controls.forEach(n => { n.dataset.wasDisabled = String(n.disabled); n.disabled = true; });
      try { await fn(); if (status.textContent === '正在处理…') status.textContent = ''; } catch (e) { status.textContent = (e.message || '操作失败，请重试') + (dirty ? '。编辑内容仍保留。' : ''); } finally { busy = false; controls.forEach(n => { n.disabled = n.dataset.wasDisabled === 'true'; }); }
    };
    const close = () => { if (busy) return; if (dirty && !confirm('还有未保存的修改，确定放弃？')) return; dialog.close(); dialog.remove(); };
    dialog.addEventListener('cancel', e => { e.preventDefault(); close(); });
    const header = (title, back = close) => { panel.replaceChildren(); const bar = el('header'); bar.append(button('‹ 返回', back), el('strong', title)); panel.append(bar, status); return bar; };
    const save = async () => { if (dirty) { current = await request(`/api/me/articles/${current.id}/draft`, 'PUT', { version: current.revision, document: current.document }); dirty = false; } return current; };
    const leaveEditor = () => { if (busy) return; if (dirty && !confirm('还有未保存的修改，确定放弃并返回列表？')) return; dirty = false; current = null; void library(); };
    async function library() {
      mode = 'library'; const bar = header('文章'); bar.append(button('写文章', () => void run(async () => { current = await request('/api/me/articles', 'POST', {}); dirty = false; editor(); })));
      const nav = el('nav'); nav.append(button(mine ? '群文章' : '✓ 群文章', () => { if (!busy) { mine = false; void library(); } }), button(mine ? '✓ 我的文章' : '我的文章', () => { if (!busy) { mine = true; void library(); } }),button('币安广场',()=>window.ElonSquare.open(api))); panel.append(nav);
      const list = el('div', null, 'article-library'); panel.append(list);
      const load = async offset => { const p = await request(`/api/me/articles?offset=${offset}${mine ? '' : '&group_id=' + encodeURIComponent(groupId)}`);
        if (!p.items.length && offset === 0) list.append(el('p', mine ? '还没有文章，开始写第一篇吧。' : '群里还没有文章，点击“写文章”开始。'));
        p.items.forEach(a => list.append(card(a, () => { if (busy) return; if (a.status === 'withdrawn') { status.textContent = '文章已撤下'; return; } void run(async () => { if (mine) { current = await request(`/api/me/articles/${a.id}/draft`); dirty = false; editor(); } else await read(a.id, a.revision); }); })));
        if (p.next_offset !== null) { const more = button('加载更多', () => void run(async () => { await load(p.next_offset); more.remove(); })); list.append(more); }
      };
      await run(() => load(0)); if (!list.childElementCount) list.append(button('重试', () => void library()));
    }
    async function read(id, revision) {
      mode = 'read'; header('文章', groupId ? () => void library() : close);
      const retry = button('重新读取', () => void run(() => read(id, revision))); panel.append(retry);
      const a = await request(`/api/me/articles/${id}/revisions/${revision}`); retry.remove(); panel.append(body(a));
    }
    function preview() { mode = 'preview'; const bar = header('预览文章', editor); bar.append(button('发布到群', () => void run(chooseGroups)), button('公开链接', () => void run(shareLink)),button('币安广场',()=>void run(async()=>window.ElonSquare.open(api,await save())))); panel.append(body(current)); }
    // Public page: opt-in per revision; anyone with the link can read, until the author revokes it.
    async function shareLink() {
      const a = await save(); const share = await request(`/api/me/articles/${a.id}/share`, 'POST', { version: a.revision }); current.status = 'published';
      const section = el('section', null, 'article-publish'); section.append(el('h3', '公开链接'), el('p', '任何人打开都能阅读，分享到微信、X 等平台会显示标题和封面。修改后需重新生成。'));
      const link = el('a', share.url); link.href = share.url; link.target = '_blank'; link.rel = 'noopener noreferrer'; link.style.overflowWrap = 'anywhere'; section.append(link);
      section.append(button('复制链接', () => { navigator.clipboard?.writeText(share.url).then(() => { status.textContent = '链接已复制'; }, () => { status.textContent = '复制失败，请长按链接复制'; }); }),
        button('关闭公开链接', () => void run(async () => { await request(`/api/me/articles/${a.id}/share`, 'DELETE'); section.remove(); status.textContent = '公开链接已关闭'; })), button('收起', () => section.remove()));
      panel.insertBefore(section, panel.children[2] || null);
    }
    async function chooseGroups() {
      const data = await request('/api/me/groups'); const section = el('section', null, 'article-publish'), form = el('div'); section.append(el('h3', '选择发布群聊'), el('p', '仅作者和所选群的当前成员可读。相同版本不会重复发送。'), form);
      const checks = (data.groups || []).map(g => { const label = el('label'), check = el('input'); check.type = 'checkbox'; check.value = g.id; check.checked = g.id === groupId; label.append(check, document.createTextNode(g.name)); form.append(label); return check; });
      section.append(button('取消', () => section.remove()), button('确认发布', () => void run(async () => { const ids = checks.filter(c => c.checked).map(c => c.value); if (!ids.length || ids.length > 10) throw Error('请选择1至10个群聊'); const a = await save(); const result = await request(`/api/me/articles/${a.id}/publish`, 'POST', { version: a.revision, group_ids: ids }); current.status = 'published'; section.remove(); status.textContent = result.messages.length ? '文章已发布到群聊' : '这些群已收到此版本，没有重复发送'; })));
      panel.insertBefore(section, panel.children[2] || null);
    }
    function editor() {
      mode = 'editor'; const bar = header('编辑文章', leaveEditor); bar.append(button('保存草稿', () => void run(async () => { await save(); status.textContent = '草稿已保存'; })), button('预览', preview));
      status.textContent = dirty ? '尚未保存' : '草稿已保存'; const fields = el('div', null, 'article-fields'), d = current.document; panel.append(fields);
      const changed = () => { dirty = true; status.textContent = '尚未保存，请保存草稿'; };
      const field = (label, value, max, setter, area = false) => { const wrap = el('label', label), input = el(area ? 'textarea' : 'input'); input.value = value; input.maxLength = max; if (area) input.rows = 5; input.oninput = () => { setter(input.value); changed(); }; wrap.append(input); return wrap; };
      const picker = cover => { const wrap = el('label', cover ? '选择封面' : '＋ 插入图片'), input = el('input'); input.type = 'file'; input.accept = 'image/png,image/jpeg,image/webp'; input.onchange = () => { const file = input.files[0]; input.value = ''; if (file) void run(async () => { const m = await upload(file, request); current.media[m.id] = m.data_url; if (cover) d.cover = m.id; else { if (d.blocks.length >= 120) throw Error('正文最多120个段落'); d.blocks.push({ type: 'image', media_id: m.id, caption: '' }); } changed(); editor(); }); }; wrap.append(input); return wrap; };
      fields.append(field('标题', d.title, 120, v => { d.title = v; }), field('摘要（选填）', d.summary, 300, v => { d.summary = v; }, true), picker(true));
      if (d.cover && current.media[d.cover]) fields.append(image(current.media[d.cover], true), button('移除封面', () => { d.cover = null; changed(); editor(); }));
      fields.append(el('h3', '图文正文'));
      d.blocks.forEach((b, i) => { const section = el('section', null, 'article-block'), controls = el('div'); controls.append(el('small', `第${i + 1}段`), button('删除', () => { d.blocks.splice(i, 1); changed(); editor(); })); if (i > 0) controls.append(button('上移', () => { [d.blocks[i - 1], d.blocks[i]] = [b, d.blocks[i - 1]]; changed(); editor(); })); section.append(controls);
        if (b.type === 'image') section.append(image(current.media[b.media_id]), field('图片说明', b.caption, 300, v => { b.caption = v; }));
        else { const select = el('select'); [['paragraph', '正文'], ['heading', '小标题'], ['quote', '引用']].forEach(([v, name]) => { const option = el('option', name); option.value = v; select.append(option); }); select.value = b.type; select.onchange = () => { b.type = select.value; changed(); }; section.append(select, field('内容', b.text, 50000, v => { b.text = v; }, true)); }
        fields.append(section);
      });
      fields.append(button('＋ 添加段落', () => { if (d.blocks.length >= 120) { status.textContent = '正文最多120个段落'; return; } d.blocks.push({ type: 'paragraph', text: '' }); changed(); editor(); }), picker(false), el('p', '修改后需要保存；再次发布会发送新版本，旧卡片保留原文。'), button('撤下文章', () => { if (confirm('撤下后，所有群中的文章将无法打开，也不能再次发布。确定撤下？')) void run(async () => { const a = await save(); await request(`/api/me/articles/${a.id}/withdraw`, 'POST', { version: a.revision }); dirty = false; current = null; busy = false; await library(); }); }));
    }
    const warn = e => { if (dirty) { e.preventDefault(); e.returnValue = ''; } };
    window.addEventListener('beforeunload', warn); dialog.addEventListener('close', () => window.removeEventListener('beforeunload', warn), { once: true });
    if (ref) void run(() => read(ref.article_id, ref.revision)); else void library();
  }
  window.ElonArticles = {
    reference,
    install(api, getGroup, strip) {
      if (!strip) return; const entry = button('文章', () => { const g = getGroup(); if (g) open(api, g.id); }); entry.className = 'article-entry'; strip.after(entry);
      const update = () => { entry.hidden = strip.classList.contains('hidden') || !getGroup(); }; new MutationObserver(update).observe(strip, { attributes: true, attributeFilter: ['class'] }); update();
    },
    mount(bubble, text, api, userId) {
      const ref = reference(text); if (!ref) return; const click = () => open(api, null, ref); bubble.replaceChildren(card(ref, click));
      if (cacheUser !== userId) { previews.clear(); cacheUser = userId; }
      const key = `${ref.article_id}:${ref.revision}`, prior = previews.get(key);
      let pending = prior && Date.now() - prior.time < 60000 ? prior.promise : null;
      if (!pending) { pending = api(`/api/me/articles/${ref.article_id}/revisions/${ref.revision}?compact=true`).then(r => r.ok ? r.json() : null).catch(() => null); if (previews.size >= 160) previews.delete(previews.keys().next().value); previews.set(key, { time: Date.now(), promise: pending }); }
      pending.then(a => { if (a && cacheUser === userId && bubble.isConnected) bubble.replaceChildren(card(a, click)); });
    },
  };
})();
