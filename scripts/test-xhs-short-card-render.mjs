// Render public previews exported by the Rust xhs_cn_live_reported_cards acceptance test.
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const previews = JSON.parse(await readFile(process.argv[2] || '.ai-tmp/xhs-cards.json', 'utf8'));
assert.equal(previews.length, 2);
for (const p of previews) {
  assert.equal(p.site, '小红书'); assert.equal(p.status, 'ready');
  assert.ok(p.title && p.author && p.cover_data_url);
}
const assets = new URL('../server/src/assets/', import.meta.url);
const js = await readFile(new URL('social_links.js', assets), 'utf8');
const css = await readFile(new URL('social_links.css', assets), 'utf8');
const output = path.resolve('.ai-tmp/xhs-cards'); await mkdir(output, { recursive: true });
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
  const page = await browser.newPage();
  const errors = []; page.on('pageerror', e => errors.push(e.message));
  for (const width of [1280, 360]) {
    await page.setViewportSize({ width, height: 960 });
    await page.setContent('<style>body{margin:20px;background:#232427;color:#eee;font-family:Arial,sans-serif}main{display:flex;align-items:start;flex-wrap:wrap;gap:24px}section{max-width:100%}</style><main><section id="first"></section><section id="second"></section></main>');
    await page.addStyleTag({ content: css }); await page.addScriptTag({ content: js });
    await page.evaluate(previews => {
      globalThis.opens = [];
      for (const [i, p] of previews.entries()) ElonSocialLinks.mount(document.getElementById(i ? 'second' : 'first'), p.url, {
        desktop: innerWidth > 600, owner: 'public-xhs-acceptance',
        api: async () => p, open: value => opens.push(value.url),
      });
    }, previews);
    await page.waitForFunction(() => document.querySelectorAll('.social-link-poster[data-cover="ready"]').length === 2);
    for (const [i, id] of ['first', 'second'].entries()) {
      assert.equal(await page.locator(`#${id} .social-link-title`).textContent(), previews[i].title);
      assert.ok((await page.locator(`#${id} .social-link-source`).textContent()).includes(previews[i].author));
      const bounds = await page.locator(`#${id}`).evaluate(node => {
        const rect = selector => { const r = node.querySelector(selector).getBoundingClientRect(); return { top: r.top, bottom: r.bottom, width: r.width }; };
        const image = node.querySelector('.social-link-cover');
        return { media: rect('.social-link-media'), title: rect('.social-link-title'), footer: rect('.social-link-video-footer'), loaded: image.naturalWidth > 0 };
      });
      assert.ok(bounds.loaded && bounds.media.width >= 144 && bounds.media.width <= 280);
      assert.ok(bounds.title.top >= bounds.media.bottom && bounds.footer.top >= bounds.title.bottom);
      await page.locator(`#${id} .social-link-card`).click();
    }
    assert.deepEqual(await page.evaluate(() => opens), previews.map(p => p.url));
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    await page.screenshot({ path: path.join(output, `xhs-${width}.png`), fullPage: true });
    await page.locator('#first .social-link-cover').evaluate(img => img.dispatchEvent(new Event('error')));
    assert.equal(await page.locator('#first .social-link-card').getAttribute('data-cover'), 'missing');
    assert.equal(await page.locator('#first .social-link-title').textContent(), previews[0].title);
    assert.ok(await page.locator('#first .social-link-retry').isVisible());
    console.log(`PASS ${width}: both reported covers, titles, authors, original-link actions, retry and no overflow`);
  }
  assert.deepEqual(errors, []);
} finally { await browser.close(); }
