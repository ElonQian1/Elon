const test = require('node:test');
const assert = require('node:assert/strict');
require('../server/src/assets/message_timeline.js');
const row = n => ({ id: String(n).padStart(6, '0'), created_at: '2026-10-02', content: 'fixture', timeline_cursor: 'b' + n, timeline_after_cursor: 'a' + n });
const page = (from, to, extra = {}) => ({ schema: 'elon.message_timeline.v2', messages: Array.from({ length: to - from }, (_, i) => row(from + i)), removed_ids: [], sync: 'watermark', has_older: from > 0, has_newer: true, has_more: true, reset: false, ...extra });
test('bookmark target starts a bounded historical window without following live arrivals', () => {
  const t = ElonMessageTimeline.create({ maxMessages: 50 });
  t.apply(page(100, 150), 'around');
  assert.equal(t.snapshot().following, false);
  t.apply(page(1000, 1001), 'sync');
  assert.equal(t.snapshot().messages.at(-1).id, '000149');
});
test('forward history restores adjacent evicted messages and advances both retained boundaries', () => {
  const t = ElonMessageTimeline.create({ maxMessages: 50 });
  t.apply(page(100, 150), 'around');
  t.apply(page(150, 200), 'newer');
  assert.equal(t.snapshot().messages[0].id, '000150');
  assert.match(t.query({ kind: 'group', id: 'g' }, 'newer'), /after=a199/);
  assert.match(t.query({ kind: 'group', id: 'g' }, 'older'), /before=b150/);
  t.apply(page(100, 150), 'older');
  assert.equal(t.snapshot().messages.at(-1).id, '000149');
});
test('reaching latest through sequential history does not mark skipped history read', () => {
  const t = ElonMessageTimeline.create();
  t.apply(page(100, 150), 'around');
  t.apply(page(150, 180, { has_newer: false }), 'newer');
  assert.equal(t.snapshot().hasNewer, false);
  assert.equal(t.snapshot().following, false);
  t.apply(page(170, 180, { has_newer: false }), 'latest');
  assert.equal(t.snapshot().following, true);
});
