(function (root) {
  'use strict';
  function reconcile(previous, incoming) {
    const prior = new Map(previous.filter(m => m.id).map(m => [m.id, m]));
    return Array.from(new Map(incoming.map(item => {
      const old = prior.get(item.id);
      const useOld = old && ((old.recalled_at && !item.recalled_at) || Number(old.revision || 1) > Number(item.revision || 1));
      return [item.id, useOld ? old : item];
    })).values());
  }
  function create(options) {
    const cache = options.cache;
    const timeline = root.ElonMessageTimeline.create({ maxMessages: 150 });
    let lastRead = '';
    const nav = document.createElement('div'), older = document.createElement('button'), latest = document.createElement('button');
    nav.hidden = true; nav.setAttribute('aria-label', '聊天历史'); older.textContent = '加载更早消息'; latest.textContent = '回到最新消息';
    [older, latest].forEach(button => { button.type = 'button'; button.style.minHeight = '48px'; nav.append(button); });
    options.list?.before(nav);
    let edgePending = false, historyLoading = false, lastTop = 0, intentUntil = 0, touchY = 0, edgeFrame = 0;
    function navigation() { const state = timeline.snapshot(); nav.hidden = !active; older.hidden = !state.hasOlder; latest.hidden = !state.hasNewer; older.disabled = historyLoading || edgePending; older.textContent = older.disabled ? '正在加载…' : '加载更早消息'; }
    function requestOlder() {
      if (historyLoading) return;
      if (!active || !visible() || !timeline.snapshot().hasOlder) { edgePending = false; return; }
      if (jobs.has(active.key)) { edgePending = true; navigation(); return; }
      edgePending = false; intentUntil = 0;
      if (edgeFrame) root.cancelAnimationFrame(edgeFrame); edgeFrame = 0;
      historyLoading = true; timeline.follow(false); void refresh(false, 'older');
    }
    function atEdge() { if (Date.now() < intentUntil && options.list?.scrollTop <= 96) requestOlder(); }
    function scheduleEdge() {
      if (historyLoading) return;
      intentUntil = Date.now() + 800;
      if (edgeFrame) root.cancelAnimationFrame(edgeFrame);
      edgeFrame = root.requestAnimationFrame(() => { edgeFrame = root.requestAnimationFrame(() => { edgeFrame = 0; atEdge(); }); });
    }
    function followPosition() {
      const list = options.list; if (!list) return;
      timeline.follow(list.scrollHeight - list.clientHeight - list.scrollTop < 80);
      if (list.scrollTop < lastTop && Date.now() < intentUntil) atEdge();
      lastTop = list.scrollTop; if (lastTop > 96) edgePending = false;
    }
    const wheel = event => { if (event.deltaY < 0) scheduleEdge(); else edgePending = false; };
    const touchStart = event => { touchY = event.touches[0]?.clientY || 0; };
    const touchMove = event => { if ((event.touches[0]?.clientY || 0) > touchY + 24) scheduleEdge(); };
    const keyboard = event => { if (['ArrowUp', 'PageUp', 'Home'].includes(event.key)) scheduleEdge(); };
    const edgeListeners = [['wheel', wheel], ['touchstart', touchStart], ['touchmove', touchMove], ['keydown', keyboard]];
    edgeListeners.forEach(([name, listener]) => options.list?.addEventListener(name, listener, { passive: true }));
    options.list?.addEventListener('scroll', followPosition, { passive: true });
    older.onclick = requestOlder;
    latest.onclick = () => { edgePending = false; timeline.follow(true); void refresh(true, 'latest'); };
    const jobs = new Map(), snapshots = new Map(), outbox = new Map();
    let owner = '', epoch = 0, active = null, rendered = '', timer = null, wakeTimer = null, ticks = 0;
    const visible = options.visible || (() => document.visibilityState !== 'hidden');
    const session = () => options.session() || '';
    const scopeKey = key => options.userId() + ':' + key;
    const contactKey = (kind, contact) => kind + ':' + contact.id;
    function cancel() { epoch++; jobs.forEach(job => job.controller.abort()); jobs.clear(); }
    function ensureOwner() {
      if (owner === session()) return;
      const changingAccount = !!owner;
      cancel(); snapshots.clear(); outbox.clear(); timeline.reset(); lastRead = ''; nav.hidden = true; active = null; rendered = ''; owner = session();
      if (changingAccount) options.accountChanged?.();
    }
    async function json(path, init = {}, controller = new AbortController(), timeout = 12000) {
      let deadline;
      try {
        return await Promise.race([
          (async () => {
            const res = await options.api(path, { ...init, signal: controller.signal, cache: 'no-store' });
            const data = await res.json().catch(() => ({}));
            if (!res.ok) throw Object.assign(new Error(data.error || '同步失败'), { status: res.status, authFailed: [401, 403].includes(res.status) });
            return data;
          })(),
          new Promise((_, reject) => { deadline = setTimeout(() => { controller.abort(); reject(new Error('同步超时，请重试')); }, timeout); }),
        ]);
      } finally { clearTimeout(deadline); }
    }
    function read(key, path, field, apply, failed) {
      ensureOwner();
      if (!owner || !visible()) return Promise.resolve();
      const old = jobs.get(key);
      if (old) { old.again = () => read(key, path, field, apply, failed); return old.promise; }
      const ticket = epoch, identity = owner, diskKey = scopeKey(key);
      const job = { controller: new AbortController(), again: null };
      const valid = () => ticket === epoch && identity === session() && jobs.get(key) === job;
      jobs.set(key, job);
      job.promise = json(path, {}, job.controller).then(data => {
        if (!valid()) return;
        const rows = data[field];
        if (!Array.isArray(rows) || rows.some(row => !row || typeof row !== 'object' || !row.id)) throw new Error('同步数据格式不正确');
        let merged = key.includes(':') ? reconcile(snapshots.get(key) || [], rows) : rows;
        if (options.normalize) merged = options.normalize(key, merged);
        snapshots.set(key, merged); cache.put(diskKey, merged); apply(merged);
      }).catch(error => {
        if (!valid()) return;
        if ([401, 403, 404].includes(error.status)) { cache.remove(diskKey); snapshots.delete(key); outbox.delete(key); }
        failed(error);
        if (error.status === 401) options.denied?.();
      }).finally(() => {
        if (!valid()) return;
        jobs.delete(key); if (job.again) job.again();
      });
      return job.promise;
    }
    function paint(scroll) {
      if (!active) return;
      const { key, kind, contact } = active;
      const rows = snapshots.get(key) || [], pending = outbox.get(key) || [];
      const ids = new Set(rows.map(m => m.id));
      const remaining = pending.filter(m => !m.id || !ids.has(m.id));
      outbox.set(key, remaining);
      const shown = rows.concat(remaining), signature = key + JSON.stringify(shown);
      if (signature !== rendered) { options.render(shown, kind, contact, scroll); rendered = signature; }
      options.status(shown.length ? '' : '还没有消息');
    }
    function refresh(scroll = false, direction = 'sync') {
      ensureOwner();
      if (!active || !visible()) return Promise.resolve();
      const { key, kind, contact } = active;
      const existing = jobs.get(key);
      if (existing) { if (direction !== 'sync' || !existing.again) existing.again = () => refresh(scroll, direction); return existing.promise; }
      const ticket = epoch, identity = owner;
      const job = { controller: new AbortController(), again: null };
      const valid = () => ticket === epoch && identity === session() && jobs.get(key) === job && active?.key === key;
      jobs.set(key, job); older.disabled = true;
      const mode = direction === 'sync' && !timeline.snapshot().sync ? 'latest' : direction;
      const list = options.list, top = list?.getBoundingClientRect().top || 0;
      const anchor = (mode === 'older' || mode === 'window') && Array.from(list?.children || []).find(node => node.dataset.messageId && node.getBoundingClientRect().bottom > top);
      const saved = anchor && { id: anchor.dataset.messageId, offset: anchor.getBoundingClientRect().top - top };
      const path = mode === 'window' ? '/api/me/message-timeline/window' : timeline.query({ kind, id: contact.id }, mode);
      const init = mode === 'window' ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ kind, id: contact.id, message_ids: timeline.snapshot().messages.map(m => m.id) }) } : {};
      job.promise = json(path, init, job.controller).then(async page => {
        if (!valid()) return;
        if (page.reset) { const s = timeline.snapshot(), live = s.following && !s.hasNewer; job.again = () => refresh(live, live ? 'latest' : 'window'); return; }
        const state = timeline.apply(page, mode);
        if (mode !== 'sync' || page.messages.length || page.removed_ids.length) {
          let rows = reconcile(snapshots.get(key) || [], state.messages);
          if (options.normalize) rows = options.normalize(key, rows);
          snapshots.set(key, rows); cache.put(scopeKey(key), rows); paint(scroll || mode === 'latest');
          if (saved && list) {
            const node = Array.from(list.children).find(node => node.dataset.messageId === saved.id);
            if (node) list.scrollTop += node.getBoundingClientRect().top - list.getBoundingClientRect().top - saved.offset;
          }
        }
        navigation();
        const id = state.messages.at(-1)?.id;
        if (state.following && !state.hasNewer && id && id !== lastRead && visible()) {
          await json('/api/me/message-timeline/read', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ kind, id: contact.id, message_id: id }) }, job.controller);
          if (valid()) lastRead = id;
        }
        if (mode === 'sync' && page.has_more && valid() && !job.again) job.again = () => refresh();
      }).catch(error => {
        if (!valid()) return;
        if ([401, 403, 404].includes(error.status)) { timeline.reset(); cache.remove(scopeKey(key)); snapshots.delete(key); outbox.delete(key); rendered = ''; paint(false); }
        options.status([401, 403, 404].includes(error.status) ? '无法访问此会话，请检查账号或成员权限' : '同步暂时失败，已保留现有消息 · 点击重试', () => refresh(false, mode));
      }).finally(() => {
        if (!valid()) return;
        jobs.delete(key); if (mode === 'older') historyLoading = false; navigation();
        if (job.again) job.again();
        if (edgePending) requestOlder();
      });
      return job.promise;
    }
    function directory(kind) {
      ensureOwner(); if (!owner) return Promise.resolve();
      if (!snapshots.has(kind)) {
        const saved = cache.get(scopeKey(kind));
        if (Array.isArray(saved)) { snapshots.set(kind, saved); options.directory(kind, saved); }
      }
      return read(kind, '/api/me/' + kind, kind, rows => options.directory(kind, rows), error => {
        if ([401, 403, 404].includes(error.status)) options.directory(kind, []);
        options.directoryError?.(kind, error);
      });
    }
    function directories() { return Promise.all([directory('friends'), directory('groups')]); }
    function close() { cancel(); active = null; rendered = ''; timeline.reset(); lastRead = ''; nav.hidden = true; options.status(''); edgePending = false; historyLoading = false; intentUntil = 0; if (edgeFrame) root.cancelAnimationFrame(edgeFrame); edgeFrame = 0; }
    function open(kind, contact) {
      ensureOwner(); close();
      const key = contactKey(kind, contact); active = { key, kind, contact };
      if (!snapshots.has(key)) {
        const saved = cache.get(scopeKey(key)); if (Array.isArray(saved)) snapshots.set(key, saved);
      }
      while (snapshots.size > 12) { const oldKey = Array.from(snapshots.keys()).find(item => item !== key && item.includes(':')); if (!oldKey) break; snapshots.delete(oldKey); }
      rendered = ''; paint(true);
      if (!(snapshots.get(key) || []).length) options.status('正在同步消息…');
      return refresh(true);
    }
    function invalidate(kind, id) {
      ensureOwner(); const key = contactKey(kind, { id });
      const job = jobs.get(key); if (job) { job.controller.abort(); jobs.delete(key); }
      cache.remove(scopeKey(key));
    }
    async function send(kind, contact, content, attachments = [], quoteSource = null, quote = null) {
      ensureOwner(); const identity = owner, key = contactKey(kind, contact);
      const pending = { client_id: 'local-' + Date.now() + '-' + Math.random(), content, attachments, quote, outgoing: true, created_at: new Date().toISOString(), send_status: '发送中…' };
      outbox.set(key, (outbox.get(key) || []).concat(pending)); if (active?.key === key) paint(true);
      try {
        const data = await json('/api/me/' + kind + 's/' + encodeURIComponent(contact.id) + '/messages', { method: 'POST', body: JSON.stringify({ content, attachments, ...(quoteSource ? { quote_source: quoteSource } : {}) }) }, undefined, 30000);
        if (identity !== session()) return;
        if (!data.message?.id) throw new Error('发送结果未确认，请刷新核对后再重试');
        Object.assign(pending, data.message, { send_status: '' });
        invalidate(kind, contact.id);
        if (active?.key === key) { paint(true); await refresh(true, 'latest'); }
        directories();
      } catch (error) {
        if (identity !== session()) return;
        pending.send_status = '发送结果未确认，请刷新核对后再重试';
        if (active?.key === key) paint(false);
        throw error;
      }
    }
    function pause() { cancel(); historyLoading = false; edgePending = false; navigation(); clearInterval(timer); timer = null; clearTimeout(wakeTimer); }
    function wake() {
      if (!visible() || !session()) return;
      cancel(); refresh(); directories(); options.wake?.();
      if (!timer) timer = setInterval(() => {
        if (!visible()) return;
        refresh(); if (++ticks % 5 === 0) directories();
      }, 15000);
    }
    function scheduleWake() { clearTimeout(wakeTimer); wakeTimer = setTimeout(wake, 200); }
    const visibilityChanged = () => visible() ? scheduleWake() : pause();
    const listeners = [['visibilitychange', visibilityChanged, root.document], ['pageshow', scheduleWake, root], ['focus', scheduleWake, root], ['online', scheduleWake, root], ['pagehide', pause, root]];
    listeners.forEach(([event, fn, target]) => target?.addEventListener(event, fn));
    return {
      open, close, refresh, directory, directories, invalidate, send, wake, json,
      reset() { pause(); close(); snapshots.clear(); outbox.clear(); cache.clear(); owner = ''; },
      destroy() { pause(); close(); nav.remove(); options.list?.removeEventListener('scroll', followPosition); edgeListeners.forEach(([event, fn]) => options.list?.removeEventListener(event, fn)); listeners.forEach(([event, fn, target]) => target?.removeEventListener(event, fn)); },
    };
  }
  root.ElonSocialChatRecovery = { create, reconcile };
})(globalThis);
