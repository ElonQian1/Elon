import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium, webkit } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const safari = process.argv.includes('--webkit');
const browser = await (safari ? webkit.launch({ headless: true }) : chromium.launch({ channel: 'msedge', headless: true }));
try {
  const context = await browser.newContext({ userAgent: safari ? 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1' : 'Mozilla/5.0 (Linux; Android 15) Chrome/140.0.0.0 Mobile Safari/537.36', viewport: { width: 360, height: 800 }, isMobile: true, hasTouch: true });
  const page = await context.newPage(); let extraPages = 0; context.on('page', () => extraPages++);
  await page.setContent('<style>#messages{height:150px;overflow:auto}#messages div{height:1200px}</style><div id="messages"><div>Selected group</div></div><textarea id="draft">Unsent draft</textarea><div id="card"></div>');
  for (const name of ['social_links', 'social_wechat_handoff']) await page.addScriptTag({ content: await readFile(new URL(`../server/src/assets/${name}.js`, import.meta.url), 'utf8') });
  await page.evaluate(() => {
    const url = 'https://weixin.qq.com/sph/Aur6t4pfk3';
    document.querySelector('#messages').scrollTop = 213;
    const host = document.querySelector('#card');
    const button = document.createElement('button'); button.textContent = 'Open video'; host.append(button);
    const handoff = ElonWechatHandoff.create(host, () => ({ url }), { isCurrent: () => true, api: async () => ({ schema: 1, source_url: url, expires_at_ms: Date.now() + 60000,
      launch_url: 'weixin://biz/finder/openFinderFeed/' + encodeURIComponent('exportId=export/fixture123&actionType=0') }) });
    button.onclick = handoff.open;
  });
  const original = page.url(); await page.getByRole('button', { name: 'Open video', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('[role="status"]').textContent === '已请求打开微信');
  assert.equal(page.url(), original); assert.equal(extraPages, 0);
  assert.equal(await page.locator('#draft').inputValue(), 'Unsent draft');
  assert.equal(await page.locator('#messages').evaluate(n => n.scrollTop), 213);
  console.log(`PASS: ${safari ? 'WebKit iPhone emulation' : 'Chromium Android emulation'}, real browser user gesture, no intermediate tab/navigation, draft and scroll unchanged; external WeChat/PWA task return is not simulated`);
} finally { await browser.close(); }
