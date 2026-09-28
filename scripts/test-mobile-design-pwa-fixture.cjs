// Real-browser smoke of the offline production-page fixture; capture remains a workbench task.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');

(async () => {
  const externalOrigin = process.argv.find(arg => arg.startsWith('--origin='))?.slice(9);
  const fixture = externalOrigin ? null : createFixture();
  const origin = externalOrigin || await fixture.listen();
  const parsed = new URL(origin);
  let browser;
  const results = [], errors = [], apiPaths = new Set();
  try {
    assert.equal(parsed.hostname, '127.0.0.1');
    assert.equal(parsed.protocol, 'http:');
    assert.equal(parsed.origin, origin);
    assert.deepEqual(await (await fetch(origin + '/fixture/info')).json(),
      { schema: 'elon.mobile_design_fixture.v1', synthetic: true, productionNetwork: false });
    assert(!/__\w+_PNG_B64__/.test(await (await fetch(origin + '/?fixture=login')).text()),
      'fixture must render every production PNG placeholder');
    for (const asset of ['ic_project_members_toolbar.png', 'ic_side_menu_folder_closed.png']) {
      const response = await fetch(origin + '/assets/' + asset);
      assert.equal(response.status, 200, asset);
      assert.equal(response.headers.get('content-type'), 'image/png');
      assert.equal(Buffer.from(await response.arrayBuffer()).subarray(0, 8).toString('hex'), '89504e470d0a1a0a');
    }
    browser = await chromium.launch({ channel: process.env.BROWSER_CHANNEL || 'msedge', headless: true });
    assert.equal((await fetch(origin + '/api/me')).status, 401);
    assert.equal((await fetch(origin + '/api/auth/login', { method: 'POST', body: JSON.stringify({ account: 'wrong', password: 'wrong' }) })).status, 401);
    assert.equal((await fetch(origin + '/api/auth/login', { method: 'POST', headers: { Origin: 'https://outside.invalid' }, body: '{}' })).status, 403);
    const context = await browser.newContext({ viewport: { width: 411, height: 842 }, isMobile: true, hasTouch: true });
    await context.route('**/*', route => {
      if (new URL(route.request().url()).origin === origin) return route.continue();
      return route.abort();
    });
    const page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));
    page.on('request', request => { const p = new URL(request.url()).pathname; if (p.startsWith('/api/')) apiPaths.add(p); });
    page.setDefaultTimeout(10000);
    await page.goto(origin + '/?fixture=login');
    await page.locator('#loginView').waitFor({ state: 'visible' });
    assert.equal(await page.locator('#loginBtn').textContent(), '登录');
    await page.goto(origin + '/?fixture=register&auth=register');
    await page.locator('#nicknameInput').waitFor({ state: 'visible' });
    assert.equal(await page.locator('#loginBtn').textContent(), '创建账号');
    results.push('public-login-register');
    await page.goto(origin + '/?fixture=projects&tab=projects');
    await page.locator('#accountInput').fill('mobile-v2-fixture');
    await page.locator('#passwordInput').fill('offline-fixture-only');
    await page.locator('#loginBtn').click();
    await page.locator('#appView:not(.hidden)').waitFor();
    const token = await page.evaluate(() => localStorage.getItem('lodex_token'));
    assert.match(token, /^fixture-[a-f0-9]{48}$/);
    const auth = { Authorization: 'Bearer ' + token };
    assert.equal((await fetch(origin + '/api/not-declared', { headers: auth })).status, 404);
    for (const [method, api] of [['PUT', '/api/auth/password'], ['POST', '/api/auth/sessions/revoke-others'], ['DELETE', '/api/projects/preview-work']]) {
      assert.equal((await fetch(origin + api, { method, headers: auth, body: '{}' })).status, 405);
    }
    results.push('actual-fixture-login-and-write-denial');
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      await page.goto(origin + '/?fixture=projects&tab=projects');
      await page.locator('.project-browser-item[data-project-browser-index="1"]').waitFor({ state: 'visible' });
      const projectTitle = await page.locator('.project-browser-item[data-project-browser-index="1"]').innerText();
      assert(projectTitle.includes('一个较长的项目名称，用于检查换行和大字体'));
      results.push('projects-long-title-' + theme);
      await page.goto(origin + '/?fixture=empty&tab=projects');
      await page.locator('#projectBrowserBody').getByText('暂无个人项目', { exact: true }).waitFor({ state: 'visible' });
      assert.equal(await page.locator('.project-browser-item').count(), 0);
      results.push('projects-empty-' + theme);
      await page.goto(origin + '/?fixture=chat_result&tab=projects');
      await page.locator('#chatList .bubble.ai').filter({ hasText: '这是离线布局示例' }).waitFor({ state: 'visible' });
      const result = await page.locator('#chatList').innerText();
      assert(result.includes('构建结果：成功'));
      assert(result.includes('不代表实际任务已完成'));
      results.push('project-to-result-' + theme);
      await page.goto(origin + '/?fixture=account_security&tab=profile');
      await page.locator('#accountIdentityMask.active').waitFor({ state: 'visible' });
      await page.locator('#accountSecuritySummary').filter({ hasText: '密码已启用' }).waitFor();
      await page.locator('#accountSessionList').filter({ hasText: 'Web 演示设备' }).waitFor();
      assert.equal(await page.locator('#accountGoogleBindingState').textContent(), '暂未配置');
      assert.equal(await page.locator('#accountRevokeOthers').isDisabled(), true);
      results.push('account-security-' + theme);
    }
    const layoutFailures = [];
    for (const theme of ['light', 'dark']) {
      await page.evaluate(value => localStorage.setItem('elon.mobile.appearance.v2', value), theme);
      for (const width of [320, 411, 720]) {
        await page.setViewportSize({ width, height: 842 });
        await page.goto(origin + '/?fixture=chat_result&tab=projects');
        await page.locator('#chatList .bubble.ai').filter({ hasText: '这是离线布局示例' }).waitFor();
        const layout = await page.evaluate(() => {
          const app = document.querySelector('#appView'), toolbar = app.querySelector('.toolbar');
          const rect = element => element.getBoundingClientRect();
          return { appWidth: rect(app).width, toolbarWidth: rect(toolbar).width,
            contentWidth: rect(app.querySelector('.content')).width,
            viewportWidth: innerWidth, scrollWidth: document.documentElement.scrollWidth,
            toolbarColor: getComputedStyle(toolbar).backgroundColor,
            surfaceColor: getComputedStyle(app).backgroundColor,
            misplacedProjectAction: !!document.querySelector('#projectSpaceAiMenu').getClientRects().length,
            brokenImages: [...document.images].filter(image => {
              const r = rect(image);
              return r.width && r.height && r.x < innerWidth && r.right > 0 && r.y < innerHeight && r.bottom > 0 &&
                getComputedStyle(image).visibility !== 'hidden' && (!image.complete || !image.naturalWidth);
            }).map(image => image.getAttribute('src')) };
        });
        if (Math.abs(layout.appWidth - layout.toolbarWidth) > 1 ||
            Math.abs(layout.appWidth - layout.contentWidth) > 1 ||
            layout.scrollWidth > layout.viewportWidth + 1 ||
            layout.toolbarColor !== layout.surfaceColor || layout.misplacedProjectAction || layout.brokenImages.length) {
          layoutFailures.push({ theme, width, ...layout });
        }
        results.push('chat-layout-and-images-' + theme + '-' + width);
      }
    }
    assert.deepEqual(layoutFailures, [], 'chat must use the full surface and theme without hidden-page actions or broken images');
    assert.deepEqual(errors, []);
    if (process.argv.includes('--prepare-profile')) {
      assert(externalOrigin, 'profile requires a separately running fixture so its session remains valid');
      const directory = path.resolve(__dirname, '../.elon/ui-tuner/pwa-sessions');
      fs.mkdirSync(directory, { recursive: true });
      fs.writeFileSync(path.join(directory, 'mobile-v2-offline.json'), JSON.stringify({ version: 1,
        localStorage: { lodex_token: token } }) + '\n', { mode: 0o600 });
      results.push('local-auth-profile-prepared');
    }
    await context.close();
    console.log(JSON.stringify({ schema: 'elon.mobile_design_fixture_test.v1', status: 'passed',
      synthetic: true, productionAuthVerified: false, writesPerformed: false, scenarios: results,
      requestedApiPaths: [...apiPaths].sort() }, null, 2));
  } finally { if (browser) await browser.close(); if (fixture) await fixture.close(); }
})().catch(error => { console.error(error.stack); process.exitCode = 1; });
