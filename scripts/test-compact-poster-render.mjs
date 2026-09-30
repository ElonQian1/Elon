import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const output = path.resolve('.ai-tmp/compact-posters'); await mkdir(output, { recursive: true });
const js = await readFile(new URL('../server/src/assets/social_links.js', import.meta.url), 'utf8');
const css = await readFile(new URL('../server/src/assets/social_links.css', import.meta.url), 'utf8');
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
  for (const [width, dpr] of [[360, 3], [1280, 1]]) {
    const page = await browser.newPage({ viewport: { width, height: 900 }, deviceScaleFactor: dpr });
    await page.setContent('<style>body{margin:16px;background:#181a1d;color:white;font-family:Arial}main{display:flex;flex-wrap:wrap;gap:16px;align-items:start}.bubble{max-width:calc(100vw - 64px)}</style><main></main>');
    await page.addStyleTag({ content: css }); await page.addScriptTag({ content: js });
    await page.evaluate(() => {
      for (const [w, h] of [[480, 640], [900, 1600], [192, 256], [640, 640], [1280, 720], [32, 64]]) {
        const canvas = document.createElement('canvas'); canvas.width = w; canvas.height = h;
        const ctx = canvas.getContext('2d'); ctx.fillStyle = '#347466'; ctx.fillRect(0, 0, w, h);
        ctx.fillStyle = '#ffffff'; ctx.font = `${Math.max(12, w / 12)}px Arial`; ctx.fillText(`${w} x ${h}`, w / 10, h / 2);
        const node = document.createElement('div'); node.className = 'bubble social-link-only'; document.querySelector('main').append(node);
        const url = `https://weixin.qq.com/sph/fixture${w}x${h}`;
        ElonSocialLinks.mount(node, url, { compact: true, api: async () => ({ schema: 1, url, title: 'Poster', author: 'LongCreatorNameWithoutSpaces'.repeat(5), status: 'ready', image: canvas.toDataURL('image/jpeg', .72) }) });
      }
    });
    for (const card of await page.locator('.social-link-card').all()) {
      await card.scrollIntoViewIfNeeded();
      await card.locator('img.social-link-cover').waitFor({ state: 'visible' });
      await page.waitForFunction(el => el.dataset.cover === 'ready', await card.elementHandle());
    }
    await page.evaluate(() => scrollTo(0, 0));
    const values = await page.evaluate(() => [...document.querySelectorAll('.social-link-wrap')].map(w => {
      const card = w.querySelector('.social-link-card').getBoundingClientRect(), media = w.querySelector('.social-link-media').getBoundingClientRect();
      const image = w.querySelector('img.social-link-cover'), poster = image.getBoundingClientRect(), play = w.querySelector('.social-link-play').getBoundingClientRect();
      return { width: card.width, height: media.height, ratio: media.width / media.height, intrinsicRatio: image.naturalWidth / image.naturalHeight, tiny: image.naturalWidth < 100, posterWidth: poster.width, posterHeight: poster.height, playContained: play.top >= media.top && play.bottom <= media.bottom && play.right <= media.right, cardOverflow: card.right > innerWidth };
    }));
    for (const v of values) {
      assert.ok(v.width <= 280 && v.height <= 281); assert.equal(v.cardOverflow, false); assert.equal(v.playContained, true);
      assert.ok(Math.abs(v.ratio - Math.max(.5, Math.min(2, v.intrinsicRatio))) < .01);
      if (v.tiny) assert.ok(v.posterWidth <= 32 && v.posterHeight <= 64, 'tiny source is not enlarged');
    }
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    await page.screenshot({ path: path.join(output, `posters-${width}-${dpr}.png`), fullPage: true });
    await page.close(); console.log(`PASS: ${width}px DPR${dpr}, six cover sizes, no stretching/overflow, controls remain visible`);
  }
} finally { await browser.close(); }
