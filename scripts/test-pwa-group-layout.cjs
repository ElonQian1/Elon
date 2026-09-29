// Production PWA layout with synthetic conversations; never logs into production.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require('playwright');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'webkit';
const baseline = process.env.PWA_LAYOUT_BASELINE === '1';
const output = path.resolve(process.env.PWA_LAYOUT_OUTPUT || '.ai-tmp/group-layout');

(async () => {
  fs.mkdirSync(output, { recursive: true });
  const fixture = createFixture(), origin = await fixture.listen();
  const browser = await playwright[engine].launch({ headless: true,
    ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  const errors = [], writes = [], receipts = [];
  let summaryMode = 'ready';
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
    await context.route('**/*', route => {
      const request = route.request(), url = new URL(request.url()), p = url.pathname;
      if (url.origin !== origin) return route.abort();
      const json = (body, status = 200) => route.fulfill({ json: body, status });
      if (p === '/api/me/groups') return json({ groups: [{ id: 'layout-group', name: '产品讨论群', member_count: 12 }] });
      if (p === '/api/me/friends') return json({ friends: [{ id: 'layout-friend', nickname: '布局测试好友' }] });
      if (p.startsWith('/api/me/groups/layout-group') || p.startsWith('/api/me/friends/layout-friend')) {
        if (request.method() !== 'GET') { writes.push(p); return json({}, 405); }
        if (p.endsWith('/summary-posts/summary-1')) return json({ post: { id: 'summary-1', title: '下午讨论', summary: '这是合成总结。', status: 'ready' }, sources: [] });
        if (p.endsWith('/summary-posts')) {
          if (summaryMode === 'error') return json({ error: '离线测试：总结暂时无法读取' }, 503);
          return json({ posts: summaryMode === 'empty' ? [] : [{ id: 'summary-1', title: '下午讨论',
            status: 'ready_with_fallback', source_message_count: 91, created_by_name: '演示成员', content: '这是合成总结。' }] });
        }
        return json({ messages: Array.from({ length: 24 }, (_, i) => ({ id: 'layout-' + i,
          content: i === 23 ? '可以了，希望给聊天内容多留一些空间。' : '这是离线布局演示消息 ' + (i + 1),
          sender_user_id: i % 3 ? 'demo-member' : 'mobile-v2-fixture', sender_name: '演示成员',
          outgoing: i % 3 === 0, created_at: new Date(1790640000000 + i * 60000).toISOString() })),
          items: [], articles: [], members: [], ai_members: [] });
      }
      return route.continue();
    });
    const page = await context.newPage(); page.setDefaultTimeout(8000);
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(origin + '/?fixture=login');
    await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only');
    await page.locator('#loginBtn').tap();
    await page.locator('#appView:not(.hidden)').waitFor();
    // Finish fixture reads before replacing the document; WebKit reports aborted
    // fetches during a full-page navigation as access-control page errors.
    await page.waitForLoadState('networkidle');
    async function openGroup() {
      await page.waitForLoadState('networkidle');
      await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
      await page.locator('.conversation-item').filter({ hasText: '产品讨论群' }).tap();
      await page.locator('#chatList').getByText('可以了，希望给聊天内容多留一些空间。', { exact: true }).waitFor();
      await page.locator('#groupSummaryAction').getByText(summaryMode === 'error' ? '重试' : summaryMode === 'empty' ? '生成' : '查看', { exact: true }).waitFor();
    }
    async function measure(label) {
      const metrics = await page.evaluate(() => {
        const rect = selector => { const r = document.querySelector(selector).getBoundingClientRect();
          return { top: r.top, bottom: r.bottom, height: r.height, width: r.width }; };
        const input = rect('#inputBar'), list = rect('#chatList'), toolbar = rect('.toolbar');
        return { input, list, toolbar, visibleMessages: Math.min(list.bottom, input.top) - list.top,
          horizontalOverflow: document.documentElement.scrollWidth > innerWidth,
          inputVisible: input.bottom <= (visualViewport?.height || innerHeight) + (visualViewport?.offsetTop || 0) + 1 };
      });
      receipts.push({ label, ...metrics }); return metrics;
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(theme => localStorage.setItem('elon.mobile.appearance.v2', theme), theme);
      for (const width of [320, 390, 430]) {
        await page.setViewportSize({ width, height: 844 }); await openGroup();
        const metrics = await measure(`${theme}-${width}`);
        if (!baseline) {
          assert(metrics.visibleMessages >= 650, 'message area must receive space reclaimed from chrome');
          assert(metrics.toolbar.height <= 52 && metrics.input.height <= 80);
          assert(!metrics.horizontalOverflow && metrics.inputVisible);
          assert.equal(await page.locator('#planBtn').isVisible(), false);
          assert.equal(await page.locator('#groupAssistantEntry').isVisible(), false);
          for (const selector of ['#moreBtn', '#imageEditBtn', '#emojiBtn', '#voiceBtn', '#groupSummaryStrip']) {
            const box = await page.locator(selector).boundingBox(); assert(box.height >= 48, selector + ' touch target');
          }
        }
        if (width === 390) await page.screenshot({ path: path.join(output, `${engine}-${baseline ? 'before' : 'after'}-${theme}.png`) });
      }
    }
    if (!baseline) {
      await page.setViewportSize({ width: 390, height: 844 }); await openGroup();
      await page.locator('#moreBtn').tap();
      const menu = page.getByRole('dialog', { name: '群聊工具', exact: true }); await menu.waitFor();
      await menu.getByRole('button', { name: '群 AI 助手', exact: true }).tap();
      await page.getByRole('dialog').filter({ hasText: '暂无关注事项' }).waitFor();
      await page.getByRole('button', { name: '关闭', exact: true }).last().tap();
      await page.locator('#moreBtn').tap(); await menu.getByRole('button', { name: '文章', exact: true }).tap();
      await page.locator('dialog[open]').waitFor();
      assert.equal(await page.locator('dialog[open]').evaluate(el => el.classList.contains('group-chat-tools')), false);
      await page.locator('dialog[open]').evaluate(el => el.close());
      await page.locator('#moreBtn').tap(); await menu.getByRole('button', { name: '总结帖', exact: true }).tap();
      await page.getByText('下午讨论', { exact: true }).last().waitFor();
      await page.locator('.group-summary-dialog [data-close]').tap(); await openGroup();
      await page.locator('#moreBtn').tap(); await page.keyboard.press('Escape');
      assert.equal(await page.locator('#moreBtn').evaluate(el => document.activeElement === el), true);
      assert.equal(await page.locator('#moreBtn').getAttribute('aria-expanded'), 'false');
      await page.locator('#inputPlaceholder').tap(); await page.locator('#messageInput').fill('保留草稿\n第二行内容');
      // Synthetic VisualViewport changes reproduce the geometry contract, not an iPhone keyboard.
      await page.evaluate(() => {
        Object.defineProperties(visualViewport, { height: { configurable: true, get: () => 420 },
          offsetTop: { configurable: true, get: () => 0 } });
        visualViewport.dispatchEvent(new Event('resize'));
      });
      await page.waitForFunction(() => document.querySelector('#appView').classList.contains('group-keyboard-open'));
      const keyboard = await measure('keyboard-420'); assert(keyboard.inputVisible && keyboard.visibleMessages >= 200);
      await page.waitForFunction(() => { const list = document.querySelector('#chatList'); return list.scrollHeight - list.scrollTop - list.clientHeight <= 24; });
      assert.equal(await page.locator('#groupSummaryStrip').isVisible(), false);
      assert.equal(await page.locator('#messageInput').inputValue(), '保留草稿\n第二行内容');
      await page.screenshot({ path: path.join(output, `${engine}-keyboard.png`), clip: { x: 0, y: 0, width: 390, height: 420 } });
      await page.evaluate(() => { Object.defineProperty(visualViewport, 'offsetTop', { configurable: true, get: () => 80 }); visualViewport.dispatchEvent(new Event('scroll')); });
      await page.waitForFunction(() => document.querySelector('#appView').getBoundingClientRect().top === 80);
      assert((await measure('keyboard-panned')).inputVisible);
      await page.evaluate(() => { delete visualViewport.height; delete visualViewport.offsetTop; visualViewport.dispatchEvent(new Event('resize')); });
      await page.waitForFunction(() => !document.querySelector('#appView').classList.contains('group-keyboard-open'));
      await page.locator('#messageInput').fill(''); await page.locator('#messageInput').blur();
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      await page.locator('#chatList').evaluate(el => { el.scrollTop = 100; });
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      await page.waitForFunction(() => document.querySelector('#chatList').scrollTop === 100);
      await page.locator('#moreBtn').tap(); await menu.getByRole('button', { name: '关闭', exact: true }).tap();
      assert.equal(await page.locator('#chatList').evaluate(el => el.scrollTop), 100, 'tools must preserve reading position');
      await page.locator('#chatList').evaluate(el => { el.scrollTop = el.scrollHeight; });
      // Safe areas are injected separately because desktop engines have zero device insets.
      await page.addStyleTag({ content: ':root{--top-safe-area:47px}.group-chat-active #inputBar{padding-bottom:34px}' });
      const safe = await measure('safe-area-47-34'); assert(safe.toolbar.height >= 95 && safe.inputVisible);
      await page.screenshot({ path: path.join(output, `${engine}-safe-area.png`) });
      for (const mode of ['empty', 'error']) { summaryMode = mode; await openGroup(); await page.locator('#moreBtn').tap(); await menu.waitFor(); await page.keyboard.press('Escape'); }
      summaryMode = 'ready'; await openGroup();
      await page.evaluate(() => { document.querySelector('#topTitle').textContent = '一个非常长的群聊标题用来检查窄屏显示与工具入口'; });
      await page.setViewportSize({ width: 320, height: 568 });
      // Explicit text-size stress avoids pretending desktop text-size-adjust emulates iOS settings.
      for (const scale of [1.5, 2]) {
        const style = await page.addStyleTag({ content: `#topTitle{font-size:${18 * scale}px!important}
          .summary-title{font-size:${14 * scale}px!important}.summary-action,.bubble,#inputPlaceholder,#messageInput{font-size:${16 * scale}px!important}
          .group-chat-tools{font-size:${16 * scale}px!important}` });
        const large = await measure(`text-${scale}-320`); assert(!large.horizontalOverflow && large.inputVisible);
        const title = await page.locator('#topTitle').boundingBox(); assert(title.height <= large.toolbar.height);
        await page.locator('#moreBtn').tap(); await menu.getByRole('button', { name: '关闭', exact: true }).tap();
        await page.screenshot({ path: path.join(output, `${engine}-text-${scale}.png`) });
        await page.locator('#inputPlaceholder').tap(); await page.locator('#messageInput').fill('大字体输入\n保留完整文字');
        assert((await measure(`text-${scale}-input`)).inputVisible);
        await page.locator('#messageInput').fill(''); await page.locator('#messageInput').blur();
        await style.evaluate(el => el.remove());
      }
      await page.setViewportSize({ width: 844, height: 390 });
      await page.waitForFunction(() => document.querySelector('#appView').getBoundingClientRect().height <= 390);
      const landscape = await measure('landscape'); assert(!landscape.horizontalOverflow && landscape.inputVisible);
      await page.setViewportSize({ width: 390, height: 844 });
      await page.locator('#backBtn').tap();
      assert.equal(await page.locator('#appView').evaluate(el => el.classList.contains('group-chat-active')), false);
      await page.locator('.conversation-item').filter({ hasText: '布局测试好友' }).tap();
      assert.equal(await page.locator('#planBtn').isVisible(), true, 'friend composer is outside this change');
      await page.locator('#moreBtn').tap(); await page.locator('#nativeOnlyMask.active').waitFor();
      assert.equal(await page.locator('.group-chat-tools').count(), 0);
      assert.deepEqual(writes, []); assert.deepEqual(errors, []);
    }
    fs.writeFileSync(path.join(output, `${engine}-${baseline ? 'before' : 'after'}.json`), JSON.stringify(receipts, null, 2));
    console.log(JSON.stringify({ engine, baseline, cases: receipts.length, receipts, productionWrites: false }));
  } finally { await browser.close(); await fixture.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
