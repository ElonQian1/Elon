import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const script = readFileSync(new URL('../android/app/src/main/assets/social_link_performance.js', import.meta.url), 'utf8');
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) });
let cases = 0;
try {
  const page = await browser.newPage();
  await page.addInitScript(script);
  await page.route('**/*', route => route.request().resourceType() === 'document'
    ? route.fulfill({ contentType: 'text/html', body: '<h1>Fixture</h1><div id="js_content" style="visibility:hidden;opacity:0">private body token=secret</div>' })
    : route.abort());
  await page.goto('https://mp.weixin.qq.com/s/fixture?token=secret');
  let result = await page.evaluate(() => window.__elonLinkPerformance.snapshot());
  assert.equal(result.content_state, 'hidden'); assert.equal(result.content_visible, false); cases++;
  assert.equal(await page.locator('#js_content').getAttribute('style'), 'visibility:hidden;opacity:0'); cases++;
  await page.evaluate(() => { document.querySelector('#js_content').style.cssText = ''; });
  result = await page.evaluate(() => window.__elonLinkPerformance.snapshot());
  assert.equal(result.content_state, 'visible'); assert.equal(result.content_visible, true); cases++;
  assert.ok(result.response_end_ms >= 0); assert.ok(result.load_ms >= 0); cases++;
  await page.evaluate(() => {
    window.dispatchEvent(new ErrorEvent('error', { message: 'private token=secret' }));
    const img = document.createElement('img'); img.src = 'https://res.wx.qq.com/private?token=secret';
    document.body.append(img);
  });
  await page.waitForFunction(() => window.__elonLinkPerformance.snapshot().resource_error_hosts.length === 1);
  result = await page.evaluate(() => window.__elonLinkPerformance.snapshot());
  assert.equal(result.js_errors, 1); assert.deepEqual(result.resource_error_hosts, ['res.wx.qq.com']);
  assert.ok(!JSON.stringify(result).includes('secret')); assert.ok(!JSON.stringify(result).includes('private')); cases++;
  await page.addScriptTag({ content: script });
  assert.equal(await page.evaluate(() => window.__elonLinkPerformance.snapshot().js_errors), 1); cases++;
  await page.evaluate(() => {
    window.__elonLinkPerformance.stop();
    window.dispatchEvent(new ErrorEvent('error', { message: 'after stop' }));
  });
  assert.equal(await page.evaluate(() => window.__elonLinkPerformance.snapshot().js_errors), 1); cases++;
  await page.goto('https://example.org/article');
  await page.waitForFunction(() => window.__elonLinkPerformance.snapshot().content_visible);
  assert.ok(await page.evaluate(() => window.__elonLinkPerformance.snapshot().slow_resources.length <= 8)); cases++;
  await page.evaluate(() => { const frame = document.createElement('iframe'); frame.src = 'https://example.org/frame'; document.body.append(frame); });
  await page.waitForFunction(() => document.querySelector('iframe').contentDocument?.readyState === 'complete');
  assert.equal(await page.frames()[1].evaluate(() => typeof window.__elonLinkPerformance), 'undefined'); cases++;
  console.log(JSON.stringify({ cases, passed: true, fixtureOnly: true, contentExported: false }));
} finally { await browser.close(); }
