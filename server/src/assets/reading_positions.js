(function (root) {
  'use strict';
  const uid = () => root.crypto?.randomUUID?.() || `reading-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  const clone = value => JSON.parse(JSON.stringify(value));
  const scopeKey = s => JSON.stringify([s.kind, s.project || '', s.id]);
  function create(options) {
    const { owner, scope, request } = options;
    const storage = options.storage || root.localStorage;
    const key = `elon.reading.v1:${owner}:${scopeKey(scope)}`;
    let alive = true, busy = false, active = null, timer = null, sending = '', supported = false, unavailable = false, error = '';
    let state = { bookmarks: [], conversation_progress: null, conversation_candidates: [], queue: [], sequence: 0, device: uid() };
    try {
      const saved = JSON.parse(storage.getItem(key) || 'null');
      if (saved && Array.isArray(saved.bookmarks) && Array.isArray(saved.queue)) state = { ...state, ...saved };
    } catch { error = '无法读取本机书签记录'; }
    supported = state.supported === true;
    state.device = uid(); state.sequence = 0;
    const valid = () => alive && (!options.current || options.current());
    function snapshot() {
      return clone({ ...state, active: active?.id ?? null, supported, unavailable, error, syncing: busy, pending: state.queue.length });
    }
    function notify() { if (valid()) options.onChange?.(snapshot()); }
    function save(edit) {
      const before = clone(state);
      edit(state);
      try {
        const body = JSON.stringify(state);
        if (state.queue.length > 1000 || body.length > 1000000) throw new Error('书签本机空间已满，请联网同步后重试');
        storage.setItem(key, body); error = ''; notify(); return true;
      } catch (e) { state = before; error = e.message || '本机保存失败，请重试'; notify(); return false; }
    }
    function entry(id) { return id ? state.bookmarks.find(b => b.id === id) : null; }
    function progress(id) { return id ? entry(id)?.progress : state.conversation_progress; }
    function pendingPosition(id) { return state.queue.filter(q => q.bookmark_id === id && ['progress', 'resolve'].includes(q.action)).at(-1)?.position; }
    function nextOperation() { const blocked = new Set(state.queue.filter(q => q.blocked).map(q => q.bookmark_id)); return state.queue.find(q => !blocked.has(q.bookmark_id)); }
    function getPosition(id, resume = true) { return resume ? pendingPosition(id) || progress(id)?.position || entry(id)?.anchor : entry(id)?.anchor; }
    function schedule() { clearTimeout(timer); if (valid()) timer = setTimeout(() => void flush(), 3000); }
    function operation(action, id, data = {}) {
      return { ...scope, project: scope.project || '', action, bookmark_id: id, operation_id: uid(), device_id: state.device, device_seq: ++state.sequence, ...data };
    }
    function activate(id) {
      active = id === null ? null : { id, revision: progress(id)?.revision || 0 };
      notify();
    }
    function putPosition(position, explicitId) {
      const id = explicitId ?? active?.id;
      if (id == null || !position?.message_id || unavailable) return false;
      const revision = active?.id === id ? active.revision : progress(id)?.revision || 0;
      const ok = save(s => {
        s.queue = s.queue.filter(q => !(q.action === 'progress' && q.bookmark_id === id && q.operation_id !== sending));
        s.queue.push(operation('progress', id, { position, base_revision: revision }));
      });
      if (ok) schedule();
      return ok;
    }
    function add(message, title = '', note = '', duplicate = false) {
      if (!message?.id || unavailable || !supported) return null;
      const existing = state.bookmarks.find(b => b.anchor?.message_id === message.id && !b.deleted);
      if (existing && !duplicate) return existing.id;
      const id = uid(), now = new Date().toISOString();
      const anchor = { message_id: message.id, created_at: message.created_at || '', fraction: 0 };
      const label = (title.trim() || `书签 ${new Date().toLocaleString()}`).slice(0, 80);
      if (!save(s => {
        s.bookmarks.push({ id, title: label, note, anchor, revision: 1, created_at: now, updated_at: now, progress: null, local: true });
        s.queue.push(operation('create', id, { position: anchor, title: label, note }));
      })) return null;
      schedule(); return id;
    }
    function rename(id, title, note) {
      const item = entry(id); if (!item || unavailable) return false;
      const ok = save(s => {
        const create = s.queue.find(q => q.action === 'create' && q.bookmark_id === id && !q.attempted && q.operation_id !== sending);
        if (create) { create.title = title; create.note = note; }
        else s.queue.push(operation('update', id, { base_revision: item.revision, title, note }));
        item.title = title; item.note = note;
      });
      if (ok) schedule(); return ok;
    }
    function remove(id) {
      const item = entry(id); if (!item) return null;
      const removed = clone(item);
      const ok = save(s => {
        const unsent = s.queue.some(q => q.action === 'create' && q.bookmark_id === id && !q.attempted && q.operation_id !== sending);
        s.queue = s.queue.filter(q => q.bookmark_id !== id || q.operation_id === sending || (q.action === 'create' && q.attempted));
        if (!unsent) s.queue.push(operation('delete', id, { base_revision: item.revision }));
        s.bookmarks = s.bookmarks.filter(b => b.id !== id);
      });
      if (ok) { if (active?.id === id) active = null; schedule(); notify(); return removed; }
      return null;
    }
    function restore(item) {
      const id = add({ id: item.anchor.message_id, created_at: item.anchor.created_at }, item.title, item.note, true);
      if (id && item.progress?.position) putPosition(item.progress.position, id);
      return id;
    }
    function resolve(id, position) {
      const ok = save(s => { s.queue = s.queue.filter(q => q.bookmark_id !== id || q.operation_id === sending); s.queue.push(operation('resolve', id, { position, base_revision: progress(id)?.revision || 0 })); });
      if (ok) schedule(); return ok;
    }
    async function load() {
      if (!valid() || unavailable) return;
      try {
        const capability = await request('GET', '/api/me/reading-capabilities');
        if (!valid()) return;
        supported = capability.reading_bookmarks === true && capability.timeline_around === true;
        if (!supported) { notify(); return; }
        let next = '', rows = [], ordinary = null, candidates = [];
        do {
          const args = new URLSearchParams({ ...scope, project: scope.project || '', after: next });
          const response = await request('GET', '/api/me/reading-bookmarks?' + args);
          if (!valid()) return;
          rows.push(...response.bookmarks); next = response.next || '';
          ordinary = response.conversation_progress; candidates = response.conversation_candidates || [];
        } while (next && rows.length < 100);
        save(s => {
          s.supported = supported;
          const removed = new Set(s.queue.filter(q => q.action === 'delete').map(q => q.bookmark_id));
          const local = s.bookmarks.filter(b => s.queue.some(q => q.action === 'create' && q.bookmark_id === b.id) && !rows.some(r => r.id === b.id));
          s.bookmarks = rows.filter(b => !removed.has(b.id)).concat(local);
          for (const edit of s.queue.filter(q => q.action === 'update')) {
            const b = s.bookmarks.find(b => b.id === edit.bookmark_id); if (b) { b.title = edit.title; b.note = edit.note; }
          }
          s.conversation_progress = ordinary; s.conversation_candidates = candidates;
        });
        if (active?.id && !entry(active.id)) { active = null; error = '正在阅读的书签已删除'; notify(); }
        void flush();
      } catch (e) {
        if (!valid()) return;
        if (e.status === 404) supported = false;
        if ([401, 403].includes(e.status)) { unavailable = true; active = null; }
        error = [401, 403].includes(e.status) ? '无法访问此会话的书签' : '暂未同步；已保留本机书签和阅读位置'; notify();
      }
    }
    async function flush() {
      clearTimeout(timer);
      if (busy || !valid() || unavailable || !supported) return;
      busy = true; notify();
      try {
        while (nextOperation() && valid()) {
          const first = nextOperation(); if (!first.attempted && !save(() => { first.attempted = true; })) return;
          const op = clone(first); sending = op.operation_id;
          const result = await request('POST', '/api/me/reading-bookmarks', op);
          if (!valid()) return;
          const ok = save(s => {
            s.queue = s.queue.filter(q => q.operation_id !== op.operation_id);
            const b = entry(op.bookmark_id);
            if (result.revision && b) b.revision = result.revision;
            if (result.revision) {
              let revision = result.revision;
              for (const queued of s.queue.filter(q => q.bookmark_id === op.bookmark_id && ['update', 'delete'].includes(q.action))) queued.base_revision = revision++;
            }
            if (op.action === 'create' && b) b.local = false;
            if (result.progress) {
              if (b) { b.progress = result.progress; b.candidates = result.candidates || []; }
              else if (!op.bookmark_id) { s.conversation_progress = result.progress; s.conversation_candidates = result.candidates || []; }
              if (!result.conflict && active?.id === op.bookmark_id) active.revision = result.progress.revision;
            }
          });
          if (!ok) return;
          if (result.conflict) { error = '阅读位置有冲突，请在书签列表选择保留的位置'; notify(); break; }
        }
      } catch (e) {
        if (!valid()) return;
        if ([400, 409, 429].includes(e.status)) save(s => { const q = s.queue.find(q => q.operation_id === sending); if (q) q.blocked = e.status; });
        if ([401, 403].includes(e.status)) { unavailable = true; active = null; }
        const failed = state.queue.find(q => q.operation_id === sending);
        if (e.status === 404 && failed?.action !== 'create') {
          // A deleted remote bookmark cannot be recreated by queued progress.
          const id = failed?.bookmark_id;
          save(s => { s.queue = s.queue.filter(q => q.bookmark_id !== id); s.bookmarks = s.bookmarks.filter(b => b.id !== id); });
          if (active?.id === id) active = null;
        }
        error = e.status === 409 ? '书签已变化，请刷新列表后重试' : '已保存到本机，待联网同步'; notify();
      } finally { sending = ''; busy = false; notify(); }
    }
    async function retryOperation(operationId, discard = false) {
      const op = state.queue.find(q => q.operation_id === operationId); if (!op) return;
      if (discard) { save(s => {
        s.queue = s.queue.filter(q => op.action === 'create' ? q.bookmark_id !== op.bookmark_id : q.operation_id !== operationId);
        if (op.action === 'create') s.bookmarks = s.bookmarks.filter(b => b.id !== op.bookmark_id);
      }); await load(); return; }
      await load();
      const revision = entry(op.bookmark_id)?.revision;
      save(() => { op.operation_id = uid(); delete op.blocked; if (['update', 'delete'].includes(op.action) && revision) op.base_revision = revision; });
      await flush();
    }
    function close() { clearTimeout(timer); alive = false; active = null; }
    return { snapshot, load, flush, add, rename, remove, restore, resolve, activate, putPosition, getPosition, retryOperation, close };
  }
  root.ElonReadingPositions = { create, scopeKey };
})(globalThis);
