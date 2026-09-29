// Use production PWA assets with synthetic contacts; no production login or writes.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require('playwright');
const { attachmentFixture } = require('./pwa-attachment-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'webkit';
const baseline = process.env.PWA_COMPOSER_BASELINE === '1';
const output = path.resolve(process.env.PWA_COMPOSER_OUTPUT || '.ai-tmp/inline-composer');

(async () => {
  fs.mkdirSync(output, { recursive: true });
  const browser = await playwright[engine].launch({ headless: true,
    ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const fixture = await attachmentFixture(context), { origin, state } = fixture;
  const page = await context.newPage(), errors = [], receipts = [];
  page.on('pageerror', error => errors.push(error.message)); page.setDefaultTimeout(8000);
  const input = page.locator('#messageInput');
  const settle = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  try {
    await page.goto(origin + '/?fixture=login');
    await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only');
    await page.locator('#loginBtn').tap(); await page.locator('#appView:not(.hidden)').waitFor();
    await page.waitForLoadState('networkidle');
    async function enter(kind = 'group') {
      await page.waitForLoadState('networkidle');
      await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
      await page.locator('.conversation-item').filter({ hasText: kind === 'group' ? '附件测试群' : '附件测试好友' }).tap();
      await page.locator('#inputBar.active').waitFor(); await page.waitForLoadState('networkidle'); await settle();
    }
    async function measure(label) {
      await settle();
      const metric = await page.evaluate(() => {
        const rect = selector => { const r = document.querySelector(selector).getBoundingClientRect();
          return { top: r.top, bottom: r.bottom, height: r.height, width: r.width, left: r.left, right: r.right }; };
        const bar = rect('#inputBar'), panel = rect('.input-panel'), editor = rect('#messageInput');
        const buttons = ['#imageEditBtn', '#emojiBtn', document.querySelector('#inputBar').classList.contains('has-text') ? '#sendBtn' : '#voiceBtn'].map(rect);
        return { bar, panel, editor, buttons, bottomPadding: parseFloat(getComputedStyle(document.querySelector('#inputBar')).paddingBottom),
          viewportBottom: visualViewport.height + visualViewport.offsetTop,
          layoutViewport: { width: innerWidth, height: innerHeight }, visualHeight: visualViewport.height,
          overflow: document.documentElement.scrollWidth > innerWidth,
          keyboard: document.querySelector('#appView').classList.contains('group-keyboard-open') };
      });
      receipts.push({ label, ...metric }); return metric;
    }
    function compact(m) {
      assert(m.panel.height <= 58, 'empty or single-line composer must not become two rows');
      for (const button of m.buttons) {
        assert(button.width >= 48 && button.height >= 48, 'keep usable touch targets');
        assert(Math.abs((button.top + button.bottom - m.editor.top - m.editor.bottom) / 2) <= 4, 'controls and input share one row');
      }
      assert(m.editor.width >= 130 && !m.overflow, 'narrow phones retain a usable editor');
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 390, 430]) {
        await page.setViewportSize({ width, height: 844 }); await enter();
        if (baseline) await page.locator('#inputPlaceholder').tap(); else await input.tap();
        const empty = await measure(`${theme}-${width}-focused-empty`);
        if (!baseline) compact(empty);
        await input.fill('你好'); const typed = await measure(`${theme}-${width}-single-line`);
        if (!baseline) compact(typed);
        if (width === 390) await page.screenshot({ path: path.join(output, `${engine}-${baseline ? 'before' : 'after'}-${theme}.png`) });
      }
    }
    if (!baseline) {
      await page.setViewportSize({ width: 390, height: 844 }); await enter(); await input.tap();
      await input.fill('第一行\n第二行'); const multi = await measure('two-lines');
      assert(multi.editor.height > 42 && multi.panel.height < 92, 'only actual text grows the editor');
      await input.fill('多行内容\n'.repeat(18)); const long = await measure('long-draft');
      assert(long.editor.height <= 140 && long.panel.height <= 150);
      assert.equal(await input.evaluate(el => getComputedStyle(el).overflowY), 'auto');
      await input.fill(''); compact(await measure('clear-shrinks'));
      await input.dispatchEvent('keydown', { key: 'Enter', code: 'Enter', isComposing: true });
      assert.equal(state.messages.length, 0, 'IME confirmation must not send');
      // Inset injection is separate from keyboard simulation: desktop engines have no home indicator.
      await page.addStyleTag({ content: '#appView{--top-safe-area:47px;--chat-safe-bottom:34px}' });
      const safe = await measure('keyboard-closed-insets'); assert.equal(safe.bottomPadding, 34);
      await page.evaluate(() => { window.fixtureInnerHeight = Object.getOwnPropertyDescriptor(window, 'innerHeight'); });
      for (const layoutAlsoShrinks of [false, true]) {
        // The keyboard animates through small deltas; a moving baseline can miss it entirely.
        for (const height of [800, 740, 680, 620, 540, 480, 420]) {
          await page.evaluate(({ shrink, height }) => {
            Object.defineProperties(visualViewport, { height: { configurable: true, get: () => height }, offsetTop: { configurable: true, get: () => 0 } });
            if (shrink) Object.defineProperty(window, 'innerHeight', { configurable: true, get: () => height });
            visualViewport.dispatchEvent(new Event('resize')); window.dispatchEvent(new Event('resize'));
          }, { shrink: layoutAlsoShrinks, height });
          await settle();
        }
        await page.waitForFunction(() => document.querySelector('#appView').classList.contains('group-keyboard-open'));
        const keyboard = await measure(layoutAlsoShrinks ? 'both-viewports-shrink' : 'visual-viewport-shrinks');
        compact(keyboard); assert.equal(keyboard.bottomPadding, 4);
        assert(Math.abs(keyboard.bar.bottom - keyboard.viewportBottom) <= 1, 'composer ends at the visible keyboard boundary');
        assert.equal(await page.locator('#groupSummaryStrip').isVisible(), false);
        await page.screenshot({ path: path.join(output, `${engine}-keyboard.png`), clip: { x: 0, y: 0, width: 390, height: 420 } });
        await page.evaluate(() => { Object.defineProperty(visualViewport, 'offsetTop', { configurable: true, get: () => 60 }); visualViewport.dispatchEvent(new Event('scroll')); });
        const panned = await measure('keyboard-panned'); assert(Math.abs(panned.bar.bottom - panned.viewportBottom) <= 1);
        await input.blur(); assert((await measure('blur-before-keyboard-closes')).keyboard, 'blur must not restore the home inset before the viewport recovers');
        await page.evaluate(() => {
          delete visualViewport.height; delete visualViewport.offsetTop;
          Object.defineProperty(window, 'innerHeight', window.fixtureInnerHeight);
          visualViewport.dispatchEvent(new Event('resize'));
        });
        await page.waitForFunction(() => !document.querySelector('#appView').classList.contains('group-keyboard-open'));
        assert.equal((await measure('keyboard-dismissed')).bottomPadding, 34); await input.tap();
      }
      // Pinch zoom is not a keyboard; preserve the existing viewport rather than chasing its height.
      await page.evaluate(() => { Object.defineProperties(visualViewport, { scale: { configurable: true, get: () => 2 }, height: { configurable: true, get: () => 420 } }); visualViewport.dispatchEvent(new Event('resize')); });
      assert.equal((await measure('pinch')).keyboard, false);
      await page.evaluate(() => { delete visualViewport.scale; delete visualViewport.height; visualViewport.dispatchEvent(new Event('resize')); });
      await settle();
      await page.setViewportSize({ width: 844, height: 390 }); await settle();
      const landscape = await measure('landscape');
      assert.equal(landscape.keyboard, false, 'rotation is not keyboard opening: ' + JSON.stringify(landscape));
      await page.setViewportSize({ width: 390, height: 844 }); await input.fill('附件面板保留草稿');
      await page.locator('#imageEditBtn').tap(); await page.locator('#inputAttachmentPanel').waitFor();
      await page.locator('#expandEditorBtn').tap(); await page.locator('#nativeOnlyMask.active').waitFor();
      await page.locator('#nativeOnlyMask').getByRole('button', { name: '知道了' }).tap();
      await input.tap(); assert.equal(await input.inputValue(), '附件面板保留草稿');
      await enter('friend'); await input.tap(); await input.fill('你好'); compact(await measure('friend-single-line'));
      assert.equal(await page.locator('#planBtn').isVisible(), true);
      assert.deepEqual(errors, []); assert.equal(state.messages.length, 0); assert.equal(state.uploads.length, 0);
    }
    fs.writeFileSync(path.join(output, `${engine}-${baseline ? 'before' : 'after'}.json`), JSON.stringify(receipts, null, 2));
    console.log(JSON.stringify({ status: 'passed', engine, baseline, cases: receipts.length, productionWrites: false, physicalIosKeyboardVerified: false }));
  } finally { await browser.close(); await fixture.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
