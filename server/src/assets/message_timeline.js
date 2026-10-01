(function (root) {
  'use strict';
  const SCHEMA = 'elon.message_timeline.v1';
  const compare = (a, b) => (a.created_at < b.created_at ? -1 : a.created_at > b.created_at ? 1 : a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  const revision = m => Number(m.revision || 1);
  function create(options = {}) {
    const maxMessages = Math.max(50, Math.min(300, options.maxMessages || 150));
    const maxBytes = options.maxBytes || 1500000;
    let rows = [], before = null, sync = null, hasOlder = false, hasNewer = false, following = true, unread = 0;
    function reset() { rows = []; before = sync = null; hasOlder = hasNewer = false; following = true; unread = 0; }
    function snapshot() { return { messages: rows, before, sync, hasOlder, hasNewer, following, unread }; }
    function bound(direction) {
      let bytes = rows.reduce((sum, m) => sum + new TextEncoder().encode(JSON.stringify(m)).length, 0);
      while (rows.length > 1 && (rows.length > maxMessages || bytes > maxBytes)) {
        const removed = direction === 'older' ? rows.pop() : rows.shift();
        bytes -= new TextEncoder().encode(JSON.stringify(removed)).length;
        if (direction === 'older') hasNewer = true; else hasOlder = true;
      }
      before = rows[0]?.timeline_cursor || before;
    }
    function apply(page, direction = 'sync') {
      if (!page || page.schema !== SCHEMA || !Array.isArray(page.messages) || !Array.isArray(page.removed_ids)) throw new Error('无效的消息分页响应');
      if (page.reset) return { ...snapshot(), reset: true };
      if (direction === 'latest') { rows = []; hasNewer = false; following = true; unread = 0; }
      if (direction === 'window') { hasNewer = true; following = false; }
      const prior = new Map(rows.map(m => [m.id, m]));
      const oldest = rows[0];
      const removed = new Set(page.removed_ids);
      removed.forEach(id => prior.delete(id));
      for (const message of page.messages) {
        if (!message || typeof message.id !== 'string' || typeof message.created_at !== 'string') throw new Error('无效的消息记录');
        const old = prior.get(message.id);
        if (direction === 'sync' && !old && oldest && compare(message, oldest) < 0) continue;
        if (direction === 'sync' && !old && (!following || hasNewer)) { unread++; hasNewer = true; continue; }
        prior.set(message.id, old && !message.recalled_at && (old.recalled_at || revision(old) > revision(message)) ? old : message);
      }
      rows = Array.from(prior.values()).sort(compare);
      if (direction !== 'sync' && direction !== 'window') { hasOlder = page.has_more; before = page.before; }
      if (page.sync) sync = page.sync;
      bound(direction === 'older' || !following ? 'older' : 'latest');
      return snapshot();
    }
    function query(scope, direction = 'sync') {
      const args = new URLSearchParams({ kind: scope.kind, id: scope.id, limit: '50' });
      if (scope.project) args.set('project', scope.project);
      if (direction === 'older' && before) args.set('before', before);
      else if (direction === 'sync' && sync) args.set('sync', sync);
      return '/api/me/message-timeline?' + args.toString();
    }
    return { reset, snapshot, apply, query,
      follow(value) { following = value; if (value && !hasNewer) unread = 0; },
    };
  }
  root.ElonMessageTimeline = { SCHEMA, create, compare };
})(globalThis);
