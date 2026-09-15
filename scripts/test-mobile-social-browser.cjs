// Full production mobile template, isolated HTTP fixtures, no production accounts or writes.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const assert = require('node:assert/strict');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jVHsAAAAASUVORK5CYII=';
const html = fs.readFileSync(path.join(assets, 'web_page.html'), 'utf8').replace(/__[A-Z0-9_]+_PNG_B64__/g, png).replace(/__UI_TUNER_[A-Z0-9_]+__/g, '');
const group = { id: 'g', name: '回归测试群', member_count: 2, members: [{ id: 'other', display_name: '测试成员', avatar_data_url: 'data:image/png;base64,' + png }] };
let rows = [{ id: 'one', sender_user_id: 'other', sender_name: '测试成员', content: '已缓存的第一条消息', created_at: '2026-09-15T08:00:00Z', revision: 1,
  attachments: [{ kind: 'image', url: '/fixture.png', file_name: '测试图片.png' }, { kind: 'audio', url: '/fixture.wav', file_name: '测试语音.wav' }] }];
let unavailable = false, status = 200, reads = 0, writes = 0;
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  const json = (value, code = 200) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(JSON.stringify(value)); };
  if (url.pathname === '/seed') { res.end('<html></html>'); return; }
  if (url.pathname === '/') { res.writeHead(200, { 'content-type': 'text/html' }); res.end(html); return; }
  if (url.pathname === '/sw.js') { res.writeHead(200, { 'content-type': 'application/javascript', 'cache-control': 'no-cache' }); res.end(fs.readFileSync(path.join(assets, 'mobile_shell_worker.js'))); return; }
  if (/^\/assets\/[\w.-]+\.(js|css)$/.test(url.pathname)) {
    const file = path.join(assets, path.basename(url.pathname));
    if (fs.existsSync(file)) { res.writeHead(200, { 'content-type': file.endsWith('.css') ? 'text/css' : 'application/javascript' }); res.end(fs.readFileSync(file)); return; }
  }
  if (url.pathname === '/fixture.png') { res.writeHead(200, { 'content-type': 'image/png' }); res.end(Buffer.from(png, 'base64')); return; }
  if (url.pathname.startsWith('/api/')) {
    if (unavailable) { json({ error: 'fixture offline' }, 503); return; }
    if (url.pathname === '/api/me') { json({ user: { id: 'a', account: 'fixture', nickname: '测试账号' } }); return; }
    if (url.pathname === '/api/me/friends') { json({ friends: [] }); return; }
    if (url.pathname === '/api/me/groups') { json({ groups: [group] }); return; }
    if (url.pathname === '/api/me/groups/g/messages') {
      if (req.method === 'POST') { writes++; json({ error: 'fixture intentionally rejects writes' }, 503); return; }
      reads++; json({ messages: rows }, status); return;
    }
    if (url.pathname.endsWith('/revisions')) { json({ revisions: [{ revision: 1, content: '旧版本', created_at: '2026-09-15T08:00:00Z' }, { revision: 2, content: rows[0].content }] }); return; }
    json({ projects: [], agents: [], posts: [], articles: [], members: group.members, items: [], groups: [], conversations: [] }); return;
  }
  res.writeHead(404); res.end();
});
async function main() {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const origin = 'http://127.0.0.1:' + server.address().port;
  const browser = await chromium.launch({ channel: process.env.BROWSER_CHANNEL || 'msedge', headless: true });
  const context = await browser.newContext({ viewport: { width: 412, height: 915 }, deviceScaleFactor: 1, isMobile: true, hasTouch: true });
  const page = await context.newPage(), errors = [];
  page.on('pageerror', error => errors.push(error.stack || error.message));
  await context.route(/https:\/\/.*/, route => route.abort());
  try {
    await page.goto(origin + '/seed'); await page.evaluate(() => localStorage.setItem('lodex_token', 'synthetic-browser-test'));
    await page.goto(origin, { waitUntil: 'domcontentloaded' });
    await page.locator('.conversation-item').filter({ hasText: group.name }).click();
    await page.locator('#chatList').getByText('已缓存的第一条消息').waitFor();
    assert.equal(await page.locator('#chatList audio').count(), 1);
    await page.evaluate(() => { window.audioBefore = document.querySelector('#chatList audio'); window.messageBefore = document.querySelector('.chat-message-block'); });
    rows.push({ id: 'two', sender_user_id: 'other', sender_name: '测试成员', content: '恢复后收到的新消息', created_at: '2026-09-15T08:01:00Z', revision: 1 });
    await page.evaluate(() => window.dispatchEvent(new Event('online')));
    await page.getByText('恢复后收到的新消息', { exact: true }).waitFor();
    assert.equal(await page.evaluate(() => audioBefore === document.querySelector('#chatList audio') && messageBefore === document.querySelector('.chat-message-block')), true);
    // A held background view must resume without restarting the application.
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' }); document.dispatchEvent(new Event('visibilitychange')); });
    const before = reads; await page.waitForTimeout(3200); assert.equal(reads, before);
    rows.push({ id: 'three', sender_user_id: 'other', content: '后台恢复已自动同步', created_at: '2026-09-15T08:02:00Z' });
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' }); document.dispatchEvent(new Event('visibilitychange')); });
    await page.getByText('后台恢复已自动同步', { exact: true }).waitFor();
    rows[0] = { ...rows[0], revision: 2, edited_at: '2026-09-15T08:03:00Z', content: '修改后保留历史的消息' };
    await page.evaluate(() => window.dispatchEvent(new Event('online')));
    await page.locator('#chatList').getByText('修改后保留历史的消息').waitFor();
    assert.ok(await page.getByText(/已编辑/).count() > 0);
    // Public shell and static dependencies must be available after a genuine offline reload.
    await page.waitForFunction(async () => !!navigator.serviceWorker.controller && !!(await (await caches.open('elon-mobile-shell-v1')).match('/')));
    await context.setOffline(true); await page.reload({ waitUntil: 'domcontentloaded' });
    await page.locator('.conversation-item').filter({ hasText: group.name }).click({ timeout: 4000 });
    await page.locator('#chatList').getByText('修改后保留历史的消息').waitFor({ timeout: 4000 });
    const cachedApi = await page.evaluate(async () => (await (await caches.open('elon-mobile-shell-v1')).keys()).some(request => new URL(request.url).pathname.startsWith('/api/')));
    assert.equal(cachedApi, false);
    await context.setOffline(false); await page.evaluate(() => window.dispatchEvent(new Event('online')));
    status = 403; await page.evaluate(() => window.dispatchEvent(new Event('online')));
    await page.getByText('无法访问此会话，请检查账号或成员权限', { exact: true }).waitFor();
    assert.equal(await page.locator('#chatList .chat-message-block').count(), 0);
    assert.equal(writes, 0); assert.deepEqual(errors, []);
    const output = process.env.ELON_SOCIAL_EVIDENCE_DIR || path.join(root, '.ai-tmp/mobile-social'); fs.mkdirSync(output, { recursive: true });
    status = 200; await page.evaluate(() => window.dispatchEvent(new Event('online')));
    await page.locator('#chatList').getByText('修改后保留历史的消息').waitFor();
    await page.screenshot({ path: path.join(output, 'pwa-chat-recovery.png'), fullPage: true });
    page.once('dialog', dialog => dialog.accept());
    await page.evaluate(() => document.getElementById('logoutRow').click());
    assert.equal(await page.evaluate(() => Object.keys(localStorage).some(key => key.startsWith('elon-social-cache-v1:'))), false);
    assert.equal(await page.locator('#chatList .chat-message-block').count(), 0);
    fs.writeFileSync(path.join(output, 'browser-result.json'), JSON.stringify({ passed: true, reads, writes, errors, scenarios: ['real-template', 'media-node-preservation', 'foreground-resume', 'edited-history-marker', 'offline-shell-reload', 'account-bound-cache', 'permission-removal', 'logout-clears-cache'] }, null, 2));
    console.log('PASS mobile browser: cache-first offline reload, foreground sync, media node identity, edited marker, permission invalidation; no production writes');
  } catch (error) {
    console.error(JSON.stringify({ reads, writes, errors, chat: await page.locator('#chatList').innerText().catch(() => ''), status: await page.locator('#social-sync-status').innerText().catch(() => '') }));
    throw error;
  } finally { await browser.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}
main().catch(error => { console.error(error); server.closeAllConnections(); server.close(); process.exitCode = 1; });
