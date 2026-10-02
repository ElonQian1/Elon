(function (root) {
  'use strict';
  const element = (tag, text) => { const node = document.createElement(tag); if (text != null) node.textContent = text; return node; };
  const button = (label, action) => { const node = element('button', label); node.type = 'button'; node.onclick = action; return node; };
  function mount(options) {
    const { list } = options;
    let alive = true, dialog = null, history = false, jumping = false, intent = 0, timer, deleted, generation = 0, edgeBusy = false, edgeIntent = false;
    let markerDestination = '', markerFrame = 0;
    const observedRows = new Set(), resize = new ResizeObserver(scheduleMarkers);
    const mutations = new MutationObserver(records => {
      if (records.some(record => !record.target.closest?.('.reading-message-marker') &&
        [...record.addedNodes, ...record.removedNodes].some(n => !n.classList?.contains('reading-message-marker')))) scheduleMarkers();
    });
    mutations.observe(list, { childList: true, subtree: true }); resize.observe(list);
    const bar = element('div'), label = element('span'); bar.className = 'reading-bookmarks-bar'; label.setAttribute('role', 'status');
    const browse = button('书签', show), addCurrent = button('标记当前位置', () => add(currentMessage()));
    const setCurrent = button('更新续读位置', () => { intent = Date.now() + 1600; capture(); void model.flush(); });
    bar.append(browse, addCurrent, setCurrent, label); (options.toolbar || list.parentNode).insertBefore(bar, options.toolbar ? null : list);
    const model = root.ElonReadingPositions.create({ owner: options.owner, scope: options.scope, request: options.request,
      current: () => alive && (!options.current || options.current()), onChange: render });
    function rows() { return Array.from(list.querySelectorAll('[data-message-id]')); }
    function scheduleMarkers() {
      if (!alive || markerFrame) return;
      markerFrame = requestAnimationFrame(() => { markerFrame = 0; renderMarkers(); });
    }
    function renderMarkers() {
      if (!alive) return;
      const state = model.snapshot(), labels = new Map(), currentRows = rows();
      function mark(id, label) { if (id) labels.set(id, [...(labels.get(id) || []), label]); }
      if (state.supported && !state.unavailable && (!options.current || options.current())) {
        state.bookmarks.forEach(b => mark(b.anchor?.message_id, `书签：${b.title}`));
        const active = state.bookmarks.find(b => b.id === state.active);
        if (active && markerDestination !== active.anchor?.message_id) mark(markerDestination, `续读位置：${active.title}`);
      }
      for (const row of observedRows) if (!currentRows.includes(row)) { resize.unobserve(row); observedRows.delete(row); }
      for (const row of currentRows) {
        if (!observedRows.has(row)) { observedRows.add(row); resize.observe(row); }
        const description = labels.get(row.dataset.messageId)?.join('；');
        let marker = row.querySelector(':scope > .reading-message-marker');
        if (!description) { marker?.remove(); row.classList.remove('reading-marked-message'); continue; }
        if (!marker) { marker = element('span', '🔖'); marker.className = 'reading-message-marker'; marker.setAttribute('role', 'img'); row.append(marker); }
        row.classList.add('reading-marked-message'); marker.setAttribute('aria-label', description); marker.title = description;
        const bubble = row.querySelector('[data-reading-bubble]:not([hidden]),.bubble') || row.querySelector('[data-social-content]') || row;
        const own = row.dataset.readingOwn === 'true' || !!row.querySelector('.bubble-row.user');
        const rect = row.getBoundingClientRect(), anchor = bubble.getBoundingClientRect(), viewport = list.getBoundingClientRect();
        const width = marker.offsetWidth, height = marker.offsetHeight;
        const x = own ? anchor.left - rect.left - width - 4 : anchor.right - rect.left + 4;
        const y = Math.min(Math.max(anchor.top - rect.top, viewport.top - rect.top + 4), Math.max(anchor.top - rect.top, anchor.bottom - rect.top - height));
        marker.style.left = `${Math.max(0, Math.min(rect.width - width, x))}px`; marker.style.top = `${Math.max(0, y)}px`;
      }
    }
    function visibleRow() { const top = list.getBoundingClientRect().top; return rows().find(n => n.getBoundingClientRect().bottom > top + 1); }
    function currentMessage() { const id = visibleRow()?.dataset.messageId; return options.messages().find(m => m.id === id); }
    function capture() {
      if (!alive || jumping || Date.now() > intent || options.filtered?.()) return;
      const node = visibleRow(), message = currentMessage(); if (!node || !message) return;
      const rect = node.getBoundingClientRect(), fraction = Math.max(0, Math.min(.99, (list.getBoundingClientRect().top - rect.top) / Math.max(1, rect.height)));
      markerDestination = message.id; model.putPosition({ message_id: message.id, created_at: message.created_at, fraction });
    }
    async function newerAtEdge() {
      if (!edgeIntent || edgeBusy || !history || jumping || !options.newer || list.scrollHeight - list.clientHeight - list.scrollTop > 96) return;
      edgeIntent = false; edgeBusy = true;
      try { await options.newer(); } finally { edgeBusy = false; }
    }
    function userInput(event) { intent = Date.now() + 1600; if (event.type !== 'wheel' || event.deltaY > 0) edgeIntent = true; requestAnimationFrame(() => void newerAtEdge()); }
    function scrolled() { scheduleMarkers(); if (Date.now() > intent) return; clearTimeout(timer); timer = setTimeout(capture, 600); void newerAtEdge(); }
    const events = ['wheel', 'touchmove', 'keydown', 'pointerdown'];
    events.forEach(name => list.addEventListener(name, userInput, { passive: true })); list.addEventListener('scroll', scrolled, { passive: true });
    function render() {
      if (!alive) return;
      const state = model.snapshot(), active = state.bookmarks.find(b => b.id === state.active);
      if (state.unavailable) closeDialog();
      browse.hidden = addCurrent.hidden = !state.supported || state.unavailable;
      setCurrent.hidden = browse.hidden || !history;
      label.textContent = state.error || (active ? `正在续读：${active.title}` : history ? '正在阅读历史消息' : '') || (state.pending ? '阅读位置已存本机，待同步' : '');
      options.changed?.(state);
      scheduleMarkers();
    }
    function closeDialog() { if (dialog) { const old = dialog; dialog = null; old.close(); old.remove(); } }
    function openDialog(title) {
      closeDialog(); const next = element('dialog'); next.className = 'reading-bookmarks-dialog'; next.setAttribute('aria-label', title);
      next.append(element('h2', title)); next.addEventListener('close', () => { next.remove(); if (dialog === next) dialog = null; });
      document.body.append(next); next.showModal(); dialog = next; return next;
    }
    function editor(item, message) {
      const next = openDialog(item ? '编辑书签' : '添加书签'), title = element('input'), note = element('textarea'), error = element('p');
      title.value = item?.title || ''; title.maxLength = 80; title.placeholder = '例如：周末接着看'; title.setAttribute('aria-label', '书签名称');
      note.value = item?.note || ''; note.maxLength = 1000; note.placeholder = '备注（可选）'; note.setAttribute('aria-label', '备注'); error.setAttribute('role', 'alert');
      next.append(title, note, error, button('保存', () => {
        const ok = item ? model.rename(item.id, title.value, note.value) : model.add(message, title.value, note.value);
        if (ok) { closeDialog(); label.textContent = '书签已保存，可在「书签」中继续阅读'; void model.flush(); }
        else error.textContent = model.snapshot().error || '暂时无法保存书签';
      }), button('取消', closeDialog)); title.focus();
    }
    function add(message) { if (message?.id && model.snapshot().supported) editor(model.snapshot().bookmarks.find(b => b.anchor?.message_id === message.id) || null, message); }
    async function jump(id, resume = true) {
      if (jumping) return; capture(); const ticket = ++generation; jumping = true; intent = 0;
      const position = model.getPosition(id, resume), state = model.snapshot();
      const pending = state.queue.some(q => q.bookmark_id === id && ['create', 'progress', 'resolve'].includes(q.action));
      const query = pending && position ? { around: position.message_id } : { bookmark: id, resume: String(resume) };
      closeDialog(); options.history?.(true);
      try {
        const target = await options.navigate(query);
        if (!alive || ticket !== generation) return;
        if (!target?.resolved_id) throw new Error('暂时无法定位消息，请重试');
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        if (!alive || ticket !== generation) return;
        const node = rows().find(n => n.dataset.messageId === target.resolved_id);
        if (!node) throw new Error('定位的消息暂不可见，请关闭筛选后重试');
        const fraction = target.status === 'exact' ? (position?.message_id === target.resolved_id ? position.fraction || 0 : target.fraction || 0) : 0;
        list.scrollTop += node.getBoundingClientRect().top - list.getBoundingClientRect().top + fraction * node.getBoundingClientRect().height;
        history = true; markerDestination = target.resolved_id; model.activate(id); node.setAttribute('tabindex', '-1'); node.focus({ preventScroll: true });
        if (target.status !== 'exact') label.textContent = '原消息已删除，已定位到附近消息；书签原始位置保留';
      } catch (e) { label.textContent = e.message || '跳转失败，请重试'; options.history?.(history); }
      finally { jumping = false; }
    }
    async function latest() {
      capture(); jumping = true; intent = 0;
      try { if (await options.latest()) { history = false; model.activate(''); options.history?.(false); } }
      finally { jumping = false; render(); }
    }
    function progressActions(next, id, progress, candidates) {
      if (!candidates?.length) return;
      next.append(element('p', '检测到其他设备的阅读位置，请选择保留的位置：'));
      const positions = [progress?.position, ...candidates.map(c => c.position)].filter(Boolean);
      positions.forEach(position => next.append(button(`保留 ${position.created_at || position.message_id}`, async () => {
        model.resolve(id, position); await model.flush(); show();
      })));
    }
    function show() {
      const state = model.snapshot(), next = openDialog('阅读书签');
      next.append(element('p', '原始标记固定不变；从「继续阅读」进入后，会保存各自的阅读进度。'));
      if (state.error) next.append(element('p', state.error));
      state.queue.filter(q => q.blocked).forEach(q => {
        next.append(element('p', `有一项本机修改尚未同步：${q.title || q.action}`),
          button('保留本机修改并重试', async () => { await model.retryOperation(q.operation_id); show(); }),
          button('放弃这项本机修改', async () => { await model.retryOperation(q.operation_id, true); show(); }));
      });
      if (state.conversation_progress || model.getPosition('')) next.append(button('继续上次会话阅读', () => void jump('')));
      progressActions(next, '', state.conversation_progress, state.conversation_candidates);
      state.bookmarks.sort((a, b) => String(b.progress?.updated_at || b.updated_at || b.created_at).localeCompare(String(a.progress?.updated_at || a.updated_at || a.created_at))).forEach(item => {
        const section = element('section'); section.append(element('h3', item.title), element('p', item.note || new Date(item.created_at).toLocaleString()));
        section.append(button('继续阅读', () => void jump(item.id)), button('回到原始标记', () => void jump(item.id, false)),
          button('编辑', () => editor(item)), button('另存为新书签', () => { model.add({ id: item.anchor.message_id, created_at: item.anchor.created_at }, item.title + '（副本）', item.note, true); show(); }),
          button('删除', () => { deleted = model.remove(item.id); show(); }));
        progressActions(section, item.id, item.progress, item.candidates); next.append(section);
      });
      if (!state.bookmarks.length) next.append(element('p', '还没有书签。可在消息菜单中添加，或标记当前阅读位置。'));
      if (deleted) next.append(button('撤销刚才的删除', () => { model.restore(deleted); deleted = null; show(); }));
      next.append(button('刷新与同步', async () => { await model.load(); await model.flush(); if (alive) show(); }), button('关闭', closeDialog));
    }
    const bookmark = event => add(event.detail);
    const pause = () => { capture(); void model.flush(); }, wake = () => void model.load();
    list.addEventListener('reading-bookmark', bookmark); root.addEventListener('online', wake); root.addEventListener('pagehide', pause);
    model.activate(''); void model.load(); render();
    return { add, show, latest, capture, model, historical: () => history || jumping,
      suspend() { capture(); intent = 0; clearTimeout(timer); },
      close() { capture(); void model.flush(); alive = false; generation++; clearTimeout(timer); model.close(); closeDialog(); bar.remove();
        cancelAnimationFrame(markerFrame); mutations.disconnect(); resize.disconnect(); observedRows.clear();
        rows().forEach(row => { row.querySelector(':scope > .reading-message-marker')?.remove(); row.classList.remove('reading-marked-message'); });
        events.forEach(name => list.removeEventListener(name, userInput)); list.removeEventListener('scroll', scrolled);
        list.removeEventListener('reading-bookmark', bookmark); root.removeEventListener('online', wake); root.removeEventListener('pagehide', pause); } };
  }
  root.ElonReadingBookmarksUI = { mount };
})(globalThis);
