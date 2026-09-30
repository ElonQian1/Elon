import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { webkit } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const address = new URL(process.argv[2] || 'http://43.139.149.158:8080/web');
assert.ok(['http:', 'https:'].includes(address.protocol));
address.searchParams.set('media_asset_check', Date.now().toString());
const response = await fetch(address, { signal: AbortSignal.timeout(15000) });
assert.equal(response.status, 200);
assert.equal(response.headers.get('x-elon-mobile-pwa-source'), 'runtime');
const html = await response.text();
const browser = await webkit.launch({ headless: true });
try {
  const page = await browser.newPage();
  // Parse the live shell without executing its application scripts or logging in.
  const embedded = await page.evaluate(text => {
    const document = new DOMParser().parseFromString(text, 'text/html');
    const names = ['social_links.js', 'social_links.css', 'social_wechat_handoff.js'];
    return names.map(name => {
      const nodes = document.querySelectorAll(`[data-elon-runtime-asset="/assets/${name}"]`);
      return { name, count: nodes.length, content: nodes[0]?.textContent };
    });
  }, html);
  for (const item of embedded) {
    assert.equal(item.count, 1, `${item.name}: expected one embedded runtime asset`);
    let expected = await readFile(new URL(`../server/src/assets/${item.name}`, import.meta.url), 'utf8');
    if (item.name.endsWith('.js')) expected = expected.replace(/<\/script/gi, '<\\/script');
    const normalize = value => value.replace(/\r\n/g, '\n').trim();
    assert.ok(normalize(item.content) === normalize(expected), `${item.name}: stale inline PWA generation`);
    console.log(`PASS: ${item.name} matches the live PWA inline generation`);
  }
  console.log(`PASS: live runtime at ${address.origin}${address.pathname}; external WeChat return is not tested`);
} finally {
  await browser.close();
}
