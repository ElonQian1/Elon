const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const path = require('node:path');
const rootPath = path.resolve(__dirname, '..');
class Storage {
  getItem(key) { return this[key] ?? null; }
  setItem(key, value) { this[key] = String(value); }
  removeItem(key) { delete this[key]; }
}
function fixture(api = async () => response([])) {
  const target = new EventTarget(), document = new EventTarget();
  document.visibilityState = 'visible';
  const root = vm.createContext({ console, crypto: webcrypto, TextEncoder, AbortController, setTimeout, clearTimeout, setInterval, clearInterval, document,
    addEventListener: target.addEventListener.bind(target), removeEventListener: target.removeEventListener.bind(target) });
  for (const name of ['social_chat_cache', 'social_chat_recovery']) vm.runInContext(fs.readFileSync(path.join(rootPath, 'server/src/assets', name + '.js'), 'utf8'), root);
  const storage = new Storage(), cache = root.ElonSocialChatCache.create(storage);
  let identity = 'synthetic-a', user = 'a', wakeCount = 0;
  const frames = [], directories = [], errors = [];
  const recovery = root.ElonSocialChatRecovery.create({ api, cache, session: () => identity, userId: () => user,
    render: (rows, kind, contact) => frames.push({ rows, kind, id: contact.id }), status: text => errors.push(text),
    directory: (kind, rows) => directories.push({ kind, rows }), wake: () => wakeCount++ });
  return { root, storage, cache, recovery, frames, directories, errors, document, target,
    account: id => { user = id; identity = 'synthetic-' + id; }, wakeCount: () => wakeCount };
}
const response = (messages, status = 200) => new Response(JSON.stringify({ messages, groups: messages, friends: messages }), { status });
const msg = (id, extra = {}) => ({ id, content: id, revision: 1, ...extra });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
function gate() { let done; const promise = new Promise(resolve => { done = resolve; }); return { promise, done }; }

test('cache survives recreation, expires, rejects oversized/corrupt writes and binds profile without credentials', async () => {
  const f = fixture(); let now = 100;
  const cache = f.root.ElonSocialChatCache.create(f.storage, () => now);
  cache.put('a:group:g', [msg('saved')]);
  assert.equal(cache.get('a:group:g')[0].id, 'saved');
  const next = f.root.ElonSocialChatCache.create(f.storage, () => now);
  assert.equal(next.get('a:group:g')[0].id, 'saved');
  now += 8 * 86400000; assert.equal(next.get('a:group:g'), null);
  assert.equal(cache.put('huge', 'x'.repeat(1000001)), false);
  f.storage.setItem('elon-social-cache-v1:broken', '{'); assert.equal(cache.get('broken'), null);
  f.storage.setItem('elon-social-cache-v1:untimed', '{"v":1,"data":[]}'); assert.equal(cache.get('untimed'), null);
  for (let i = 0; i < 70; i++) cache.put('key' + i, [msg('m')]);
  assert.ok(Object.keys(f.storage).length <= 40);
  await cache.profile('synthetic-token', { id: 'a', nickname: 'Test' });
  for (let i = 70; i < 140; i++) cache.put('key' + i, [msg('m')]);
  assert.equal((await cache.profile('synthetic-token')).id, 'a');
  assert.equal(await cache.profile('different'), null);
  assert.ok(!JSON.stringify(f.storage).includes('synthetic-token'));
  cache.clear(); assert.equal(Object.keys(f.storage).length, 0); f.recovery.destroy();
});

test('cached messages render before slow network, unchanged polling does not redraw', async () => {
  const pending = gate(); let calls = 0;
  const f = fixture(async () => { calls++; await pending.promise; return response([msg('cached'), msg('fresh')]); });
  f.cache.put('a:group:g', [msg('cached')]);
  const loading = f.recovery.open('group', { id: 'g' });
  assert.equal(f.frames.at(-1).rows[0].id, 'cached'); assert.equal(calls, 1);
  pending.done(); await loading;
  assert.equal(f.frames.at(-1).rows.length, 2); const count = f.frames.length;
  await f.recovery.refresh(); assert.equal(f.frames.length, count); f.recovery.destroy();
});

test('one in-flight read coalesces burst and schedules a trailing refresh', async () => {
  const pending = gate(); let calls = 0;
  const f = fixture(async () => { calls++; if (calls === 1) await pending.promise; return response([msg('m' + calls)]); });
  const loading = f.recovery.open('group', { id: 'g' });
  for (let i = 0; i < 10; i++) f.recovery.refresh();
  assert.equal(calls, 1); pending.done(); await loading; await delay(10);
  assert.equal(calls, 2); assert.equal(f.frames.at(-1).rows[0].id, 'm2'); f.recovery.destroy();
});

test('switching conversation or account drops late read and does not poison new cache', async () => {
  const pending = gate();
  const f = fixture(async path => { if (path.includes('/old/')) await pending.promise; return response([msg(path.includes('/old/') ? 'stale' : 'new')]); });
  const old = f.recovery.open('group', { id: 'old' });
  f.account('b'); await f.recovery.open('group', { id: 'new' }); pending.done(); await old;
  assert.equal(f.frames.at(-1).rows[0].id, 'new'); assert.equal(f.cache.get('b:group:old'), null);
  assert.equal(f.cache.get('a:group:old'), null); f.recovery.destroy();
});

test('permission denial clears cached content while ordinary network failure preserves it', async () => {
  let denied = false;
  const f = fixture(async () => { if (denied) return response([], 403); throw Error('offline'); });
  f.cache.put('a:group:g', [msg('saved')]); await f.recovery.open('group', { id: 'g' });
  assert.equal(f.frames.at(-1).rows[0].id, 'saved');
  denied = true; await f.recovery.refresh(); assert.equal(f.frames.at(-1).rows.length, 0);
  assert.equal(f.cache.get('a:group:g'), null); f.recovery.destroy();
});

test('edited and recalled messages never regress in screen or disk snapshots', async () => {
  const f = fixture(async () => response([msg('edit'), msg('recall')]));
  f.cache.put('a:group:g', [msg('edit', { revision: 3, content: 'corrected' }), msg('recall', { recalled_at: 'now', content: '' })]);
  await f.recovery.open('group', { id: 'g' });
  assert.equal(f.frames.at(-1).rows[0].revision, 3); assert.ok(f.frames.at(-1).rows[1].recalled_at);
  assert.equal(f.cache.get('a:group:g')[0].content, 'corrected'); f.recovery.destroy();
});

test('pending send does not block reads and receipt follows original conversation without duplicates', async () => {
  const sending = gate(); let accepted = false;
  const f = fixture(async (path, init) => {
    if (init.method === 'POST') { await sending.promise; accepted = true; return new Response(JSON.stringify({ message: msg('sent', { outgoing: true }) })); }
    if (path.includes('/messages')) return response([msg('incoming'), ...(accepted ? [msg('sent', { outgoing: true })] : [])]);
    return response([]);
  });
  await f.recovery.open('group', { id: 'g' });
  const send = f.recovery.send('group', { id: 'g' }, 'draft');
  await f.recovery.refresh(); assert.equal(f.frames.at(-1).rows.length, 2);
  assert.equal(f.cache.get('a:group:g').length, 1);
  await f.recovery.open('group', { id: 'other' }); sending.done(); await send;
  await f.recovery.open('group', { id: 'g' });
  assert.equal(f.frames.at(-1).rows.filter(m => m.id === 'sent').length, 1);
  assert.equal(f.frames.at(-1).rows.some(m => m.send_status), false); f.recovery.destroy();
});

test('hidden page pauses reads; visibility and online wake refresh immediately', async () => {
  let calls = 0;
  const f = fixture(async () => { calls++; return response([]); });
  await f.recovery.open('group', { id: 'g' }); const initial = calls;
  f.document.visibilityState = 'hidden'; f.document.dispatchEvent(new Event('visibilitychange'));
  await f.recovery.refresh(); assert.equal(calls, initial);
  f.document.visibilityState = 'visible'; f.document.dispatchEvent(new Event('visibilitychange'));
  f.target.dispatchEvent(new Event('online')); await delay(260);
  assert.ok(calls > initial); assert.equal(f.wakeCount(), 1); f.recovery.destroy();
});

test('directory caches are independent; one stalled endpoint does not hide the other', async () => {
  const pending = gate(); const f = fixture(async path => { if (path.endsWith('/friends')) await pending.promise; return response([msg(path.endsWith('/groups') ? 'g' : 'f')]); });
  const load = f.recovery.directories(); await delay(10);
  assert.equal(f.directories[0].kind, 'groups'); pending.done(); await load; f.recovery.destroy();
});

test('JSON body has deadline, failed storage is non-fatal, logout fences late writes', async () => {
  const pending = gate(); const f = fixture(async () => ({ ok: true, json: () => pending.promise }));
  await assert.rejects(f.recovery.json('/api/me', {}, undefined, 20), /同步超时/);
  f.storage.setItem = () => { throw Error('quota'); }; assert.equal(f.cache.put('x', []), false);
  const request = f.recovery.open('group', { id: 'g' }); f.recovery.reset(); pending.done({ messages: [msg('late')] }); await request;
  assert.equal(f.cache.get('a:group:g'), null); f.recovery.destroy();
});

test('mobile template and public shell worker remain syntactically valid', () => {
  const html = fs.readFileSync(path.join(rootPath, 'server/src/assets/web_page.html'), 'utf8');
  for (const match of html.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/g)) new vm.Script(match[1]);
  new vm.Script(fs.readFileSync(path.join(rootPath, 'server/src/assets/mobile_shell_worker.js'), 'utf8'));
});
