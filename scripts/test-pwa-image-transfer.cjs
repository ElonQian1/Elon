// Loopback only. Synthetic transfer payloads exercise real PWA listeners, previews and upload bytes.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { attachmentFixture } = require('./pwa-attachment-fixture.cjs');

async function main() {
  const engine = process.env.BROWSER_ENGINE || 'chromium';
  const browser = await playwright[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixture = await attachmentFixture(context), { origin, state } = fixture;
  const page = await context.newPage(), errors = [], cases = [];
  page.setDefaultTimeout(6000); page.on('pageerror', e => errors.push(e.message));
  const output = path.join(__dirname, '../.ai-tmp/pwa-image-transfer'); fs.mkdirSync(output, { recursive: true });
  const bytes = fs.readFileSync(path.join(__dirname, 'fixtures/social-source-qr.png'));
  const photo = { name: '截图.png', type: 'image/png', bytes: [...bytes] };
  const dialog = () => page.getByRole('dialog', { name: '分享预览', exact: true });
  async function enter(kind = 'group') {
    await page.goto(origin + '/?fixture=login');
    if (await page.locator('#loginView').isVisible()) {
      await page.locator('#accountInput').fill('mobile-v2-fixture');
      await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').click();
    }
    await page.locator('.conversation-item').filter({ hasText: kind === 'group' ? '附件测试群' : '附件测试好友' }).click();
    await page.locator('#inputBar.active').waitFor(); await page.waitForLoadState('networkidle');
  }
  async function transfer(type, files = [photo], extra = {}) {
    return page.evaluate(({ type, files, extra }) => {
      const data = new DataTransfer();
      files.forEach(f => data.items.add(new File([new Uint8Array(f.size ?? f.bytes)], f.name, { type: f.type })));
      if (extra.text) data.setData('text/plain', extra.text);
      if (extra.html) data.setData('text/html', extra.html);
      let payload = data;
      if (extra.itemsOnly) payload = { items: data.items, files: [], types: ['Files'], getData: t => data.getData(t) };
      if (extra.emptyFiles) payload = { items: [], files: [], types: ['Files'], getData: () => '' };
      if (type === 'dragover') payload = { types: ['Files'], get files() { throw Error('protected files read before drop'); } };
      const event = new Event(type, { bubbles: true, cancelable: true });
      Object.defineProperty(event, type === 'paste' ? 'clipboardData' : 'dataTransfer', { value: payload });
      document.querySelector(extra.selector || '#messageInput').dispatchEvent(event);
      return event.defaultPrevented;
    }, { type, files, extra });
  }
  async function cancel() { await dialog().getByRole('button', { name: '取消', exact: true }).click(); await dialog().waitFor({ state: 'detached' }); }
  async function send(kind, files = [photo]) {
    const count = state.messages.length, uploaded = state.uploads.length;
    await dialog().getByRole('button', { name: '发送', exact: true }).click();
    await dialog().waitFor({ state: 'detached' }); await page.waitForLoadState('networkidle');
    assert.equal(state.messages.length, count + 1);
    assert.equal(state.messages.at(-1).path, `/api/me/${kind}s/attach-${kind}/messages`);
    assert.equal(state.uploads.length, uploaded + files.length);
    for (let i = 0; i < files.length; i++) {
      assert.deepEqual(state.uploads[uploaded + i].buffer, Buffer.from(files[i].bytes));
      assert.equal(state.uploads[uploaded + i].attachment.mime_type, files[i].type || 'image/png');
    }
  }
  try {
    for (const kind of ['group', 'friend']) {
      await enter(kind);
      await page.locator('#messageInput').fill('未发送的草稿');
      const before = state.uploads.length;
      assert.equal(await transfer('paste'), true, 'image paste must be handled');
      await dialog().waitFor(); assert.equal(state.uploads.length, before, 'preview must not upload');
      await dialog().locator('img').waitFor();
      assert.equal(await page.locator('#messageInput').inputValue(), '未发送的草稿');
      await send(kind); assert.equal(await page.locator('#messageInput').inputValue(), '未发送的草稿');
      cases.push(kind + '-paste-preview-confirm-bytes-draft');
      assert.equal(await transfer('dragover', [], { selector: '#chatList' }), true);
      assert.equal(await transfer('drop', [photo], { selector: '#chatList' }), true);
      await send(kind); cases.push(kind + '-drop-chat-no-navigation');
    }
    await enter();
    assert.equal(await transfer('paste', [photo], { itemsOnly: true, text: '图片说明' }), true);
    assert.equal(await dialog().getByRole('textbox', { name: '附件说明' }).inputValue(), '图片说明');
    await send('group'); assert.equal(state.messages.at(-1).message.content, '图片说明');
    cases.push('clipboard-items-fallback-mixed-text');
    const unnamed = { ...photo, name: '' }, missingMime = { ...photo, name: '本地.PNG', type: '' };
    await transfer('drop', [unnamed, missingMime]);
    assert.equal(await dialog().locator('img').count(), 2);
    await send('group', [unnamed, missingMime]);
    assert.match(state.messages.at(-1).message.attachments[0].file_name, /\.png$/);
    cases.push('unnamed-clipboard-and-empty-mime-local-image');
    const file = { name: '说明.txt', type: 'text/plain', bytes: [...Buffer.from('local file')] };
    await transfer('drop', [photo, file]); await send('group', [photo, file]); cases.push('mixed-local-files');
    const beforeCancel = state.uploads.length;
    await transfer('paste'); await cancel(); assert.equal(state.uploads.length, beforeCancel);
    assert.equal(await page.locator('#messageInput').inputValue(), ''); cases.push('cancel-no-upload');
    await transfer('paste');
    await page.evaluate(photo => {
      document.querySelector('dialog.social-compose-dialog').querySelector('button').click();
      const data = new DataTransfer(); data.items.add(new File([new Uint8Array(photo.bytes)], photo.name, { type: photo.type }));
      const event = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(event, 'clipboardData', { value: data }); document.querySelector('#messageInput').dispatchEvent(event);
    }, photo);
    await dialog().waitFor(); await cancel(); cases.push('immediate-repaste-after-cancel');
    assert.equal(await transfer('paste', [], { text: '普通文字' }), false);
    assert.equal(await transfer('paste', [], { text: 'https://example.invalid/image.png', html: '<img src="https://example.invalid/image.png">' }), false);
    assert.equal(await transfer('drop', [], { text: '拖入文字' }), false);
    assert.equal(await dialog().count(), 0); cases.push('text-and-url-paste-keeps-browser-default-no-fetch');
    assert.equal(await transfer('drop', [], { emptyFiles: true }), true, 'unreadable Files transfer must be handled');
    await dialog().getByRole('status').filter({ hasText: '未能读取' }).waitFor();
    assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true); await cancel();
    cases.push('unreadable-file-actionable-error');
    for (const files of [[{ ...photo, size: 0 }], [{ ...photo, size: 12 * 1024 * 1024 + 1 }], Array(7).fill(photo)]) {
      await transfer('paste', files);
      assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true); await cancel();
    }
    assert.equal(state.uploads.length, beforeCancel); cases.push('shared-empty-size-count-limits');
    await transfer('paste'); state.failUpload = state.attempts + 1;
    await dialog().getByRole('button', { name: '发送', exact: true }).click();
    await dialog().getByRole('status').filter({ hasText: '附件上传失败' }).waitFor();
    state.failUpload = 0; await send('group'); cases.push('paste-upload-retry');
    const original = state.messages.find(item => item.path.includes('/groups/')).message;
    await page.locator(`[data-message-id="${original.id}"] .social-message-more`).click();
    await page.getByRole('dialog', { name: '消息操作', exact: true }).getByRole('button', { name: '引用', exact: true }).click();
    await page.locator('.social-quote-compose').waitFor();
    await transfer('paste'); await transfer('paste');
    assert.equal(await dialog().count(), 1, 'repeated event must not open or send twice');
    await send('group'); assert.equal(state.messages.at(-1).message.quote.message_id, original.id);
    cases.push('quote-paste-and-duplicate-event-single-send');
    await transfer('paste'); const pending = state.uploads.length;
    await page.locator('#backBtn').evaluate(button => button.click());
    await dialog().waitFor({ state: 'detached' }); assert.equal(state.uploads.length, pending);
    assert.equal(await transfer('paste'), false); cases.push('conversation-switch-cancels-preview-and-rejects-stale-input');
    await enter();
    await page.locator('#messageInput').evaluate(el => { el.disabled = true; });
    assert.equal(await transfer('paste'), false);
    await page.locator('#messageInput').evaluate(el => { el.disabled = false; });
    assert.equal(await transfer('paste', [photo], { selector: '#accountInput' }), false); cases.push('disabled-and-other-input-not-intercepted');
    if (engine === 'chromium') {
      await context.grantPermissions(['clipboard-read', 'clipboard-write']);
      await page.evaluate(() => navigator.clipboard.writeText('正常的文字粘贴'));
      await page.locator('#messageInput').focus(); await page.keyboard.press('Control+V');
      assert.equal(await page.locator('#messageInput').inputValue(), '正常的文字粘贴');
      cases.push('chromium-system-clipboard-text-paste');
      await page.evaluate(async bytes => {
        await navigator.clipboard.write([new ClipboardItem({ 'image/png': new Blob([new Uint8Array(bytes)], { type: 'image/png' }) })]);
      }, [...bytes]);
      await page.locator('#messageInput').focus(); await page.keyboard.press('Control+V');
      await dialog().waitFor(); await cancel(); cases.push('chromium-system-clipboard-keyboard-paste');
    }
    await transfer('paste'); await page.screenshot({ path: path.join(output, engine + '-preview.png') }); await cancel();
    assert.deepEqual(errors, []); assert.deepEqual(state.external, []);
    const result = { status: 'passed', engine, cases, productionWrites: false, macOsNativeVerified: false };
    fs.writeFileSync(path.join(output, engine + '.json'), JSON.stringify(result, null, 2)); console.log(JSON.stringify(result));
  } catch (error) {
    console.error(JSON.stringify({ cases, errors, dialogs: await page.locator('dialog').allTextContents(),
      input: await page.locator('#messageInput').evaluate(el => ({ disabled: el.disabled, readOnly: el.readOnly, parent: el.parentElement.className })) })); throw error;
  } finally { await browser.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
