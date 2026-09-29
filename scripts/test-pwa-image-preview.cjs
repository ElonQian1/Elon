// Local synthetic images and messages. No production account, image or writes.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'webkit';
let failed = true;
function drawing(tall) {
  const height = tall ? 2800 : 750;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1000" height="${height}" viewBox="0 0 1000 ${height}">
  <rect width="1000" height="${height}" fill="#e9eef5"/>
  <path d="M0 0L1000 ${height}M1000 0L0 ${height}" stroke="#788699" stroke-width="3"/>
  <rect x="80" y="80" width="840" height="180" rx="20" fill="#2b5791"/>
  <text x="120" y="180" fill="white" font-family="sans-serif" font-size="58">IMAGE PREVIEW</text>
  <text x="120" y="340" fill="#202733" font-family="sans-serif" font-size="32">Pinch / double tap / drag</text>
  <circle cx="500" cy="${height / 2}" r="120" fill="#006c45"/>
  <text x="140" y="${height - 90}" fill="#202733" font-family="sans-serif" font-size="42">Original image · 1000 × ${height}</text></svg>`;
}
const fixture = createFixture({ handleSyntheticRequest(req, res, url) {
  const p = url.pathname, json = value => { res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(value)); };
  if (p === '/api/me/groups') { json({ groups: [{ id: 'image-fixture', name: '图片查看验证群', member_count: 2 }] }); return true; }
  if (p.startsWith('/api/me/groups/image-fixture')) {
    json({ messages: [...Array.from({ length: 10 }, (_, i) => ({ id: 'text-' + i, content: '用于检查关闭图片后的聊天位置。', sender_name: '演示成员', created_at: '2026-09-29T08:00:00Z' })),
      ...['normal', 'tall', 'broken'].map((kind, i) => ({ id: kind, content: '', outgoing: i === 1, sender_name: '演示成员', created_at: '2026-09-29T08:00:00Z', attachments: [{ kind: 'image', mime_type: 'image/svg+xml', display_name: kind + '-图片.svg', url: url.origin + '/fixture-image/' + kind + '.svg' }] }))], members: [], posts: [], items: [], ai_members: [] }); return true;
  }
  if (p.startsWith('/fixture-image/')) {
    res.writeHead(p.includes('broken') && failed ? 503 : 200, { 'content-type': 'image/svg+xml', 'cache-control': 'no-store' });
    res.end(p.includes('broken') && failed ? '' : drawing(p.includes('tall'))); return true;
  }
  return false;
} });

async function main() {
  const origin = await fixture.listen(), output = path.resolve('.ai-tmp/image-preview'); fs.mkdirSync(output, { recursive: true });
  if (process.env.PWA_IMAGE_PREVIEW === '1') { console.log(JSON.stringify({ origin, synthetic: true })); return; }
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
    await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    const page = await context.newPage(), errors = [], cases = []; page.setDefaultTimeout(8000); page.on('pageerror', e => errors.push(e.message));
    await page.goto(origin + '/?fixture=login'); await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').tap();
    await page.locator('#appView:not(.hidden)').waitFor(); await page.waitForLoadState('networkidle');
    const dialog = () => page.getByRole('dialog', { name: '图片预览', exact: true });
    const thumbnail = (name = 'normal') => page.getByRole('button', { name: '查看大图：' + name + '-图片.svg', exact: true });
    const scale = () => page.locator('.chat-image-stage').getAttribute('data-scale').then(Number);
    async function enter() {
      await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
      await page.locator('.conversation-item').filter({ hasText: '图片查看验证群' }).tap(); await thumbnail().waitFor();
    }
    async function ready() { await page.waitForFunction(() => { const img = document.querySelector('.chat-image-original'); return img && !img.hidden && img.naturalWidth > 0; }); }
    async function opened(name = 'normal') { await thumbnail(name).scrollIntoViewIfNeeded(); await thumbnail(name).tap(); await dialog().waitFor(); if (name !== 'broken') await ready(); }
    async function closed() { await dialog().waitFor({ state: 'detached' }); await page.waitForFunction(() => !history.state?.elonImagePreview); }
    async function close() { await dialog().getByRole('button', { name: '关闭图片预览', exact: true }).tap(); await closed(); }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 390, 430]) {
        await page.setViewportSize({ width, height: 844 }); await enter(); await thumbnail().scrollIntoViewIfNeeded();
        const previous = await page.locator('#chatList').evaluate(el => el.scrollTop);
        await opened(); const bounds = await dialog().boundingBox(); assert(bounds.width >= width - 1 && bounds.height >= 843);
        assert.equal(await scale(), 1); assert.equal(await dialog().getByRole('link', { name: '下载原图' }).getAttribute('download'), 'normal-图片.svg');
        const before = await page.locator('.chat-image-original').boundingBox();
        await dialog().getByRole('button', { name: '放大图片', exact: true }).tap(); assert((await scale()) > 1);
        assert((await page.locator('.chat-image-original').boundingBox()).width > before.width);
        await dialog().getByRole('button', { name: '恢复适合屏幕' }).tap(); assert.equal(await scale(), 1);
        if (width === 390) await page.screenshot({ path: path.join(output, `${engine}-${theme}.png`) });
        await close(); assert(Math.abs(await page.locator('#chatList').evaluate(el => el.scrollTop) - previous) < 2);
        assert(await thumbnail().evaluate(el => document.activeElement === el));
        cases.push(`${theme}-${width}-open-zoom-download-close-position`);
      }
    }
    await opened(); await page.keyboard.press('Tab'); assert(await dialog().evaluate(el => el.contains(document.activeElement)));
    await page.keyboard.press('Escape'); await closed(); cases.push('keyboard-focus-escape');
    await thumbnail().focus(); await page.keyboard.press('Enter'); await ready(); await close();
    await thumbnail().focus(); await page.keyboard.press('Shift+F10');
    assert(await page.getByRole('button', { name: '识别二维码', exact: true }).first().isVisible());
    assert.equal(await dialog().count(), 0); cases.push('keyboard-open-qr-menu-preserved');
    const messageMenu = page.getByRole('dialog', { name: '消息操作', exact: true });
    if (await messageMenu.count()) { await page.keyboard.press('Escape'); await messageMenu.waitFor({ state: 'detached' }); await page.waitForTimeout(750); }
    await opened(); await page.goBack(); await closed(); assert.equal(new URL(page.url()).origin, origin); cases.push('browser-back-closes-only-preview');
    await opened(); const stage = page.locator('.chat-image-stage'), box = await stage.boundingBox();
    await stage.tap({ position: { x: box.width / 2, y: box.height / 2 } }); await stage.tap({ position: { x: box.width / 2, y: box.height / 2 } });
    assert((await scale()) > 1); await dialog().getByRole('button', { name: '恢复适合屏幕' }).tap(); cases.push('double-tap-zoom');
    if (engine === 'chromium') {
      const cdp = await context.newCDPSession(page), cy = box.y + box.height / 2, cx = box.x + box.width / 2;
      const points = d => [{ x: cx - d, y: cy, id: 1 }, { x: cx + d, y: cy, id: 2 }];
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: points(40) });
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: points(90) });
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
      await cdp.detach(); cases.push('browser-two-finger-pinch');
    } else {
      await stage.evaluate(el => {
        const b = el.getBoundingClientRect(), send = (type, id, x) => el.dispatchEvent(new PointerEvent(type, { pointerId: id, pointerType: 'touch', bubbles: true, clientX: b.x + b.width / 2 + x, clientY: b.y + b.height / 2 }));
        send('pointerdown', 10, -40); send('pointerdown', 11, 40); send('pointermove', 10, -90); send('pointermove', 11, 90); send('pointerup', 10, -90); send('pointerup', 11, 90);
      }); cases.push('webkit-pinch-event-logic-not-physical-gesture');
    }
    assert((await scale()) >= 2);
    const transform = await page.locator('.chat-image-original').evaluate(el => el.style.transform);
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 50, box.y + box.height / 2 + 25, { steps: 5 }); await page.mouse.up();
    assert.notEqual(await page.locator('.chat-image-original').evaluate(el => el.style.transform), transform); cases.push('zoomed-image-pan');
    while (!(await dialog().getByRole('button', { name: '放大图片', exact: true }).isDisabled())) await dialog().getByRole('button', { name: '放大图片', exact: true }).tap();
    assert.equal(await scale(), 8); await close(); cases.push('bounded-zoom');
    await opened('tall'); assert.equal(await scale(), 1);
    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForFunction(() => { const img = document.querySelector('.chat-image-original'), stage = document.querySelector('.chat-image-stage'); return img.getBoundingClientRect().height <= stage.clientHeight + 1; });
    await close(); await page.setViewportSize({ width: 320, height: 844 }); cases.push('long-image-orientation');
    await opened(); const fontStyle = await page.addStyleTag({ content: '.chat-image-viewer button,.chat-image-title,.chat-image-download{font-size:32px!important}' });
    const targets = await dialog().locator('button:not([hidden]),a').evaluateAll(nodes => nodes.map(el => ({ b: el.getBoundingClientRect().toJSON(), hidden: getComputedStyle(el).display === 'none' })));
    assert(targets.filter(t => !t.hidden).every(t => t.b.x >= 0 && t.b.right <= 321 && t.b.height >= 48));
    await close(); await fontStyle.evaluate(el => el.remove()); cases.push('large-font-controls-reflow');
    await opened('broken'); await dialog().getByRole('button', { name: '重试加载图片' }).waitFor();
    failed = false; await dialog().getByRole('button', { name: '重试加载图片' }).tap(); await ready(); await close(); cases.push('network-error-retry');
    await opened(); await thumbnail().evaluate(el => el.remove()); await closed(); cases.push('removed-source-closes-preview');
    await enter(); await opened(); await page.locator('#backBtn').evaluate(el => el.click()); await closed(); cases.push('leave-chat-closes-preview');
    assert.deepEqual(errors, []); assert.equal(fixture.audit.rejectedWrites, 0);
    const receipt = { status: 'passed', engine, cases, productionWrites: false, physicalIphoneVerified: false };
    fs.writeFileSync(path.join(output, `${engine}.json`), JSON.stringify(receipt, null, 2)); console.log(JSON.stringify(receipt));
  } finally { await browser.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; void fixture.close(); });
