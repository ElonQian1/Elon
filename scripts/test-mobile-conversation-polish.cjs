const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { createConversationFixture } = require('./mobile-conversation-polish-fixture.cjs');

async function main() {
  const fixture = createConversationFixture(), origin = await fixture.listen();
  const engine = process.env.BROWSER_ENGINE || 'chromium';
  const browser = await playwright[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const errors = [], samples = [];
  await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
  const page = await context.newPage();
  page.setDefaultTimeout(8000);
  page.on('pageerror', error => errors.push(error.message));
  try {
    await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
    await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only');
    await page.locator('#loginBtn').click();
    await page.locator('#appView:not(.hidden)').waitFor({ state: 'visible' });
    await page.waitForLoadState('networkidle');
    await page.goto(origin + '/?fixture=empty&home=1', { waitUntil: 'networkidle' });
    await page.locator('.empty-tip').waitFor();
    assert.equal(await page.locator('.conversation-item').count(), 0, 'empty fixture must not show synthetic contacts');
    await page.goto(origin + '/?fixture=login&conversation=group&draft=1', { waitUntil: 'networkidle' });
    await page.locator('#inputBar.has-text #messageInput').waitFor();
    assert.equal(await page.locator('#messageInput').inputValue(), '尚未发送的群聊草稿\n检查输入区展开后的布局');
    for (const theme of ['light', 'dark']) for (const width of [320, 390, 411]) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      await page.setViewportSize({ width, height: 844 });
      await page.goto(origin + '/?fixture=login&home=1', { waitUntil: 'networkidle' });
      const badge = page.locator('.avatar-badge').first();
      await badge.waitFor();
      await page.waitForLoadState('networkidle');
      const contrasts = await page.evaluate(() => {
        const luminance = color => color.match(/[0-9.]+/g).slice(0, 3).map(value => Number(value) / 255)
          .map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
          .reduce((sum, value, index) => sum + value * [.2126, .7152, .0722][index], 0);
        return ['.avatar-badge', '#tabChatBadge', '.conversation-item .time'].map(selector => {
          const node = document.querySelector(selector), style = getComputedStyle(node);
          let background = style.backgroundColor, parent = node.parentElement;
          while (['transparent', 'rgba(0, 0, 0, 0)'].includes(background) && parent) {
            background = getComputedStyle(parent).backgroundColor; parent = parent.parentElement;
          }
          const a = luminance(style.color), b = luminance(background);
          return { selector, ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05) };
        });
      });
      contrasts.forEach(({ selector, ratio }) => assert.ok(ratio >= 4.5, `${selector} must be readable in both themes`));
      await page.goto(origin + '/?fixture=login&conversation=group', { waitUntil: 'networkidle' });
      await page.locator('#chatList .bubble.friend').last().waitFor();
      await page.locator('button[title="查看引用消息"]').waitFor();
      await page.locator('#inputPlaceholder').tap();
      const input = page.locator('#messageInput');
      await input.fill('检查输入体验');
      const state = await page.evaluate(() => {
        const rect = selector => { const r = document.querySelector(selector).getBoundingClientRect(); return { width: r.width, height: r.height, top: r.top, bottom: r.bottom }; };
        return { panel: rect('.input-panel'), avatar: rect('.chat-avatar'),
          input: rect('#messageInput'), outline: getComputedStyle(document.querySelector('#messageInput')).outlineWidth,
          iconFilter: getComputedStyle(document.querySelector('#imageEditBtn img')).filter,
          overflow: document.documentElement.scrollWidth > innerWidth,
          focused: document.activeElement.id };
      });
      samples.push({ theme, width, ...state });
      const out = path.resolve(__dirname, '../.ai-tmp/conversation-polish'); fs.mkdirSync(out, { recursive: true });
      await page.screenshot({ path: path.join(out, `${engine}-${theme}-${width}.png`) });
      assert.equal(state.focused, 'messageInput');
      assert.ok(state.panel.height <= 112, 'single-line focused composer should stay compact');
      assert.equal(state.outline, '0px', 'focus is conveyed by the composer boundary');
      assert.ok(Math.abs(state.avatar.width - state.avatar.height) < 1, 'non-square photos must remain square avatars');
      assert.notEqual(state.iconFilter, 'none', 'legacy icon images must follow the theme');
      assert.equal(state.overflow, false, 'no horizontal page overflow');
      await input.fill('多行草稿\n'.repeat(12));
      assert.ok((await input.boundingBox()).height <= 141, 'long drafts scroll within the editor');
      await page.locator('#imageEditBtn').tap();
      await page.locator('#inputBar.attachments-open').waitFor();
      await page.locator('#imageEditBtn').tap();
      assert.equal(await input.inputValue(), '多行草稿\n'.repeat(12), 'opening attachments preserves the draft');
      await page.setViewportSize({ width, height: 430 });
      await input.tap();
      const editor = await input.boundingBox(), send = await page.locator('#sendBtn').boundingBox();
      assert.ok(editor.y >= 0 && editor.y + editor.height <= 430, 'editor stays inside reduced viewport');
      assert.ok(send && send.y >= 0 && send.y + send.height <= 430, 'send remains inside reduced viewport');
    }
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ status: 'passed', engine, cases: samples.length, synthetic: true, physicalIosKeyboardVerified: false }));
  } finally {
    fs.mkdirSync('.ai-tmp/conversation-polish', { recursive: true });
    fs.writeFileSync(`.ai-tmp/conversation-polish/${engine}-measurements.json`, JSON.stringify(samples, null, 2));
    await browser.close(); await fixture.close();
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
