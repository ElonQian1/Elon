const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const base = process.env.READING_FIXTURE_URL || 'http://127.0.0.1:5198/pc/tests/fixtures/reading-bookmarks.html';
(async () => {
  const { createServer } = await import('../node_modules/vite/dist/node/index.js');
  const server = process.env.READING_FIXTURE_URL ? null : await createServer({ root: require('node:path').resolve(__dirname, '..'), server: { host: '127.0.0.1', port: 5198, strictPort: true } });
  await server?.listen();
  const browser = await chromium.launch({ headless: true, channel: 'msedge' });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const bookmarks = [], writes = [], errors = []; page.on('pageerror', e => errors.push(e.message));
    const row = n => ({ id: String(n).padStart(6, '0'), content: '合成消息 ' + n, sender_user_id: 'peer', sender_name: '测试群友', created_at: '2026-10-02T01:00:00Z', timeline_cursor: 'b' + n, timeline_after_cursor: 'a' + n });
    await page.route(url => url.pathname.startsWith('/api/'), route => {
      const r = route.request(), u = new URL(r.url()), a = u.searchParams;
      const ok = json => route.fulfill({ json });
      if (u.pathname.endsWith('/reading-capabilities')) return ok({ reading_bookmarks: true, timeline_around: true });
      if (u.pathname.endsWith('/reading-bookmarks')) {
        if (r.method() === 'GET') return ok({ bookmarks, next: null });
        const op = r.postDataJSON(); writes.push(op);
        if (op.action === 'create') bookmarks.push({ id: op.bookmark_id, title: op.title, note: op.note, anchor: op.position, revision: 1 });
        if (op.action === 'progress') return ok({ progress: { position: op.position, revision: writes.length } });
        return ok({ revision: 1 });
      }
      if (u.pathname === '/api/me/message-timeline/read') return ok({});
      if (u.pathname.startsWith('/api/me/message-timeline')) {
        let start = 950, target;
        if (a.has('bookmark') || a.has('around')) {
          const id = a.get('around') || bookmarks.find(b => b.id === a.get('bookmark')).anchor.message_id;
          start = Math.max(0, Number(id) - 24); target = { resolved_id: id, status: 'exact', fraction: 0 };
        } else if (a.has('after')) start = Number(a.get('after').slice(1)) + 1;
        else if (a.has('before')) start = Math.max(0, Number(a.get('before').slice(1)) - 50);
        const messages = a.has('sync') ? [] : Array.from({ length: 50 }, (_, i) => row(start + i));
        return ok({ schema: u.pathname.endsWith('/v2') ? 'elon.message_timeline.v2' : 'elon.message_timeline.v1', messages, removed_ids: [], has_older: start > 0, has_newer: start < 950, has_more: !a.has('sync') && start > 0, sync: 'live', before: messages[0]?.timeline_cursor, target });
      }
      return ok({ requests: [], tasks: [], members: [], messages: [] });
    });
    await page.goto(base);
    await page.locator('[data-message-id="000999"]').waitFor({ state: 'attached', timeout: 60000 }).catch(async e => {
      console.error(JSON.stringify({ errors, body: (await page.locator('body').innerText()).slice(0, 1800) })); throw e;
    });
    await page.getByRole('button', { name: '标记当前位置', exact: true }).click();
    await page.getByRole('textbox', { name: '书签名称' }).fill('Win 阅读位置');
    await page.getByRole('button', { name: '保存', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('dialog[open]'));
    for (let i = 0; i < 50 && !bookmarks.length; i++) await page.waitForTimeout(20);
    assert.equal(bookmarks.length, 1);
    await page.getByRole('button', { name: '书签', exact: true }).click();
    await page.getByRole('button', { name: '继续阅读', exact: true }).click();
    await page.waitForFunction(id => document.activeElement?.dataset.messageId === id, bookmarks[0].anchor.message_id);
    await page.getByRole('img', { name: '书签：Win 阅读位置', exact: true }).waitFor();
    const markerBox = await page.getByRole('img', { name: '书签：Win 阅读位置', exact: true }).boundingBox();
    const bubbleBox = await page.locator(`[data-message-id="${bookmarks[0].anchor.message_id}"] [data-reading-bubble]`).boundingBox();
    assert.ok(markerBox && bubbleBox && markerBox.x >= bubbleBox.x + bubbleBox.width, 'bookmark sits beside the incoming bubble');
    await page.getByRole('button', { name: '书签', exact: true }).click();
    await page.getByRole('button', { name: '关闭', exact: true }).click();
    assert.deepEqual(errors, []);
    console.log('PASS: production Win React conversation + timeline hook + bookmark controls create and locate through the versioned API');
  } finally { await browser.close(); await server?.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
