// Exercise the published self-contained document with no production account/network.
const fs = require('node:fs'), path = require('node:path'), http = require('node:http'), assert = require('node:assert/strict'), vm = require('node:vm');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const input = process.argv[2]; if (!input) throw Error('Pass the generated runtime template path');
const html = fs.readFileSync(input, 'utf8').replace(/__[A-Z0-9_]+_PNG_B64__/g, 'iVBORw0KGgo=').replace(/__UI_TUNER_[A-Z0-9_]+__/g, '');
for (const m of html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)) if (!m[1].includes('src=')) new vm.Script(m[2]);
assert(!/(?:src|href)="\/assets\/[\w.-]+\.(?:js|css)/.test(html), 'runtime must not wait for separate startup assets');
const user = { id: 'fixture', account: 'fixture', nickname: '测试账号' }, group = { id: 'g', name: '测试群聊', member_count: 2 };
let unhealthy = false, apiWrites = 0;
const server = http.createServer((req, res) => {
  const p = new URL(req.url, 'http://localhost').pathname;
  const json = (data, status = 200) => { res.writeHead(status, { 'content-type': 'application/json' }); res.end(JSON.stringify(data)); };
  if (['/', '/web'].includes(p)) { res.writeHead(unhealthy ? 503 : 200, { 'content-type': 'text/html; charset=utf-8' }); res.end(unhealthy ? 'temporary outage' : html); return; }
  if (p === '/sw.js') { res.writeHead(200, { 'content-type': 'application/javascript' }); res.end(fs.readFileSync(path.join(assets, 'mobile_shell_worker.js'))); return; }
  if (p === '/api/auth/login') { json({ user, token: 'synthetic-token' }); return; }
  if (req.method !== 'GET') { apiWrites++; json({ error: 'no writes allowed' }, 405); return; }
  if (p === '/api/me') { json({ user }); return; }
  if (p === '/api/me/groups') { json({ groups: [group] }); return; }
  if (p === '/api/me/groups/g/messages') { json({ messages: [{ id: 'm', sender_user_id: 'other', sender_name: '测试成员', content: '群聊可以打开', created_at: '2026-09-20T04:00:00Z' }] }); return; }
  if (p.startsWith('/api/')) { json({ friends: [], projects: [], agents: [], posts: [], articles: [], members: [], items: [], groups: [], conversations: [] }); return; }
  res.writeHead(404); res.end();
});
(async () => {
  await new Promise(r => server.listen(0, '127.0.0.1', r));
  const origin = 'http://127.0.0.1:' + server.address().port;
  const browser = await chromium.launch({ channel: process.env.BROWSER_CHANNEL || 'msedge', headless: true });
  const results = [];
  try {
    for (const denied of [false, true]) {
      const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
      await context.route('https://**', r => r.abort());
      // No separate JS/CSS endpoint works: the delivered generation must stand alone.
      await context.route('**/assets/**', r => r.abort());
      const page = await context.newPage(), errors = [];
      page.on('pageerror', e => errors.push(e.stack || e.message));
      if (denied) await page.addInitScript(() => Object.defineProperty(window, 'localStorage', { get() { throw new Error('Storage blocked'); } }));
      await page.goto(origin + '/?tab=chat&source=pwa', { waitUntil: 'domcontentloaded' });
      assert.equal(await page.locator('#loginView').isVisible(), true);
      assert.equal(await page.locator('#mobileStartup').count(), 0);
      await page.locator('#accountInput').fill('fixture'); await page.locator('#passwordInput').fill('fixture-only'); await page.locator('#loginBtn').click();
      await page.locator('.conversation-item').filter({ hasText: group.name }).click({ timeout: 6000 });
      await page.locator('#chatList').getByText('群聊可以打开').waitFor({ timeout: 6000 });
      if (!denied) {
        await page.evaluate(() => navigator.serviceWorker.ready);
        await page.reload({ waitUntil: 'domcontentloaded' });
        await page.locator('.conversation-item').filter({ hasText: group.name }).waitFor();
        unhealthy = true;
        await page.reload({ waitUntil: 'domcontentloaded' });
        await page.locator('.conversation-item').filter({ hasText: group.name }).waitFor();
        unhealthy = false;
        await context.setOffline(true);
        await page.goto(origin + '/web?tab=chat', { waitUntil: 'domcontentloaded' });
        await page.locator('.conversation-item').filter({ hasText: group.name }).click({ timeout: 6000 });
        await page.locator('#chatList').getByText('群聊可以打开').waitFor({ timeout: 6000 });
        assert.equal(await page.evaluate(async () => (await (await caches.open('elon-mobile-shell-v1')).keys()).some(r => r.url.includes('/api/'))), false);
        await context.setOffline(false);
      }
      page.once('dialog', d => d.accept()); await page.evaluate(() => document.getElementById('logoutRow').click());
      await page.locator('#loginView').waitFor();
      assert.equal(await page.evaluate(() => ElonMobileStartup.storage.getItem('lodex_token')), null);
      if (!denied) {
        await page.evaluate(() => caches.delete('elon-mobile-shell-v1'));
        await context.setOffline(true);
        await page.reload({ waitUntil: 'domcontentloaded' });
        await page.getByRole('link', { name: '重新打开聊天' }).waitFor();
        await context.setOffline(false);
      }
      results.push({ storageDenied: denied, errors, passed: true });
      assert.deepEqual(errors, []);
      await context.close();
    }
    // A boot exception must display an actionable panel, independent of main startup.
    const context = await browser.newContext(); const page = await context.newPage();
    await page.route('**/*', r => r.fulfill({ contentType: 'text/html; charset=utf-8', body: '<body><script>' + fs.readFileSync(path.join(assets, 'mobile_startup.js'), 'utf8') + '</script><script>throw Error("fixture boot failure")</script></body>' }));
    await page.goto(origin); await page.locator('#mobileStartupRetry').waitFor();
    assert.match(await page.locator('#mobileStartupText').innerText(), /暂时未能打开/);
    await context.close(); assert.equal(apiWrites, 0);
    const dir = process.env.ELON_PWA_EVIDENCE_DIR || path.join(root, '.ai-tmp/pwa-startup'); fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, 'browser-result.json'), JSON.stringify({ results, recoveryPanel: true, apiWrites, iPhoneDeviceVerified: false }, null, 2));
    console.log('PWA_HOME_STARTUP=passed login,group,blocked-assets,storage-denied,503,offline,reload,logout,recovery-panel');
  } finally { await browser.close(); server.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; server.close(); });
