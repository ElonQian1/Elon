(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateTasksPolicy = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ID = /^[A-Za-z0-9_-]{1,128}$/;
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const filters = Object.freeze(['scheduled', 'paused', 'finished']);
  const fail = () => { throw Error('tasks_response_invalid'); };
  const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
  function text(value, max, optional = false) {
    if (optional && value == null) return null;
    if (typeof value !== 'string' || value.length > max) fail();
    return value;
  }
  function date(value) {
    if (value == null) return null;
    if (typeof value !== 'string' || value.length > 40 ||
        !/^\d{4}-\d{2}-\d{2}T[0-9:.]+(?:Z|[+-]\d{2}:\d{2})$/.test(value) ||
        !Number.isFinite(Date.parse(value))) fail();
    return value;
  }
  function path(value) {
    if (value == null || value === '') return null;
    if (typeof value !== 'string' || !UUID.test(value)) fail();
    return '/c/' + value;
  }
  function cursor(value) {
    if (value == null) return null;
    if (typeof value !== 'string' || !value.length || value.length > 2048 || /[\u0000-\u0020\u007f]/.test(value)) fail();
    return value;
  }
  function task(row) {
    if (!object(row) || typeof row.id !== 'string' || !ID.test(row.id) ||
        typeof row.is_enabled !== 'boolean' || !Array.isArray(row.next_run_times) || row.next_run_times.length > 100) fail();
    const title = text(row.title, 512);
    if (!title.trim() || /[\u0000-\u001f\u007f]/.test(title)) fail();
    const times = row.next_run_times.map(date);
    if (times.some(value => value === null)) fail();
    // Do not infer completion from next_run_times: condition watches can have none.
    return Object.freeze({ id: row.id, title, enabled: row.is_enabled,
      timingMode: ['exact_schedule', 'flexible_schedule', 'condition_watch'].includes(row.timing_mode) ? row.timing_mode : 'unknown',
      executor: row.executor === 'local' ? 'local' : row.executor == null ? 'unknown' : 'remote',
      sourcePath: path(row.source_conversation_id ?? row.initial_conversation_id ?? row.conversation_id),
      updatesPath: path(row.conversation_id), updatedAt: date(row.updated_at),
      lastRunAt: date(row.last_run_time), nextRunAt: times[0] ?? null });
  }
  function page(value) {
    if (!object(value) || !Array.isArray(value.items) || value.items.length > 200 ||
        !Object.prototype.hasOwnProperty.call(value, 'cursor')) fail();
    const items = value.items.map(task);
    if (new Set(items.map(row => row.id)).size !== items.length) fail();
    const nextCursor = cursor(value.cursor);
    return Object.freeze({ items: Object.freeze(items), nextCursor, complete: nextCursor === null });
  }
  function latest(value) {
    if (value === null) return Object.freeze({ state: 'no_update', update: null, lastRunFailed: null });
    if (!object(value)) fail();
    const wrapped = Object.prototype.hasOwnProperty.call(value, 'latest_update');
    const row = wrapped ? value.latest_update : value;
    const last = wrapped ? value.last_backing_run : null;
    if (last != null && (!object(last) || typeof last.status !== 'string')) fail();
    const lastRunFailed = last == null ? null : last.status === 'failed';
    if (row === null) return Object.freeze({ state: 'no_update', update: null, lastRunFailed });
    if (!object(row) || typeof row.id !== 'string' || !ID.test(row.id) || !object(row.metadata)) fail();
    const contentText = text(row.content_text, 128 * 1024, true);
    const requiresAction = row.metadata.automation_requires_user_action === true;
    if (!requiresAction && !contentText?.trim()) fail();
    const createdAt = date(row.created_at);
    if (!createdAt || row.content != null && !object(row.content)) fail();
    const content = row.content == null ? null : JSON.parse(JSON.stringify(row.content));
    if (JSON.stringify(content).length > 256 * 1024) fail();
    // Only the returned update is eligible for sharing, never its entire backing conversation.
    return Object.freeze({ state: requiresAction ? 'requires_action' : 'update',
      lastRunFailed: row.metadata.automation_last_backing_run_failed === true ? true : lastRunFailed,
      update: Object.freeze({ id: row.id, createdAt, contentText, content,
        fromLatestRun: row.metadata.automation_latest_update_is_from_latest_run === true }) });
  }
  return Object.freeze({ version: 1, filters, validId: value => typeof value === 'string' && ID.test(value), cursor, page, latest });
});
