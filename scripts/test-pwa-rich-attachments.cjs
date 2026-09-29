const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { attachmentFixture, audioFile, installRecorderDouble } = require('./pwa-attachment-fixture.cjs');

async function main() {
  const engine = process.env.BROWSER_ENGINE || 'chromium';
  const browser = await playwright[engine].launch({ headless: true, ...(engine === 'chromium'
    ? { channel: process.env.BROWSER_CHANNEL || 'msedge', args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream'] } : {}) });
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const fixture = await attachmentFixture(context), { state, origin } = fixture;
  const page = await context.newPage(), errors = [], cases = [];
  const output = path.join(__dirname, '../.ai-tmp/pwa-rich-attachments'); fs.mkdirSync(output, { recursive: true });
  page.setDefaultTimeout(10000); page.on('pageerror', error => errors.push(error.stack || error.message));
  const dialog = () => page.locator('dialog[aria-label="分享预览"]');
  const voice = () => page.locator('dialog[aria-label="录制语音"]');
  const photo = { name: '测试图片.png', mimeType: 'image/png', buffer: fs.readFileSync(path.join(__dirname, 'fixtures/social-source-qr.png')) };
  const document = { name: '说明.txt', mimeType: 'text/plain', buffer: Buffer.from('真实文件字节\n附件回归') };
  async function enter(kind = 'group') {
    await page.goto(origin + '/?fixture=login');
    if (await page.locator('#loginView').isVisible()) {
      await page.locator('#accountInput').fill('mobile-v2-fixture'); await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').tap();
    }
    await page.locator('.conversation-item').filter({ hasText: kind === 'group' ? '附件测试群' : '附件测试好友' }).tap();
    await page.locator('#inputBar.active').waitFor(); await page.waitForLoadState('networkidle');
  }
  async function select(action, files) {
    await page.locator('#imageEditBtn').tap();
    const pending = page.waitForEvent('filechooser');
    await page.locator('.input-attachment-action').filter({ hasText: action }).tap();
    const chooser = await pending;
    assert.equal(await page.locator('#nativeOnlyMask').evaluate(el => el.classList.contains('active')), false, 'attachment actions must not trigger the native-only handler');
    const attributes = await chooser.element().evaluate(input => ({ attached: input.isConnected, accept: input.accept, capture: input.getAttribute('capture'), multiple: input.multiple }));
    assert.equal(attributes.attached, true, 'file chooser stays attached for iOS');
    if (action === '文件') assert.equal(attributes.accept, '', 'ordinary files must not be filtered out');
    if (action === '拍照') { assert.equal(attributes.capture, 'environment'); assert.equal(attributes.multiple, false); }
    if (action === '相册') assert.equal(attributes.accept, 'image/*,video/*');
    await chooser.setFiles(files); await dialog().waitFor(); return attributes;
  }
  async function sendAndCheck(kind, files) {
    const before = state.messages.length, first = state.uploads.length;
    await dialog().getByRole('button', { name: '发送', exact: true }).tap();
    await dialog().waitFor({ state: 'detached' }); await page.waitForLoadState('networkidle');
    assert.equal(state.messages.length, before + 1);
    assert.equal(state.messages.at(-1).path, `/api/me/${kind}s/attach-${kind}/messages`);
    const refs = state.messages.at(-1).message.attachments; assert.equal(refs.length, files.length);
    for (let i = 0; i < files.length; i++) {
      const uploaded = state.uploads[first + i];
      assert.equal(uploaded.conversation, `${kind}-attach-${kind}`);
      assert.equal(uploaded.headers['content-type'], files[i].mimeType || 'application/octet-stream');
      assert.equal(refs[i].sha256, crypto.createHash('sha256').update(files[i].buffer).digest('hex'));
      assert.deepEqual(uploaded.buffer, files[i].buffer);
    }
  }
  async function cancelPreview() { await dialog().getByRole('button', { name: '取消', exact: true }).tap(); await dialog().waitFor({ state: 'detached' }); }
  async function openVoice() { await page.locator('#voiceBtn').tap(); await voice().waitFor(); assert.equal(await page.locator('#nativeOnlyMask').evaluate(el => el.classList.contains('active')), false); }
  async function cancelVoice() { await voice().getByRole('button', { name: '取消', exact: true }).tap(); await voice().waitFor({ state: 'detached' }); }
  try {
    for (const kind of ['group', 'friend']) {
      await enter(kind);
      await select('拍照', [photo]); await cancelPreview(); cases.push(kind + '-camera-cancel');
      await select('相册', [photo]);
      const links = dialog().getByRole('combobox', { name: '选择图片原文链接' });
      await links.waitFor(); await links.selectOption('https://mp.weixin.qq.com/s/synthetic-test?scene=90');
      if (kind === 'group') await page.screenshot({ path: path.join(output, `${engine}-preview.png`) });
      await sendAndCheck(kind, [photo]);
      assert.equal(state.messages.at(-1).message.attachments[0].source_link.url, 'https://mp.weixin.qq.com/s/synthetic-test?scene=90');
      await page.locator('#chatList img[alt="测试图片.png"]').last().waitFor(); cases.push(kind + '-image');
      const files = [document, { name: '计划.docx', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', buffer: Buffer.from('synthetic-docx') }, audioFile()];
      await select('文件', files); await sendAndCheck(kind, files);
      await page.locator('#chatList .voice-message-play').last().tap();
      await page.waitForFunction(() => [...document.querySelectorAll('#chatList audio')].at(-1).currentTime > 0);
      await page.locator('#chatList .voice-message-play').last().tap();
      cases.push(kind + '-documents-and-playable-audio');
    }
    await enter();
    const video = { name: '视频类型测试.mp4', mimeType: 'video/mp4', buffer: Buffer.from('synthetic-video-descriptor-not-decoding-evidence') };
    await select('相册', [video]); await sendAndCheck('group', [video]);
    const videoPlayer = page.locator('#chatList video').last(); await videoPlayer.waitFor();
    assert.equal(await videoPlayer.evaluate(el => el.controls && el.playsInline), true); cases.push('video-upload-inline-controls');
    const before = state.uploads.length;
    await select('文件', [{ ...document, buffer: Buffer.alloc(0) }]);
    assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true); await cancelPreview();
    await select('文件', Array.from({ length: 7 }, () => document));
    assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true); await cancelPreview();
    await select('文件', [{ ...document, buffer: Buffer.alloc(12 * 1024 * 1024 + 1) }]);
    assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true); await cancelPreview();
    assert.equal(state.uploads.length, before); cases.push('size-count-empty-no-upload');

    await select('文件', [document, { ...document, name: '重试.txt' }]);
    state.failUpload = state.attempts + 2;
    await dialog().getByRole('button', { name: '发送', exact: true }).tap();
    await dialog().getByRole('status').filter({ hasText: '附件上传失败' }).waitFor();
    const afterPartial = state.uploads.length;
    state.failUpload = 0; await dialog().getByRole('button', { name: '发送', exact: true }).tap();
    await dialog().waitFor({ state: 'detached' }); await page.waitForLoadState('networkidle');
    assert.equal(state.uploads.length, afterPartial + 1, 'reuse a successful upload on retry'); cases.push('partial-upload-retry');

    await select('文件', [document]); state.failSend = true;
    await dialog().getByRole('button', { name: '发送', exact: true }).tap();
    await dialog().getByRole('status').filter({ hasText: '发送结果未确认' }).waitFor();
    assert.equal(await dialog().getByRole('button', { name: '发送', exact: true }).isDisabled(), true);
    await dialog().getByRole('button', { name: '关闭', exact: true }).tap(); await dialog().waitFor({ state: 'detached' }); state.failSend = false;
    cases.push('uncertain-send-no-repeat');

    let releaseUpload; state.uploadGate = new Promise(resolve => { releaseUpload = resolve; });
    await select('文件', [document]); const beforeCancel = state.messages.length;
    const uploading = page.waitForRequest(request => request.url().includes('/chat-attachments?'));
    await dialog().getByRole('button', { name: '发送', exact: true }).tap(); await uploading;
    await cancelPreview(); releaseUpload(); state.uploadGate = null; await page.waitForLoadState('networkidle');
    assert.equal(state.messages.length, beforeCancel); cases.push('cancel-upload-no-message');

    await enter(); await installRecorderDouble(page);
    await openVoice(); await page.screenshot({ path: path.join(output, `${engine}-voice.png`) });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await voice().getByRole('status').filter({ hasText: '正在录音' }).waitFor();
    const messagesBeforeVoice = state.messages.length;
    await voice().getByRole('button', { name: '停止录音', exact: true }).tap();
    await voice().getByRole('button', { name: '使用录音', exact: true }).tap();
    assert.equal(state.messages.length, messagesBeforeVoice, 'recording needs a separate send confirmation');
    await dialog().getByRole('button', { name: '发送', exact: true }).tap(); await dialog().waitFor({ state: 'detached' });
    await page.waitForLoadState('networkidle');
    const recording = state.messages.at(-1).message.attachments[0];
    assert.equal(recording.kind, 'voice'); assert.equal(recording.mime_type, 'audio/mp4'); assert(recording.duration_seconds >= 1);
    assert(recording.display_name.endsWith('.m4a')); assert.equal(await page.evaluate(() => voiceTest.stops), 1);
    cases.push('voice-mp4-preview-send-track-release');

    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'deny'; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await voice().getByRole('status').filter({ hasText: '麦克风权限未开启' }).waitFor(); await cancelVoice(); cases.push('voice-permission-denied');
    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'pending'; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap(); await cancelVoice();
    const stopped = await page.evaluate(() => voiceTest.stops);
    await page.evaluate(() => voiceTest.resolve());
    await page.waitForFunction(count => voiceTest.stops === count + 1, stopped); cases.push('voice-cancel-pending-permission');
    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'pending'; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await page.evaluate(() => {
      Object.defineProperty(document, 'hidden', { configurable: true, value: true });
      document.dispatchEvent(new Event('visibilitychange')); delete document.hidden; voiceTest.mode = 'ok';
    });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    const beforeLateDenial = await page.evaluate(() => voiceTest.stops);
    await page.evaluate(async () => { voiceTest.reject(); await Promise.resolve(); await Promise.resolve(); });
    assert.equal(await page.evaluate(() => voiceTest.stops), beforeLateDenial, 'stale permission rejection must not stop the new recording');
    assert.equal(await voice().getByRole('button', { name: '停止录音', exact: true }).isDisabled(), false);
    await cancelVoice(); cases.push('late-permission-denial-preserves-current-recording');
    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'ok'; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap(); await cancelVoice(); cases.push('voice-cancel-recording');
    await openVoice(); const chooserPromise = page.waitForEvent('filechooser');
    await voice().getByRole('button', { name: '选择音频文件' }).tap();
    const chooser = await chooserPromise; assert.equal(await chooser.element().getAttribute('accept'), 'audio/*');
    await chooser.setFiles(audioFile()); await dialog().waitFor(); await cancelPreview(); cases.push('voice-audio-file-fallback');

    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'empty'; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await voice().getByRole('button', { name: '停止录音', exact: true }).tap();
    await voice().getByRole('status').filter({ hasText: '没有录到声音数据' }).waitFor();
    assert.equal(await voice().getByRole('button', { name: '使用录音' }).isDisabled(), true); await cancelVoice(); cases.push('empty-recording-rejected');

    await openVoice(); await page.evaluate(() => { voiceTest.mode = 'ok'; voiceTest.now = Date.now; voiceTest.offset = 0; Date.now = () => voiceTest.now() + voiceTest.offset; });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await page.evaluate(() => { voiceTest.offset = 121000; });
    await voice().getByRole('status').filter({ hasText: '已录制 120 秒' }).waitFor();
    await page.evaluate(() => { Date.now = voiceTest.now; }); await cancelVoice(); cases.push('recording-duration-limit');

    const identityStops = await page.evaluate(() => {
      window.attachmentOwner = 'account-a';
      ElonSourceCompose.openVoice({ owner: () => attachmentOwner, userId: 'account-a', kind: 'group', contact: { id: 'test' }, api: () => { throw Error('must not upload'); }, send: () => { throw Error('must not send'); } });
      return voiceTest.stops;
    });
    await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
    await page.evaluate(() => { attachmentOwner = 'account-b'; }); await voice().waitFor({ state: 'detached' });
    assert.equal(await page.evaluate(() => voiceTest.stops), identityStops + 1); cases.push('account-change-stops-recording');

    // Real browser MediaRecorder encodes a generated microphone, never the user's microphone.
    if (engine === 'chromium') {
      await enter(); await openVoice(); await voice().getByRole('button', { name: '开始录音', exact: true }).tap();
      await voice().getByRole('status').filter({ hasText: '正在录音' }).waitFor();
      await page.waitForTimeout(1200);
      await voice().getByRole('button', { name: '停止录音', exact: true }).tap();
      await voice().getByRole('button', { name: '使用录音', exact: true }).tap();
      await dialog().locator('audio').evaluate(async audio => { await audio.play(); audio.pause(); });
      await dialog().getByRole('button', { name: '发送', exact: true }).tap(); await dialog().waitFor({ state: 'detached' });
      await page.locator('#chatList .voice-message-play').last().tap();
      await page.waitForFunction(() => [...document.querySelectorAll('#chatList audio')].at(-1).currentTime > 0);
      await page.locator('#chatList .voice-message-play').last().tap();
      cases.push('real-recorder-generated-microphone-playback');
    }
    await page.waitForLoadState('networkidle');
    assert.deepEqual(errors, []); assert.deepEqual(state.external, []);
    await page.screenshot({ path: path.join(output, `${engine}.png`) });
    console.log(JSON.stringify({ status: 'passed', engine, cases, uploads: state.uploads.length, syntheticMessages: state.messages.length, productionWrites: false, physicalIosVerified: false }));
  } catch (error) {
    console.error(JSON.stringify({ cases, errors, uploads: state.uploads.length, messages: state.messages.length,
      dialogs: await page.locator('dialog').allTextContents(), native: await page.locator('#nativeOnlyText').textContent(),
      bar: await page.locator('#inputBar').getAttribute('class'), media: await page.locator('audio').evaluateAll(items => items.map(a => ({ src: a.currentSrc, error: a.error?.message, ready: a.readyState,
        wav: a.canPlayType('audio/wav'), mp4: a.canPlayType('audio/mp4'), webm: a.canPlayType('audio/webm;codecs=opus') }))) })); throw error;
  } finally { await browser.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
