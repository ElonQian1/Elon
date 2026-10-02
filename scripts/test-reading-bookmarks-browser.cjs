const assert = require('node:assert/strict');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
(async () => {
  const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await page.route('https://reading.test/**', route => route.fulfill({ contentType: 'text/html', body: '<style>#list{height:440px;overflow:auto;overflow-anchor:none}#list>div{height:60px}button{min-height:44px}</style><div id="list"></div>' }));
    await page.goto('https://reading.test');
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    for (const name of ['message_timeline', 'reading_positions', 'reading_bookmarks_ui', 'social_chat_recovery']) await page.addScriptTag({ path: path.resolve(__dirname, '../server/src/assets', name + '.js') });
    await page.addStyleTag({ path: path.resolve(__dirname, '../server/src/assets/reading_bookmarks.css') });
    await page.evaluate(async () => {
      const list = document.getElementById('list'), cache = new Map();
      const row = n => ({ id: String(n).padStart(6, '0'), created_at: '2026-10-02T01:00:00Z', content: 'fixture ' + n, timeline_cursor: 'b' + n, timeline_after_cursor: 'a' + n });
      window.ops = []; window.bookmarks = []; window.readReceipts = []; window.paths = [];
      window.chat = ElonSocialChatRecovery.create({ list, cache: { get: k => cache.get(k), put: (k, v) => cache.set(k, v), remove: k => cache.delete(k), clear: () => cache.clear() },
        session: () => 'test-session', userId: () => 'test-user', status: () => {}, directory: () => {},
        api: async (path, init) => {
          window.paths.push(path); const url = new URL(path, location.origin), args = url.searchParams;
          const respond = data => new Response(JSON.stringify(data));
          if (path.endsWith('/groups/g/messages') && init.method === 'POST') return respond({ message: row(99999) });
          if (path.endsWith('reading-capabilities')) return respond({ reading_bookmarks: true, timeline_around: true });
          if (url.pathname.endsWith('reading-bookmarks')) {
            if (init.method !== 'POST') return respond({ bookmarks: window.bookmarks, next: null });
            const op = JSON.parse(init.body); window.ops.push(op);
            if (op.action === 'create') window.bookmarks.push({ id: op.bookmark_id, title: op.title, note: op.note, anchor: op.position, revision: 1 });
            if (op.action === 'progress') { const p = { position: op.position, revision: window.ops.length }; const b = window.bookmarks.find(b => b.id === op.bookmark_id); if (b) b.progress = p; return respond({ progress: p }); }
            return respond({ revision: 1 });
          }
          if (path.endsWith('/read')) { window.readReceipts.push(JSON.parse(init.body)); return respond({}); }
          let start = 99950, end = 100000, target;
          if (args.has('bookmark') || args.has('around')) {
            if (window.failLocate) return new Response(JSON.stringify({ error: '书签不可用', code: 'reading_unavailable' }), { status: 404 });
            const b = window.bookmarks.find(b => b.id === args.get('bookmark'));
            const p = b ? args.get('resume') === 'true' && b.progress ? b.progress.position : b.anchor : { message_id: args.get('around') };
            const center = Number(p.message_id); start = Math.max(0, center - 24); end = Math.min(100000, start + 50);
            target = { resolved_id: row(center).id, status: 'exact', fraction: p.fraction || 0 };
          } else if (args.has('before')) { end = Number(args.get('before').slice(1)); start = Math.max(0, end - 50); }
          else if (args.has('after')) { start = Number(args.get('after').slice(1)) + 1; end = Math.min(start + 50, 100000); }
          else if (!args.has('sync') && window.failLatest) return new Response('{}', { status: 503 });
          const messages = args.has('sync') ? [] : Array.from({ length: end - start }, (_, i) => row(start + i));
          return respond({ schema: url.pathname.endsWith('/v2') ? 'elon.message_timeline.v2' : 'elon.message_timeline.v1', messages, removed_ids: [], before: messages[0]?.timeline_cursor, sync: 'checkpoint', has_more: start > 0 && !args.has('sync'), has_older: start > 0, has_newer: end < 100000, target });
        }, render: (rows, kind, contact, scroll) => {
          const top = list.scrollTop; list.replaceChildren(...rows.map(m => { const n = document.createElement('div'); n.dataset.messageId = m.id; n.textContent = m.content; return n; }));
          list.scrollTop = scroll ? list.scrollHeight : top;
        } });
      await window.chat.open('group', { id: 'g' });
    });
    await page.getByRole('button', { name: '书签', exact: true }).waitFor();
    async function add(id, title) {
      await page.evaluate(id => document.getElementById('list').dispatchEvent(new CustomEvent('reading-bookmark', { detail: { id, created_at: '2026-10-02T01:00:00Z' } })), id);
      await page.getByRole('textbox', { name: '书签名称' }).fill(title);
      await page.getByRole('button', { name: '保存', exact: true }).click();
      await page.waitForFunction(title => window.bookmarks.some(b => b.title === title), title);
    }
    await add('050000', '周一进度'); await add('075000', '周末进度');
    await page.getByRole('button', { name: '书签', exact: true }).click();
    await page.locator('section').filter({ hasText: '周一进度' }).getByRole('button', { name: '继续阅读', exact: true }).click();
    await page.waitForFunction(() => document.activeElement?.dataset.messageId === '050000');
    const ackCount = await page.evaluate(() => window.readReceipts.length);
    await page.locator('#list').hover(); await page.mouse.wheel(0, 180);
    await page.waitForFunction(() => window.ops.some(op => op.action === 'progress' && op.bookmark_id === window.bookmarks[0].id), { timeout: 10000 });
    const savedA = await page.evaluate(() => structuredClone(window.bookmarks[0].progress.position));
    assert.ok(Number(savedA.message_id) >= 50000 && Number(savedA.message_id) < 50100);
    for (let i = 0; i < 4; i++) await page.getByRole('button', { name: '继续向后阅读', exact: true }).click();
    await page.waitForTimeout(250);
    assert.ok(await page.locator('#list>div').count() <= 150);
    assert.equal(await page.evaluate(() => window.readReceipts.length), ackCount);
    await page.evaluate(() => { window.failLatest = true; });
    await page.getByRole('button', { name: '回到最新消息', exact: true }).click();
    await page.getByRole('status').filter({ hasText: '正在续读：周一进度' }).waitFor();
    await page.evaluate(() => { window.failLatest = false; });
    await page.getByRole('button', { name: '回到最新消息', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('#list>div:last-child')?.dataset.messageId === '099999');
    const latestRows = await page.locator('#list>div').count();
    await page.evaluate(() => { window.failLocate = true; });
    await page.getByRole('button', { name: '书签', exact: true }).click();
    await page.locator('section').filter({ hasText: '周末进度' }).getByRole('button', { name: '回到原始标记', exact: true }).click();
    await page.getByRole('status').filter({ hasText: '暂时无法定位' }).waitFor();
    assert.equal(await page.locator('#list>div').count(), latestRows);
    await page.evaluate(() => { window.failLocate = false; });
    await page.getByRole('button', { name: '书签', exact: true }).click();
    await page.locator('section').filter({ hasText: '周末进度' }).getByRole('button', { name: '回到原始标记', exact: true }).click();
    await page.waitForFunction(() => document.activeElement?.dataset.messageId === '075000');
    await page.evaluate(() => window.chat.send('group', { id: 'g' }, 'synthetic message'));
    assert.equal(await page.getByRole('status').filter({ hasText: '正在续读：' }).count(), 0);
    assert.deepEqual(await page.evaluate(() => window.bookmarks.map(b => b.anchor.message_id)), ['050000', '075000']);
    assert.deepEqual(errors, []);
    await page.evaluate(() => window.chat.destroy());
    console.log('PASS: mobile browser multi-bookmarks, durable progress, bounded pages, failure recovery, send/latest detaches bookmark, no skipped-history receipts');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
