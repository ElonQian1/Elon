// Real HTTP cache (not Playwright routes, which disable caching) + production React/PWA readers.
const assert = require('node:assert/strict');
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const output = path.join(root, '.ai-tmp', 'chat-record-cache-ui');
const image = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+nmZkAAAAASUVORK5CYII=', 'base64');
const rows = [
  ['one', 'text', '项目讨论\n原始消息保留', null],
  ['article', 'link', '[链接] 合成预览标题 https://mp.weixin.qq.com/s/fixture-article', null],
  ['channels', 'channels', '[视频号] 视频描述 https://weixin.qq.com/sph/fixture', null],
  ['image', 'image', '[图片]', 'image_test'],
  ['video', 'video', '[视频]', 'video_test'],
  ['forward', 'forward', '[聊天记录]', null],
].map(([id, kind, text, asset_id]) => ({ id, kind, text, asset_id, parent_id: null, sender: '示例用户', time: '2026-09-28 12:00', filename: asset_id ? id : '' }));
rows.push({ ...rows[0], id: 'nested', parent_id: 'forward', text: '嵌套的聊天消息' });
const card = { schema: 'chat_record_bundle_v1', record_id: 'record_test', group_id: 'group_test', title: '微信聊天记录', summary: '示例', message_count: 6, total_count: 7 };
const view = { card, owner_id: 'author', document: { title: card.title, raw_text: 'Original text', warnings: [], messages: rows } };
let counts = {}, revoked = false, video = Buffer.alloc(0), sent = [];
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname === '/api/me/groups') { res.writeHead(200, { 'Content-Type': 'application/json' }); res.end(JSON.stringify({ groups: [{ id: 'target', name: '验收群' }] })); return; }
  if (url.pathname === '/api/me/groups/target/messages') {
    let body = ''; for await (const part of req) body += part;
    sent.push(JSON.parse(body)); res.writeHead(200, { 'Content-Type': 'application/json' }); res.end(JSON.stringify({ message: { id: 'sent' } })); return;
  }
  if (url.pathname.startsWith('/api/me/groups/')) {
    if (req.headers.authorization !== 'Bearer synthetic-session' || revoked) { res.writeHead(403, { 'Cache-Control': 'private, no-store' }); res.end('{}'); return; }
    const key = url.pathname.split('/').at(-1), tag = `"fixture-${key}-1"`;
    const hit = req.headers['if-none-match'] === tag;
    counts[`${key}:${hit ? 304 : 200}`] = (counts[`${key}:${hit ? 304 : 200}`] || 0) + 1;
    const bytes = key === 'image_test' ? image : key === 'video_test' ? video : Buffer.from(JSON.stringify(view));
    res.writeHead(hit ? 304 : 200, { ETag: tag, Vary: 'Authorization', 'Cache-Control': 'private, no-cache', 'Content-Type': key === 'image_test' ? 'image/png' : key === 'video_test' ? 'video/webm' : 'application/json' });
    res.end(hit ? undefined : bytes); return;
  }
  if (url.pathname === '/api/me/link-preview') {
    let body = ''; for await (const part of req) body += part;
    const link = JSON.parse(body).url;
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ schema: 1, url: link, title: '合成预览标题', author: '示例作者', status: 'ready', image: null })); return;
  }
  if (url.pathname === '/fixture-pwa') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end('<!doctype html><meta name="viewport" content="width=device-width, initial-scale=1"><body style="background:#202020;color:#eee;font-family:sans-serif"><button id="open">打开测试记录</button></body>'); return;
  }
  const upstream = http.request({ hostname: '127.0.0.1', port: +(process.env.RECORD_VITE_PORT || 5199), path: req.url, method: req.method, headers: req.headers }, r => { res.writeHead(r.statusCode, r.headers); r.pipe(res); });
  upstream.on('error', () => { res.writeHead(502); res.end('Vite is not running'); }); req.pipe(upstream);
});
(async () => {
  fs.mkdirSync(output, { recursive: true });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'chrome' });
  try {
    // Generate independently of application layout; a clipped canvas can produce no frames.
    const source = await browser.newPage();
    await source.setContent('<canvas width="160" height="90"></canvas>');
    video = Buffer.from(await source.evaluate(async () => {
      const canvas = document.querySelector('canvas'), c = canvas.getContext('2d');
      const stream = canvas.captureStream(0), track = stream.getVideoTracks()[0];
      const recorder = new MediaRecorder(stream, { mimeType: 'video/webm;codecs=vp8' }), chunks = [];
      recorder.ondataavailable = e => { if (e.data.size) chunks.push(e.data); };
      const finished = new Promise(resolve => { recorder.onstop = resolve; });
      try {
        recorder.start(100);
        for (let i = 0; i < 80 && (i < 10 || !chunks.length); i++) {
          c.fillStyle = i % 2 ? '#308966' : '#b3d5c4'; c.fillRect(0, 0, 160, 90);
          await new Promise(requestAnimationFrame); track.requestFrame();
          await new Promise(r => setTimeout(r, 100));
        }
        recorder.stop(); await finished;
        return Array.from(new Uint8Array(await new Blob(chunks).arrayBuffer()));
      } finally { stream.getTracks().forEach(t => t.stop()); }
    }));
    await source.close();
    assert.ok(video.length > 100, 'Synthetic WebM must contain encoded frames before reader tests');
    fs.writeFileSync(path.join(output, 'record-preview.webm'), video);
    for (const kind of ['pc', 'pwa']) for (const width of [1280, 390]) {
      counts = {}; revoked = false; sent = [];
      const context = await browser.newContext({ viewport: { width, height: 900 } });
      const page = await context.newPage(); const errors = [];
      page.on('pageerror', e => errors.push(e.message));
      await page.goto(base + (kind === 'pc' ? '/pc/tests/fixtures/chat-records.html' : '/fixture-pwa'));
      if (kind === 'pwa') {
        for (const name of ['social_links.css', 'chat_records.css']) await page.addStyleTag({ content: fs.readFileSync(path.join(root, 'server/src/assets', name), 'utf8') });
        for (const name of ['social_links.js', 'social_link_viewer.js', 'chat_record_presentation.js', 'chat_record_actions.js', 'chat_record_video.js', 'chat_record_media.js', 'chat_records.js']) await page.addScriptTag({ content: fs.readFileSync(path.join(root, 'server/src/assets', name), 'utf8') });
        await page.evaluate(c => { document.querySelector('#open').onclick = () => ElonChatRecords.open(c, { owner: 'reader', current: () => true, api: (p, init) => fetch(p, { ...init, headers: { Authorization: 'Bearer synthetic-session' } }) }); }, card);
      }
      await page.getByRole('button', { name: '打开测试记录', exact: true }).click();
      const dialog = page.getByRole('dialog', { name: '聊天记录', exact: true });
      await dialog.locator('article').first().waitFor();
      assert.equal(await dialog.locator('.social-link-card').count(), 2);
      const channels = dialog.locator('.social-link-channels'); await channels.scrollIntoViewIfNeeded();
      await page.waitForFunction(() => document.querySelector('.social-link-channels .social-link-source')?.textContent === '示例作者');
      assert.equal(await channels.locator('.social-link-play').isVisible(), true);
      assert.equal(await dialog.getByText('[链接]', { exact: true }).count(), 0);
      assert.equal(await dialog.locator('p').filter({ hasText: 'https://mp.weixin.qq.com' }).count(), 0);
      await channels.click({ button: 'right' });
      await dialog.getByRole('button', { name: '转发到群聊', exact: true }).click();
      const forward = page.getByRole('dialog', { name: '转发到群聊', exact: true });
      await forward.getByLabel('转发目标').selectOption('target'); assert.equal(sent.length, 0);
      await forward.getByRole('button', { name: '确认转发', exact: true }).click();
      await forward.getByText('已发送到群聊', { exact: true }).waitFor();
      assert.deepEqual(sent, [{ content: 'https://weixin.qq.com/sph/fixture' }]);
      await forward.getByRole('button', { name: '完成', exact: true }).click();
      await page.evaluate(() => {
        Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined });
        document.execCommand = command => {
          if (command !== 'copy') return false;
          window.copiedRecordLink = document.activeElement.value;
          return true;
        };
      });
      await channels.locator('..').locator('.chat-record-link-actions > button').filter({ hasText: '复制链接' }).click();
      assert.equal(await page.evaluate(() => window.copiedRecordLink), 'https://weixin.qq.com/sph/fixture');
      if (kind === 'pc' && width > 520) {
        const before = await dialog.boundingBox(); const title = dialog.locator('h2'); const p = await title.boundingBox();
        await page.mouse.move(p.x + 40, p.y + 8); await page.mouse.down(); await page.mouse.move(p.x + 150, p.y + 50, { steps: 5 }); await page.mouse.up();
        const after = await dialog.boundingBox(); assert.ok(after.x > before.x + 80); assert.ok(after.y >= 0);
        await title.dblclick(); assert.ok(Math.abs((await dialog.boundingBox()).x - before.x) < 2);
      }
      async function media() {
        await dialog.locator('.chat-record-asset').first().scrollIntoViewIfNeeded();
        await page.waitForFunction(() => [...document.querySelectorAll('article img')].some(i => i.naturalWidth === 1));
        await dialog.locator('.chat-record-video-frame').scrollIntoViewIfNeeded();
        await page.waitForFunction(() => document.querySelector('.chat-record-video-frame img')?.naturalWidth > 1);
        assert.equal(await dialog.locator('video').count(), 0, 'No autoplaying video before explicit play');
        await page.screenshot({ path: path.join(output, `${kind}-${width}-poster.png`) });
        await dialog.getByRole('button', { name: '播放视频', exact: true }).click();
        try { await page.waitForFunction(() => document.querySelector('article video')?.readyState >= 1); }
        catch (e) { console.log('VIDEO_DIAGNOSTIC', { bytes: video.length, counts, state: await page.evaluate(() => { const v = document.querySelector('article video'); return { present: !!v, ready: v?.readyState, error: v?.error?.code, network: v?.networkState }; }) }); throw e; }
      }
      await media();
      await dialog.getByRole('button', { name: '聊天记录 · 1 条', exact: true }).click();
      await dialog.getByText('嵌套的聊天消息', { exact: true }).waitFor();
      await dialog.getByRole('button', { name: '返回', exact: true }).click();
      await media();
      await dialog.getByRole('button', { name: '关闭', exact: true }).click();
      await page.getByRole('button', { name: '打开测试记录', exact: true }).click(); await media();
      assert.equal(counts['record_test:200'], 1); assert.equal(counts['image_test:200'], 1); assert.equal(counts['video_test:200'], 1);
      assert.ok(counts['record_test:304'] >= 1); assert.ok(counts['image_test:304'] >= 1); assert.ok(counts['video_test:304'] >= 1);
      assert.equal(await dialog.evaluate(n => n.scrollWidth <= n.clientWidth + 1), true);
      await channels.scrollIntoViewIfNeeded(); await page.screenshot({ path: path.join(output, `${kind}-${width}.png`) });
      await dialog.getByRole('button', { name: '关闭', exact: true }).click(); revoked = true;
      await page.getByRole('button', { name: '打开测试记录', exact: true }).click();
      await dialog.getByText('记录已撤回，或你已不在此群聊中').waitFor();
      assert.equal(await dialog.locator('article').count(), 0); assert.deepEqual(errors, []);
      // Preserve an explicit play click while the visible attachment is still downloading.
      await page.evaluate(async bytes => {
        const host = document.createElement('div'); document.body.append(host);
        let resolveBlob;
        const pending = new Promise(resolve => { resolveBlob = resolve; });
        const dispose = ElonRecordMedia.mount(host, { kind: 'video', filename: 'pending.webm' }, { current: () => true, load: () => pending, scope: 'pending' });
        host.querySelector('button').click(); host.querySelector('button').click();
        resolveBlob(new Blob([new Uint8Array(bytes)], { type: 'video/webm' }));
        await new Promise(resolve => setTimeout(resolve, 50));
        const played = !!host.querySelector('video'); dispose(); host.remove();
        if (!played) throw Error('Play click was lost during download');
      }, [...video]);
      console.log(`RECORD_CACHE_UI=passed ${kind} ${width} real-http-304/image/video/nesting/reopen/revoke/cards/drag`);
      await context.close();
    }
  } finally { await browser.close(); await new Promise(resolve => server.close(resolve)); }
})().catch(e => { console.error(e); server.close(); process.exitCode = 1; });
