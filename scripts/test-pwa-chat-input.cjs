// Exercise production markup, styles and handlers; all messages stay in local fixtures.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');

async function main() {
  const fixture = createFixture();
  const origin = await fixture.listen();
  const engine = process.env.BROWSER_ENGINE || 'chromium';
  const errors = [], sent = [], cases = [], projectFrames = [];
  let browser, page, step = 'login';
  try {
    browser = await playwright[engine].launch({ headless: true,
      ...(engine === 'chromium' ? { channel: process.env.BROWSER_CHANNEL || 'msedge' } : {}) });
    const context = await browser.newContext({ viewport: { width: 390, height: 844 },
      isMobile: true, hasTouch: true, reducedMotion: 'reduce' });
    await context.route('**/*', async route => {
      const request = route.request(), url = new URL(request.url()), p = url.pathname;
      if (url.origin !== origin) return route.abort();
      const json = body => route.fulfill({ json: body });
      if (p === '/api/me/groups') return json({ groups: [{ id: 'input-group', name: '输入回归群', member_count: 2 }] });
      if (p === '/api/me/friends') return json({ friends: [{ id: 'input-friend', nickname: '输入回归好友', account: 'fixture-friend' }] });
      if (/^\/api\/me\/(groups|friends)\/input-/.test(p)) {
        if (request.method() === 'POST' && p.endsWith('/messages')) {
          const content = request.postDataJSON().content;
          const message = { id: 'sent-' + sent.length, content, outgoing: true,
            sender_user_id: 'mobile-v2-fixture', created_at: new Date().toISOString() };
          sent.push({ path: p, content, message });
          return json({ message });
        }
        return json({ messages: sent.filter(item => item.path === p).map(item => item.message),
          posts: [], items: [], members: [], ai_members: [] });
      }
      return route.continue();
    });
    page = await context.newPage();
    page.setDefaultTimeout(8000);
    page.on('pageerror', error => errors.push(error.message));
    page.on('websocket', socket => {
      if (new URL(socket.url()).pathname.startsWith('/ws/projects/')) {
        socket.on('framesent', frame => projectFrames.push(frame.payload));
      }
    });
    await page.goto(origin + '/?fixture=login');
    await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only');
    await page.locator('#loginBtn').tap();
    await page.locator('#appView:not(.hidden)').waitFor();
    const input = page.locator('#messageInput');
    const focused = () => input.evaluate(element => document.activeElement === element);
    async function openEmpty() {
      await input.tap();
      assert.equal(await focused(), true, 'a single tap on the empty composer must focus the real textarea');
      assert.equal(await input.isVisible(), true);
      assert.equal(await input.isEditable(), true);
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const kind of ['group', 'friend', 'project']) {
        step = `${theme}-${kind}`;
        await page.goto(origin + '/?fixture=' + (kind === 'project' ? 'chat_result&tab=projects' : 'login'));
        if (kind !== 'project') {
          await page.locator('.conversation-item').filter({ hasText: kind === 'group' ? '输入回归群' : '输入回归好友' }).tap();
        }
        await page.locator('#inputBar.active').waitFor();
        if (kind === 'project') await page.locator('#chatList .bubble.ai').filter({ hasText: '这是离线布局示例' }).waitFor();
        await openEmpty();
        await page.keyboard.insertText('中文输入测试');
        assert.equal(await input.inputValue(), '中文输入测试');
        await input.evaluate(element => element.blur());
        await input.tap();
        assert.equal(await focused(), true, 'a draft stays focusable after blur');
        assert.equal(await input.inputValue(), '中文输入测试');
        await input.fill('');
        await input.evaluate(element => element.blur());
        await openEmpty();
        await page.locator('#imageEditBtn').tap();
        await page.locator('#inputBar.attachments-open').waitFor();
        await openEmpty();
        assert.equal(await page.locator('#inputBar').evaluate(el => el.classList.contains('attachments-open')), false);
        await page.keyboard.insertText('附件后继续输入');
        if (kind !== 'project') {
          const before = sent.length;
          await page.locator('#sendBtn').tap();
          await page.waitForFunction(() => document.querySelector('#messageInput').value === '');
          await page.locator('#chatList .bubble.user').filter({ hasText: '附件后继续输入' }).last().waitFor();
          assert.equal(sent.length, before + 1, 'send only one synthetic message');
          assert.equal(sent.at(-1).path, `/api/me/${kind}s/input-${kind}/messages`);
          assert.equal(sent.at(-1).content, '附件后继续输入');
          await input.evaluate(element => element.blur());
          await openEmpty();
          await page.keyboard.insertText('发送后继续输入');
          assert.equal(await input.inputValue(), '发送后继续输入');
        }
        const out = path.resolve(__dirname, '../.ai-tmp/pwa-chat-input');
        fs.mkdirSync(out, { recursive: true });
        if (kind === 'group') await page.screenshot({ path: path.join(out, `${engine}-${theme}.png`) });
        // Let post-send directory refreshes settle before unloading the fixture page.
        await page.waitForLoadState('networkidle');
        cases.push(`${theme}-${kind}`);
      }
    }
    assert.deepEqual(errors, []);
    assert.deepEqual(projectFrames, [], 'never send a project task');
    console.log(JSON.stringify({ status: 'passed', engine, cases, syntheticMessages: sent.length,
      productionNetwork: false, physicalIosKeyboardVerified: false }));
  } catch (error) {
    console.error(JSON.stringify({ step, sent: sent.length, errors, state: page && await page.evaluate(() => ({
      active: document.activeElement?.id, bar: document.querySelector('#inputBar')?.className,
      draft: document.querySelector('#messageInput')?.value, chat: document.querySelector('#chatList')?.textContent,
    })) }));
    throw error;
  } finally { await browser?.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
