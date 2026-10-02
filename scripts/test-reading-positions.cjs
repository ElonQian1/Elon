const test = require('node:test');
const assert = require('node:assert/strict');
require('../server/src/assets/reading_positions.js');
function fixture(storage = { text: null, getItem() { return this.text; }, setItem(k, v) { this.text = v; } }) {
  const sent = [], rows = []; let reject = null, hold = null;
  const model = ElonReadingPositions.create({ owner: 'u', scope: { kind: 'group', id: 'g' }, storage,
    request: async (method, path, body) => {
      if (reject) throw reject;
      if (path.includes('capabilities')) return { reading_bookmarks: true, timeline_around: true };
      if (method === 'GET') return { bookmarks: rows, next: null, conversation_progress: null };
      sent.push(structuredClone(body));
      if (hold) { const fn = hold; hold = null; return fn(body); }
      return body.action === 'progress' ? { progress: { position: body.position, revision: sent.length } } : { revision: 1 };
    } });
  return { model, sent, rows, storage, offline(value) { reject = value ? new Error('offline') : null; }, hold(fn) { hold = fn; } };
}
test('multiple fixed anchors keep independent progress and latest does not overwrite them', async () => {
  const f = fixture(); await f.model.load();
  const a = f.model.add({ id: 'a' }), b = f.model.add({ id: 'b' });
  f.model.activate(a); f.model.putPosition({ message_id: 'a10', fraction: .3 });
  f.model.activate(b); f.model.putPosition({ message_id: 'b20', fraction: .4 });
  f.model.activate(''); f.model.putPosition({ message_id: 'latest', fraction: 0 }); await f.model.flush();
  assert.equal(f.model.getPosition(a, false).message_id, 'a');
  assert.equal(f.model.getPosition(a).message_id, 'a10'); assert.equal(f.model.getPosition(b).message_id, 'b20'); f.model.close();
});
test('offline close and restart retains exact operation IDs and positions', async () => {
  const f = fixture(); await f.model.load(); f.offline(true);
  const id = f.model.add({ id: 'm' }); f.model.activate(id); f.model.putPosition({ message_id: 'later', fraction: .7 });
  await f.model.flush(); const ops = f.model.snapshot().queue.map(q => q.operation_id); f.model.close();
  const next = fixture(f.storage); next.offline(true);
  assert.deepEqual(next.model.snapshot().queue.map(q => q.operation_id), ops);
  assert.equal(next.model.getPosition(id).message_id, 'later'); assert.equal(next.model.snapshot().supported, true); next.model.close();
});
test('late acknowledgement cannot erase a newer pending position', async () => {
  const f = fixture(); await f.model.load(); const id = f.model.add({ id: 'm' }); await f.model.flush();
  let release; f.hold(() => new Promise(resolve => { release = resolve; }));
  f.model.activate(id); f.model.putPosition({ message_id: 'first' }); const flushing = f.model.flush();
  f.model.putPosition({ message_id: 'second' });
  release({ progress: { position: { message_id: 'first' }, revision: 1 } }); await flushing;
  assert.equal(f.model.getPosition(id).message_id, 'second'); assert.equal(f.model.snapshot().pending, 0); f.model.close();
});
test('disk write failure does not claim a successful bookmark', async () => {
  const f = fixture(); await f.model.load(); f.storage.setItem = () => { throw new Error('quota'); };
  assert.equal(f.model.add({ id: 'm' }), null); assert.equal(f.model.snapshot().bookmarks.length, 0); assert.match(f.model.snapshot().error, /quota/); f.model.close();
});
test('deleting a create whose acknowledgement was lost still sends the tombstone', async () => {
  const f = fixture(); await f.model.load(); const id = f.model.add({ id: 'm' });
  f.hold(() => { throw new Error('lost acknowledgement'); }); await f.model.flush();
  const original = f.sent[0]; f.model.remove(id); await f.model.flush();
  assert.equal(f.sent[1].operation_id, original.operation_id);
  assert.equal(f.sent[2].action, 'delete'); assert.equal(f.sent[2].bookmark_id, id); f.model.close();
});
test('cross-device conflict preserves candidates until explicit resolution', async () => {
  const f = fixture(); await f.model.load(); const id = f.model.add({ id: 'm' }); await f.model.flush();
  f.hold(() => ({ conflict: true, progress: { position: { message_id: 'remote' }, revision: 5 }, candidates: [{ position: { message_id: 'local' } }] }));
  f.model.activate(id); f.model.putPosition({ message_id: 'local' }); await f.model.flush();
  assert.equal(f.model.snapshot().bookmarks[0].candidates[0].position.message_id, 'local');
  assert.equal(f.model.getPosition(id).message_id, 'remote'); f.model.resolve(id, { message_id: 'local' });
  assert.equal(f.model.snapshot().queue[0].base_revision, 5); f.model.close();
});
