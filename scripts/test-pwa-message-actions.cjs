// Production PWA, loopback-only APIs and synthetic content. Never sends real group messages.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { attachmentFixture } = require('./pwa-attachment-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'chromium';
async function main() {
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
  const fixture = await attachmentFixture(context), { state, origin } = fixture;
  const page = await context.newPage(), errors = [], cases = [];
  page.setDefaultTimeout(8000); page.on('pageerror', e => errors.push(e.message));
  const output = path.resolve('.ai-tmp/pwa-message-actions'); fs.mkdirSync(output, { recursive: true });
  const png = fs.readFileSync(path.join(__dirname, 'fixtures/social-source-qr.png'));
  const image = { kind: 'image', mime_type: 'image/png', file_name: 'example.png', display_name: '图片', size_bytes: png.length, url: origin + '/api/user/mobile-v2-fixture/chat-attachments/download/0' };
  state.uploads.push({ attachment: image, buffer: png });
  const original = { id: 'photo', content: '', sender_name: '演示成员', revision: 1, attachments: [image], created_at: '2026-09-29T08:00:00Z' };
  for (const kind of ['group', 'friend']) {
    state.messages.push({ path: `/api/me/${kind}s/attach-${kind}/messages`, message: { ...original, id: kind + '-photo' } },
      { path: `/api/me/${kind}s/attach-${kind}/messages`, message: { ...original, id: kind + '-text', attachments: [], content: '一条文字消息' } });
  }
  const row = id => page.locator(`[data-message-id="${id}"]`), menu = () => page.getByRole('dialog', { name: '消息操作', exact: true });
  const preview = () => page.locator('.social-quote-compose');
  async function enter(kind = 'group') {
    await page.goto(origin + '/?fixture=login');
    if (await page.locator('#loginView').isVisible()) {
      await page.locator('#accountInput').fill('mobile-v2-fixture'); await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').tap();
    }
    await page.locator('.conversation-item').filter({ hasText: kind === 'group' ? '附件测试群' : '附件测试好友' }).tap();
    await row(kind + '-photo').waitFor(); await page.waitForLoadState('networkidle');
  }
  async function open(id) { await row(id).locator('.social-message-more').tap(); await menu().waitFor(); }
  async function quote(id) { await open(id); await menu().getByRole('button', { name: '引用', exact: true }).tap(); await preview().waitFor(); await menu().waitFor({ state: 'detached' }); assert(await page.locator('#messageInput').evaluate(el => document.activeElement === el)); }
  async function forwardTo(kind) {
    const dialog = page.getByRole('dialog', { name: '转发消息', exact: true }); await dialog.waitFor();
    await dialog.getByRole('combobox').selectOption(`${kind}:attach-${kind}`);
    await dialog.getByRole('button', { name: '确认转发', exact: true }).tap();
    await dialog.getByRole('status').filter({ hasText: /^已转发/ }).waitFor();
    await dialog.getByRole('button', { name: '关闭', exact: true }).tap();
  }
  try {
    await enter();
    const picture = row('group-photo').locator('img[alt="图片"]');
    await picture.scrollIntoViewIfNeeded();
    await picture.dispatchEvent('pointerdown', { pointerType: 'touch', clientX: 100, clientY: 250 });
    await menu().waitFor(); await picture.dispatchEvent('pointerup', { pointerType: 'touch' });
    for (const name of ['引用', '复制', '转发', '多选', '识别二维码']) assert(await menu().getByRole('button', { name, exact: true }).isVisible());
    assert.equal(await page.getByRole('dialog', { name: '图片预览', exact: true }).count(), 0);
    await menu().getByRole('button', { name: '关闭', exact: true }).tap(); cases.push('image-long-press-common-menu-no-preview');
    await page.waitForTimeout(850);
    await picture.tap(); await page.getByRole('dialog', { name: '图片预览', exact: true }).waitFor();
    await page.getByRole('button', { name: '关闭图片预览', exact: true }).tap(); cases.push('single-tap-preview-retained');
    await picture.dispatchEvent('contextmenu'); await menu().waitFor(); await page.keyboard.press('Escape');
    await menu().waitFor({ state: 'detached' });
    assert(await row('group-photo').locator('.social-message-more').evaluate(el => document.activeElement === el)); cases.push('right-click-escape-focus');
    await picture.dispatchEvent('pointerdown', { pointerType: 'touch', clientX: 100, clientY: 250 });
    await picture.dispatchEvent('pointermove', { pointerType: 'touch', clientX: 100, clientY: 280 });
    await page.waitForTimeout(600); assert.equal(await menu().count(), 0); cases.push('scroll-cancels-long-press');

    for (const kind of ['group', 'friend']) {
      await enter(kind); await page.locator('#messageInput').fill('保留草稿'); await quote(kind + '-photo');
      assert.equal(await page.locator('#messageInput').inputValue(), '保留草稿');
      assert(await preview().locator('img').isVisible());
      await page.getByRole('button', { name: '取消引用', exact: true }).tap();
      assert.equal(await page.locator('#messageInput').inputValue(), '保留草稿'); assert.equal(await preview().isVisible(), false);
      await quote(kind + '-photo'); await page.locator('#sendBtn').tap();
      await page.waitForFunction(() => document.querySelector('.social-quote-compose').hidden);
      const sent = state.messages.at(-1).message;
      assert.deepEqual(sent.quote_source, { message_id: kind + '-photo', revision: 1 });
      assert.equal(sent.content, '保留草稿'); assert.equal(sent.attachments.length, 0);
      await row(sent.id).getByRole('button', { name: /演示成员/ }).waitFor(); cases.push(kind + '-quote-cancel-send-structured');
    }
    await enter(); await quote('group-photo'); state.failSend = true;
    await page.locator('#messageInput').fill('失败保留'); await page.locator('#sendBtn').tap();
    await page.waitForFunction(() => document.querySelector('#messageInput').value === '失败保留');
    assert(await preview().isVisible()); state.failSend = false; cases.push('send-failure-retains-draft-quote');

    await enter(); await open('group-photo'); await menu().getByRole('button', { name: '转发', exact: true }).tap();
    const before = state.uploads.length; await forwardTo('friend');
    assert.equal(state.uploads.length, before + 1); assert.deepEqual(state.uploads.at(-1).buffer, png);
    assert.equal(state.uploads.at(-1).conversation, 'friend-attach-friend');
    assert.equal(state.messages.at(-1).message.attachments.length, 1); cases.push('forward-original-image-bytes-new-target-scope');

    await open('group-photo'); await menu().getByRole('button', { name: '多选', exact: true }).tap();
    await row('group-text').getByRole('checkbox', { name: '选择消息' }).check();
    assert.equal(await page.locator('.social-message-selection span').textContent(), '已选择 2 条');
    await page.locator('.social-message-selection').getByRole('button', { name: '转发', exact: true }).tap();
    const count = state.messages.length; await forwardTo('friend');
    assert.equal(state.messages.length, count + 2); assert.equal(state.messages.at(-2).message.content, ''); assert.equal(state.messages.at(-1).message.content, '一条文字消息');
    await page.getByRole('button', { name: '取消多选' }).tap(); cases.push('multiselect-image-text-ordered-forward');

    await open('group-photo'); await menu().getByRole('button', { name: '转发', exact: true }).tap(); state.failSend = true;
    const dialog = page.getByRole('dialog', { name: '转发消息', exact: true });
    await dialog.getByRole('combobox').selectOption('friend:attach-friend');
    await dialog.getByRole('button', { name: '确认转发' }).tap(); await dialog.getByRole('status').filter({ hasText: '不会自动重发' }).waitFor();
    assert(await dialog.getByRole('button', { name: '确认转发' }).isDisabled());
    await dialog.getByRole('button', { name: '关闭', exact: true }).tap(); state.failSend = false; cases.push('uncertain-forward-no-retry');

    await page.evaluate(() => {
      window.copied = null;
      Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { write: async items => {
        const blob = await items[0].getType('image/png'); window.copied = Array.from(new Uint8Array(await blob.arrayBuffer()));
      }, writeText: async value => { window.copied = value; } } });
    });
    await open('group-photo'); await menu().getByRole('button', { name: '复制', exact: true }).tap();
    await page.waitForFunction(() => Array.isArray(window.copied)); assert.deepEqual(await page.evaluate(() => copied), Array.from(png)); cases.push('clipboard-receives-image-bytes');
    await page.evaluate(async item => {
      const url = new URL(item.url); url.protocol = 'http:'; url.port = '1'; window.copied = null;
      await ElonSocialMessageTransfer.copy([{ attachments: [{ ...item, url: url.href }] }], { api: (...args) => fetch(...args), current: () => true });
    }, image);
    assert.deepEqual(await page.evaluate(() => copied), Array.from(png)); cases.push('attachment-http-port-rebased-to-current-ingress');
    const jpeg = await page.evaluate(async url => {
      const bitmap = await createImageBitmap(await (await fetch(url)).blob()), canvas = document.createElement('canvas');
      canvas.width = bitmap.width; canvas.height = bitmap.height; canvas.getContext('2d').drawImage(bitmap, 0, 0); bitmap.close();
      return canvas.toDataURL('image/jpeg').split(',')[1];
    }, image.url);
    const jpegRef = { ...image, file_name: 'photo.jpg', mime_type: 'image/jpeg', url: origin + '/api/user/mobile-v2-fixture/chat-attachments/download/' + state.uploads.length };
    state.uploads.push({ attachment: jpegRef, buffer: Buffer.from(jpeg, 'base64') });
    state.messages.push({ path: '/api/me/groups/attach-group/messages', message: { ...original, id: 'jpeg', attachments: [jpegRef] } });
    // Re-enter the latest window; background arrivals must not move a history reader.
    await enter(); await row('jpeg').waitFor();
    await page.evaluate(() => { window.copied = null; Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { write: async items => {
      const blob = await items[0].getType('image/png'); window.copied = Array.from(new Uint8Array(await blob.arrayBuffer()));
    } } }); });
    await open('jpeg'); await menu().getByRole('button', { name: '复制', exact: true }).tap(); await page.waitForFunction(() => Array.isArray(window.copied));
    assert.deepEqual((await page.evaluate(() => copied)).slice(0, 8), [137, 80, 78, 71, 13, 10, 26, 10]); cases.push('jpeg-copied-as-browser-compatible-png');
    await page.evaluate(() => Object.defineProperty(navigator, 'clipboard', { configurable: true, value: {} }));
    await open('group-photo'); await menu().getByRole('button', { name: '复制', exact: true }).tap();
    await menu().getByRole('status').filter({ hasText: '不支持复制' }).waitFor(); await page.keyboard.press('Escape'); cases.push('clipboard-unsupported-explicit');

    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 390, 1024]) {
        await page.setViewportSize({ width, height: 844 }); await enter(); await quote('group-photo');
        await page.locator('#messageInput').fill('这是一条用于检查引用样式的消息'); await page.locator('#messageInput').blur();
        assert(await preview().evaluate(el => el.getBoundingClientRect().right <= innerWidth && el.getBoundingClientRect().left >= 0));
        await page.screenshot({ path: path.join(output, `${engine}-${theme}-${width}.png`) });
        cases.push(`${theme}-${width}-quote-layout`);
      }
    }
    await enter(); await quote('group-photo');
    state.messages.find(x => x.message.id === 'group-photo').message.revision = 2;
    // Production timeline sync is every 15 s; retain the real schedule here.
    await preview().getByText('原消息已修改或不可用，请取消后重新引用').waitFor({ timeout: 20000 });
    const posts = state.messages.length; await page.locator('#messageInput').fill('不得发送'); await page.locator('#sendBtn').tap();
    await page.waitForFunction(() => document.querySelector('#messageInput').value === '不得发送');
    assert.equal(state.messages.length, posts); cases.push('source-revision-fails-before-write');
    await enter('friend'); assert.equal(await preview().isVisible(), false); cases.push('scope-change-clears-quote');
    assert.deepEqual(errors, []); assert.deepEqual(state.external, []);
    console.log(JSON.stringify({ engine, passed: cases.length, cases, productionWrites: 0, errors }));
  } finally { await context.close(); await browser.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
