// Real production components with isolated, synthetic API traffic only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const out = path.join(root, '.ai-tmp', 'social-quotes');
fs.mkdirSync(out, { recursive: true });
(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  try {
    for (const width of [1280, 390, 320]) for (const theme of ['dark', 'light']) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = [], sends = []; let fail = true;
      page.on('pageerror', e => errors.push(e.message));
      await page.addInitScript({ path: path.join(root, 'server/src/assets/social_links.js') });
      await page.addInitScript(theme => { document.addEventListener('DOMContentLoaded', () => {
        if (theme === 'light') for (const [key, value] of Object.entries({ '--bg-base': '#fff', '--surface': '#fff', '--surface-2': '#f4f4f4', '--bg-hover': '#eee', '--text': '#202124', '--text-soft': '#60646b', '--text-muted': '#646870', '--line': '#c8cbd0', '--line-soft': '#d8dade', '--accent': '#202124' })) document.documentElement.style.setProperty(key, value);
      }); }, theme);
      await page.route(/^https?:\/\/[^/]+\/api\//, async route => {
        const url = route.request().url();
        if (url.endsWith('/link-preview')) return route.fulfill({ json: { url: 'https://app.binance.com/uni-qr/cpos/fixture', title: '城市散步：用镜头记录身边的日常', site: '视频', image: '/fixture-cover.png' } });
        if (url.endsWith('/messages') && route.request().method() === 'POST') {
          const body = route.request().postDataJSON(); sends.push(body);
          return route.fulfill({ status: fail ? 409 : 200, json: fail ? { error: '原消息已修改，请重新选择引用' } : { message: { id: 'sent', sender_user_id: 'me', outgoing: true, content: body.content,
            quote: { message_id: 'source', sender_name: '示例群友', content: 'https://app.binance.com/uni-qr/cpos/fixture', revision: 1, attachments: [], unavailable: false } } } });
        }
        return route.fulfill({ json: {} });
      });
      await page.route('**/fixture-cover.png', route => route.fulfill({ contentType: 'image/png', body: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lN8AAAAASUVORK5CYII=', 'base64') }));
      await page.goto(`http://127.0.0.1:${process.env.QUOTE_VITE_PORT || 5198}/pc/tests/fixtures/social-quotes.html`);
      await page.locator('[data-message-id="source"]').waitFor({ timeout: 10000 }).catch(async error => {
        throw new Error(JSON.stringify({ errors, body: (await page.locator('body').innerText()).slice(0, 1200), cause: error.message }));
      });
      const draft = page.getByRole('textbox', { name: '发送消息到 示例群聊…' });
      const input = await draft.count() ? draft : page.getByPlaceholder('发送消息到 示例群聊…');
      await page.locator('[data-message-id="source"] [data-social-content]').click({ button: 'right' });
      await page.getByRole('menuitem', { name: '引用', exact: true }).click();
      await page.getByLabel('待发送引用', { exact: true }).waitFor();
      assert.equal(await input.inputValue(), '');
      await input.fill('引用后的新回复');
      await page.getByRole('button', { name: '取消引用', exact: true }).click();
      assert.equal(await input.inputValue(), '引用后的新回复');
      await page.locator('[data-message-id="source"] [data-social-content]').click({ button: 'right' });
      await page.getByRole('menuitem', { name: '引用', exact: true }).click();
      await page.getByRole('button', { name: '发送', exact: true }).click();
      await page.getByRole('alert').filter({ hasText: '原消息已修改' }).waitFor();
      assert.equal(sends[0].content, '引用后的新回复');
      assert.deepEqual(sends[0].quote_source, { message_id: 'source', revision: 1 });
      assert.equal(await input.inputValue(), '引用后的新回复');
      assert.equal(await page.getByLabel('待发送引用', { exact: true }).count(), 1);
      await page.screenshot({ path: path.join(out, `draft-${width}-${theme}.png`), fullPage: true });
      fail = false;
      await page.getByRole('button', { name: '发送', exact: true }).click();
      await page.locator('[data-message-id="sent"]').waitFor();
      assert.equal(await page.getByLabel('待发送引用', { exact: true }).count(), 0);
      const quote = page.locator('[data-message-id="sent"]').getByLabel('引用的消息', { exact: true });
      assert.equal(await quote.count(), 1);
      await quote.getByRole('button').click();
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1);
      assert.equal(overflow, false);
      await page.screenshot({ path: path.join(out, `sent-${width}-${theme}.png`), fullPage: true });
      assert.deepEqual(errors, []); await page.close();
    }
    console.log('PASS: quote/cancel/failure/retry/send/navigation at 1280/390/320, light/dark; synthetic traffic only');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
