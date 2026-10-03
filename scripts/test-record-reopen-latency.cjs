const assert = require('node:assert/strict');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+nmZkAAAAASUVORK5CYII=', 'base64');
const card = { schema: 'chat_record_bundle_v1', record_id: 'record_test', group_id: 'group_test', title: 'Cache fixture', summary: 'Fixture', message_count: 2, total_count: 2 };
const view = { card, owner_id: 'author', document: { title: card.title, raw_text: '', warnings: [], messages: [1, 2].map(n => ({ id: String(n), parent_id: null, sender: 'Fixture', time: '', kind: 'image', text: '', filename: `image${n}.png`, asset_id: `image${n}` })) } };
let held = false, pending = [], status = 200, downloads = 0, reads = 0;
const server = http.createServer((req, res) => {
  if (req.url.startsWith('/api/me/groups/')) {
    if (req.url.includes('/assets/')) { downloads++; res.writeHead(200, { 'Content-Type': 'image/png', 'Cache-Control': 'no-store' }); res.end(image); return; }
    reads++;
    const reply = () => { res.writeHead(status, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' }); res.end(JSON.stringify(view)); };
    if (held) pending.push(reply); else reply();
    return;
  }
  const upstream = http.request({ hostname: '127.0.0.1', port: +(process.env.RECORD_VITE_PORT || 5198), path: req.url, method: req.method, headers: req.headers }, r => { res.writeHead(r.statusCode, r.headers); r.pipe(res); });
  upstream.on('error', () => { res.writeHead(502); res.end(); }); req.pipe(upstream);
});
const release = () => { held = false; pending.splice(0).forEach(reply => reply()); };
(async () => {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  try {
    for (const width of [1280, 390]) {
      downloads = reads = 0; status = 200; release();
      const context = await browser.newContext({ viewport: { width, height: 1000 } });
      const page = await context.newPage(), errors = [];
      page.on('pageerror', e => errors.push(e.message));
      await page.clock.install();
      await page.goto(`http://127.0.0.1:${server.address().port}/pc/tests/fixtures/chat-records.html`);
      const open = () => page.getByRole('button', { name: '打开测试记录', exact: true }).click();
      const close = () => page.getByRole('button', { name: '关闭', exact: true }).click();
      const images = () => page.waitForFunction(() => [...document.querySelectorAll('article img')].filter(i => i.naturalWidth === 1).length === 2, null, { timeout: 1500 });
      await open(); await images(); assert.equal(downloads, 2); await close();
      held = true;
      const started = Date.now(); await open();
      await images(); // Must finish while the document request is deliberately unanswered.
      assert.equal(held, true); assert.equal(downloads, 2); assert.ok(reads >= 2);
      console.log(`RECORD_HOT_OPEN width=${width} images_ms=${Date.now() - started} attachment_downloads=${downloads} network_document_held=true`);
      release(); await page.waitForTimeout(100); await close();
      held = true; await page.reload(); await open(); await images();
      assert.equal(downloads, 2, 'reload must reuse persisted document and image bytes');
      release(); await page.waitForTimeout(100); await close();
      status = 503; await open(); await images();
      await page.getByText(/读取失败（503）/).waitFor(); await close();
      status = 200; held = true; await open(); await images();
      assert.equal(downloads, 2, 'transient errors must not delete image cache');
      // Explicit denial must clear already displayed local data, even inside the lease.
      status = 403; release();
      await page.getByRole('alert').filter({ hasText: '记录已撤回，或你已不在此群聊中' }).waitFor();
      assert.equal(await page.locator('article').count(), 0);
      await close(); status = 200; held = true; await open();
      await page.waitForTimeout(150); assert.equal(await page.locator('article').count(), 0);
      release(); await images(); assert.equal(downloads, 4, 'revoked media must have been invalidated');
      // An open reader renews in place instead of blanking at five minutes on a healthy network.
      const beforeRenewal = reads;
      await page.clock.fastForward(5 * 60_000 - 7900); await page.waitForTimeout(200);
      assert.ok(reads > beforeRenewal, 'an open reader must renew its expiring lease'); await images();
      // A failed refresh cannot keep an open cached document visible indefinitely.
      await close(); held = true; await open(); await images();
      await page.clock.fastForward(5 * 60_000 + 100);
      assert.equal(await page.locator('article').count(), 0, 'lease expiry must hide even an already open reader');
      await close(); release();
      held = true; await open(); await page.waitForTimeout(150);
      assert.equal(await page.locator('article').count(), 0, 'expired disk data must wait for authorization');
      release(); await images(); await close();
      held = true;
      await page.evaluate(async () => {
        const { useAuthStore } = await import('/pc/src/store/auth.ts');
        useAuthStore.getState().acceptSession('different-session', '2099-01-01', { id: 'reader', account: 'Fixture' });
      });
      await open(); await page.waitForTimeout(150);
      assert.equal(await page.locator('article').count(), 0, 'a new login session must not inherit the old authorization lease');
      release();
      assert.deepEqual(errors, []); await context.close();
      console.log(`RECORD_REOPEN=passed width=${width} warm/disk-reload/transient-error/denial/expiry/session`);
    }
  } finally { release(); await browser.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
})().catch(e => { release(); server.closeAllConnections(); server.close(); console.error(e); process.exitCode = 1; });
