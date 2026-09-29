// Synthetic speech-tone messages only. Optional Rust fixture exercises the real download handler.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const engine = process.env.BROWSER_ENGINE || 'webkit';
const tone = fs.readFileSync(path.join(__dirname, 'fixtures/voice-tone.m4a'));
const backend = process.env.ELON_VOICE_FIXTURE_PORT;
const ranges = [], errors = [], cases = [];
let fail = false, recalled = false, generation = 0;
const fixture = createFixture({ handleSyntheticRequest(req, res, url) {
  const p = url.pathname, json = value => { res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(value)); };
  if (p === '/api/me/groups') { json({ groups: [{ id: 'voice-fixture', name: '语音播放验证群', member_count: 2 }] }); return true; }
  if (p.startsWith('/api/me/groups/voice-fixture')) {
    json({ messages: [1, 2].map(i => ({ id: 'voice-' + i, content: '', outgoing: i === 2,
      sender_user_id: i === 2 ? 'mobile-v2-fixture' : 'other-fixture', sender_name: i === 2 ? '我' : '演示成员',
      recalled_at: recalled && i === 1 ? new Date().toISOString() : null,
      created_at: '2026-09-29T06:00:00Z', attachments: [{ kind: 'audio', mime_type: 'audio/mp4',
        display_name: '合成语音.m4a', file_name: 'voice.m4a', duration_seconds: i === 1 ? 2 : undefined,
        url: url.origin + `/api/user/fixture/chat-attachments/room/voice.m4a?id=${i}&generation=${generation}` }] })), posts: [], members: [], items: [], ai_members: [] });
    return true;
  }
  if (p.includes('/chat-attachments/')) {
    ranges.push(req.headers.range || 'full');
    if (fail) { res.writeHead(503); res.end(); return true; }
    if (backend) {
      const request = http.request({ hostname: '127.0.0.1', port: Number(backend), path: '/voice.m4a', method: req.method, headers: req.headers }, upstream => {
        res.writeHead(upstream.statusCode, upstream.headers); upstream.pipe(res);
      });
      request.on('error', () => { res.writeHead(502); res.end(); }); request.end(); return true;
    }
    const match = /^bytes=(\d+)-(\d*)$/.exec(req.headers.range || '');
    const start = match ? Number(match[1]) : 0, end = match && match[2] ? Math.min(Number(match[2]), tone.length - 1) : tone.length - 1;
    if (start >= tone.length) { res.writeHead(416, { 'content-range': `bytes */${tone.length}` }); res.end(); return true; }
    res.writeHead(match ? 206 : 200, { 'content-type': 'audio/mp4', 'accept-ranges': 'bytes',
      'content-length': end - start + 1, ...(match ? { 'content-range': `bytes ${start}-${end}/${tone.length}` } : {}) });
    res.end(req.method === 'HEAD' ? undefined : tone.subarray(start, end + 1)); return true;
  }
  return false;
} });

async function main() {
  const origin = await fixture.listen(), output = path.resolve('.ai-tmp/voice-playback'); fs.mkdirSync(output, { recursive: true });
  if (process.env.PWA_VOICE_PREVIEW === '1') { console.log(JSON.stringify({ origin, fixture: 'synthetic', backend: !!backend })); return; }
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
    await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
    const page = await context.newPage(); page.setDefaultTimeout(10000); page.on('pageerror', e => errors.push(e.message));
    await page.goto(origin + '/?fixture=login'); await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only'); await page.locator('#loginBtn').tap();
    await page.locator('#appView:not(.hidden)').waitFor(); await page.waitForLoadState('networkidle');
    const audio = () => page.locator('#chatList audio'), buttons = () => page.locator('#chatList .voice-message-play');
    async function enter() {
      await page.waitForLoadState('networkidle'); await page.goto(origin + '/?fixture=login', { waitUntil: 'networkidle' });
      await page.locator('.conversation-item').filter({ hasText: '语音播放验证群' }).tap(); await buttons().first().waitFor();
    }
    async function playing(index = 0) {
      await buttons().nth(index).tap();
      await page.waitForFunction(i => { const player = document.querySelectorAll('#chatList audio')[i]; return !player.paused && player.currentTime > .1; }, index);
    }
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 390, 430]) {
        await page.setViewportSize({ width, height: 844 }); await enter();
        assert.equal(await audio().first().getAttribute('controls'), null);
        assert.equal(await buttons().first().innerText(), '2″');
        assert.equal(await page.locator('#chatList .voice-message-options a').first().isVisible(), false);
        const metrics = await buttons().first().boundingBox(); assert(metrics.width >= 112 && metrics.height >= 48);
        assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
        await playing(); assert.equal(await buttons().first().getAttribute('data-state'), 'playing');
        await buttons().first().tap(); assert.equal(await audio().first().evaluate(el => el.paused), true);
        await playing(); await playing(1);
        assert.equal(await audio().first().evaluate(el => el.paused), true, 'only one voice plays');
        await page.waitForFunction(() => document.querySelectorAll('#chatList audio')[1].ended && document.querySelectorAll('#chatList .voice-message-play')[1].dataset.state === 'ready');
        assert.equal(await buttons().nth(1).getAttribute('data-state'), 'ready');
        await playing(1); await buttons().nth(1).tap();
        await page.locator('#chatList .voice-message-options summary').first().tap();
        const link = page.locator('#chatList .voice-message-options a').first(); assert.equal(await link.isVisible(), true);
        assert.equal(await link.getAttribute('download'), '合成语音.m4a'); await page.locator('#chatList .voice-message-options summary').first().tap();
        if (width === 390) await page.screenshot({ path: path.join(output, `${engine}-${theme}.png`) });
        cases.push(`${theme}-${width}-aac-play-pause-switch-replay-download`);
      }
    }
    generation++; await enter(); fail = true; await buttons().first().tap(); await page.locator('.voice-message-play[data-state="error"]').waitFor();
    assert.match(await page.locator('.voice-message-status').first().innerText(), /加载失败/);
    fail = false; await playing(); await buttons().first().tap(); cases.push('network-error-retry');
    await playing();
    await page.locator('#chatList .voice-message-player').first().evaluate(el => { window.removedVoice = el.querySelector('audio'); el.remove(); });
    await page.waitForFunction(() => window.removedVoice.paused && !window.removedVoice.hasAttribute('src'));
    cases.push('removed-message-stops-and-releases');
    await enter(); await playing();
    await page.locator('#backBtn').tap(); await page.waitForFunction(() => [...document.querySelectorAll('audio')].every(el => el.paused));
    cases.push('leave-chat-stops');
    const response = await context.request.get(origin + '/api/user/fixture/chat-attachments/room/voice.m4a', { headers: { Range: 'bytes=0-1' } });
    assert.equal(response.status(), 206); assert.equal(response.headers()['content-type'], 'audio/mp4');
    assert.equal((await response.body()).length, 2); cases.push('range-probe');
    assert.deepEqual(errors, []); assert.equal(fixture.audit.rejectedWrites, 0);
    const receipt = { status: 'passed', engine, cases, ranges: [...new Set(ranges)], actualRustHandler: !!backend, productionWrites: false, physicalIphoneVerified: false };
    fs.writeFileSync(path.join(output, `${engine}.json`), JSON.stringify(receipt, null, 2)); console.log(JSON.stringify(receipt));
  } finally { await browser.close(); await fixture.close(); }
}
main().catch(error => { console.error(error); process.exitCode = 1; void fixture.close(); });
