// Production UI with loopback-only synthetic messages; no production account or writes.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'webkit', baseline = process.env.DENSITY_BASELINE === '1';
const output = path.resolve('.ai-tmp/chat-density'); fs.mkdirSync(output, { recursive: true });
const messages = ['公众号链接要怎么分享过来？', '安卓可以直接转发，也可以复制链接。', '或者把图片发到群里。', '好的，我试一下。', '文字和图片都能查看，语音也可以播放。', '这条已经补充过内容。', '阅读时可以保留原来的位置。', '收到，稍后一起检查。'].map((content, i) => ({
  id: 'density-' + i, content, sender_user_id: i % 3 === 0 ? 'mobile-v2-fixture' : 'density-member',
  sender_name: i % 3 === 0 ? '演示用户' : '群成员', outgoing: i % 3 === 0, revision: i === 5 ? 2 : 1,
  created_at: `2026-09-30T05:${i === 0 ? '42' : '47'}:${String(i).padStart(2, '0')}Z`, attachments: []
}));
const fixture = createFixture({ handleSyntheticRequest(req, res, url) {
  const json = value => { res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' }); res.end(JSON.stringify(value)); };
  if (url.pathname === '/api/me/groups') { json({ groups: [{ id: 'density-group', name: '阅读体验验证群', member_count: 2 }] }); return true; }
  if (url.pathname.startsWith('/api/me/groups/density-group')) {
    const id = url.pathname.match(/\/messages\/(density-\d+)/)?.[1], message = messages.find(m => m.id === id);
    if (req.method === 'PATCH' && message) {
      let body = ''; req.on('data', chunk => { body += chunk; }); req.on('end', () => {
        const change = JSON.parse(body); assert.equal(change.expected_revision, message.revision);
        message.content = change.content; message.revision++; json({ message });
      }); return true;
    }
    if (url.pathname.endsWith('/revisions') && message) { json({ revisions: [{ revision: message.revision, content: message.content, created_at: message.created_at }, { revision: 1, content: '原来的文字', created_at: message.created_at }], next_before_revision: null }); return true; }
    json({ messages, members: [], posts: [], items: [], ai_members: [] }); return true;
  }
  return false;
} });
async function main() {
  const origin = await fixture.listen();
  if (process.env.DENSITY_PREVIEW === '1') { console.log(JSON.stringify({ origin, synthetic: true })); return; }
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
    await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    const page = await context.newPage(), errors = [], cases = [], metrics = []; page.setDefaultTimeout(8000); page.on('pageerror', e => errors.push(e.message));
    await page.goto(origin + '/?fixture=login'); await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').tap(); await page.locator('#appView:not(.hidden)').waitFor();
    async function enter() {
      await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
      await page.locator('.conversation-item').filter({ hasText: '阅读体验验证群' }).tap(); await page.locator('[data-message-id="density-7"]').waitFor();
      await page.locator('#chatList').evaluate(el => { el.scrollTop = 0; });
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 390, 430]) {
        await page.setViewportSize({ width, height: 844 }); await enter();
        const result = await page.evaluate(() => {
          const list = document.querySelector('#chatList'), bounds = list.getBoundingClientRect();
          const blocks = [...list.querySelectorAll('.chat-message-block')].map(el => el.getBoundingClientRect());
          const targets = [...list.querySelectorAll('.social-message-more')].map(el => el.getBoundingClientRect());
          return { visible: blocks.filter(b => b.top >= bounds.top && b.bottom <= bounds.bottom).length,
            contentHeight: blocks.at(-1).bottom - blocks[0].top, rowHeights: blocks.map(b => b.height), viewport: bounds.height,
            font: getComputedStyle(list.querySelector('.bubble')).fontSize,
            overflow: document.documentElement.scrollWidth > innerWidth || list.scrollWidth > list.clientWidth,
            targetsValid: targets.every(b => b.width >= 48 && b.height >= 48 && b.left >= 0 && b.right <= innerWidth) };
        });
        metrics.push({ theme, width, ...result }); assert(!result.overflow); assert(result.targetsValid); assert.equal(result.font, '16px');
        if (!baseline) {
          assert(await page.locator('.group-revision-actions').first().isHidden());
          const header = page.locator('[data-message-id="density-5"] .chat-sender-name'); assert.match(await header.innerText(), /已编辑/);
          const beforeFile = path.join(output, `${engine}-before.json`);
          if (fs.existsSync(beforeFile)) {
            const before = JSON.parse(fs.readFileSync(beforeFile)).metrics.find(m => m.theme === theme && m.width === width);
            assert(result.contentHeight < before.contentHeight * .75, 'reclaim at least 25% transcript space without reducing font or targets');
            assert(result.visible > before.visible, 'more complete messages fit in the same viewport');
          }
        }
        if (width === 390) await page.screenshot({ path: path.join(output, `${engine}-${baseline ? 'before' : 'after'}-${theme}.png`) });
        cases.push(`${theme}-${width}-density-readable-targets`);
      }
    }
    if (!baseline) {
      const row = id => page.locator(`[data-message-id="density-${id}"]`), menu = () => page.getByRole('dialog', { name: '消息操作', exact: true });
      async function open(id) { await row(id).locator('.social-message-more').tap(); await menu().waitFor(); }
      for (const fontScale of [1.5, 2]) {
        await page.setViewportSize({ width: 320, height: 844 }); await enter();
        const style = await page.addStyleTag({ content: `.bubble{font-size:${16 * fontScale}px!important}.chat-sender-name{font-size:${12 * fontScale}px!important}` });
        assert(await page.locator('#chatList').evaluate(el => el.scrollWidth <= el.clientWidth));
        await open(1); await menu().getByRole('button', { name: '关闭', exact: true }).tap(); await style.evaluate(el => el.remove());
        cases.push('large-text-' + fontScale);
      }
      await open(1); assert.equal(await menu().getByRole('button', { name: '编辑', exact: true }).count(), 0); await page.keyboard.press('Escape');
      await open(0); await menu().getByRole('button', { name: '编辑', exact: true }).tap();
      const edit = page.getByRole('dialog', { name: '编辑消息', exact: true }); await edit.waitFor();
      await edit.getByLabel('消息文字', { exact: true }).fill('已通过菜单修改这条消息'); await edit.getByRole('button', { name: '保存修改' }).tap();
      await edit.waitFor({ state: 'detached' });
      await page.waitForFunction(() => document.querySelector('[data-message-id="density-0"] .bubble')?.innerText.trim() === '已通过菜单修改这条消息');
      assert.equal(messages[0].revision, 2); cases.push('edit-from-menu-preserves-revision-workflow');
      await open(5); await menu().getByRole('button', { name: '查看修改记录' }).tap();
      const history = page.getByRole('dialog', { name: '修改记录', exact: true }); await history.locator('article pre').filter({ hasText: '原来的文字' }).waitFor();
      await history.getByRole('button', { name: '关闭', exact: true }).tap(); cases.push('revision-history-stays-accessible');
      await page.setViewportSize({ width: 390, height: 844 }); await enter(); await page.setViewportSize({ width: 390, height: 440 });
      await open(2); await menu().getByRole('button', { name: '引用', exact: true }).tap();
      assert(await page.locator('.social-quote-compose').isVisible()); assert(await page.locator('#messageInput').evaluate(el => document.activeElement === el));
      await page.getByRole('button', { name: '取消引用', exact: true }).tap(); cases.push('reduced-viewport-quote-and-composer');
    }
    assert.deepEqual(errors, []); assert.equal(fixture.audit.rejectedWrites, 0);
    const receipt = { status: 'passed', engine, baseline, cases, metrics, productionWrites: false, physicalIphoneVerified: false };
    fs.writeFileSync(path.join(output, `${engine}-${baseline ? 'before' : 'after'}.json`), JSON.stringify(receipt, null, 2));
    console.log(JSON.stringify(receipt));
  } finally { await browser.close(); await fixture.close(); }
}
main().catch(e => { console.error(e); process.exitCode = 1; void fixture.close(); });
