// Public Bilibili poster plus unavailable-provider states, rendered by the production module.
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const output = path.resolve(process.argv[2] || '.ai-tmp/media-cards');
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
  const page = await browser.newPage();
  const url = 'https://www.bilibili.com/video/BV19eYH6NEsC/';
  const response = await page.request.get(url, { timeout: 15000 });
  assert.ok(response.ok(), `Public sample HTTP ${response.status()}`);
  const head = await response.text();
  const metadata = await page.evaluate(html => {
    const doc = new DOMParser().parseFromString(html, 'text/html');
    return { image: doc.querySelector('meta[property="og:image"]')?.content, title: doc.querySelector('meta[property="og:title"]')?.content };
  }, head);
  assert.ok(metadata.image && metadata.title);
  const imageUrl = new URL(metadata.image, url); imageUrl.protocol = 'https:';
  const imageResponse = await page.request.get(imageUrl.href, { timeout: 15000 });
  assert.ok(imageResponse.ok());
  const bytes = await imageResponse.body(); assert.ok(bytes.length <= 1048576);
  const mime = imageResponse.headers()['content-type'].split(';')[0];
  assert.match(mime, /^image\/(jpeg|png|webp)$/);
  const raster = `data:${mime};base64,${bytes.toString('base64')}`;
  const js = await readFile(new URL('../server/src/assets/social_links.js', import.meta.url), 'utf8');
  const css = await readFile(new URL('../server/src/assets/social_links.css', import.meta.url), 'utf8');
  for (const width of [1280, 360]) {
    await page.setViewportSize({ width, height: 940 });
    await page.setContent('<style>body{background:#101112;color:white;font-family:Arial,sans-serif;margin:20px}main{display:flex;gap:24px;flex-wrap:wrap;align-items:start}section{max-width:100%}</style><main><section id="bili"></section><section id="douyin"></section><section id="note"></section></main>');
    await page.addStyleTag({ content: css }); await page.addScriptTag({ content: js });
    await page.evaluate(async ({ url, metadata, raster }) => {
      const img = new Image(); img.src = raster; await img.decode();
      const canvas = document.createElement('canvas'); canvas.width = 480; canvas.height = Math.round(480 * img.height / img.width);
      canvas.getContext('2d').drawImage(img, 0, 0, canvas.width, canvas.height);
      const poster = canvas.toDataURL('image/jpeg', 0.7);
      globalThis.opens = [];
      for (const [id, link, result] of [
        ['bili', url, { title: metadata.title, cover_data_url: poster, status: 'ready' }],
        ['douyin', 'https://v.douyin.com/_XMEsxVKKOY/', { title: '抖音视频', status: 'unavailable' }],
        ['note', 'https://www.xiaohongshu.com/discovery/item/6a6e8951000000002402c81f', { title: '小红书笔记', status: 'unavailable' }],
      ]) ElonSocialLinks.mount(document.getElementById(id), link, { desktop: innerWidth > 600, api: async () => ({ schema: 1, url: link, ...result }), open: p => opens.push(p.url) });
    }, { url, metadata, raster });
    await page.waitForFunction(() => document.querySelector('#bili .social-link-card').dataset.cover === 'ready');
    assert.equal(await page.locator('.social-link-poster').count(), 3);
    assert.equal(await page.locator('#note .social-link-play').textContent(), '↗');
    for (const id of ['bili', 'douyin', 'note']) {
      const bounds = await page.locator(`#${id}`).evaluate(node => {
        const r = selector => { const b = node.querySelector(selector).getBoundingClientRect(); return { top: b.top, bottom: b.bottom, width: b.width, height: b.height, right: b.right }; };
        return { media: r('.social-link-media'), title: r('.social-link-title'), footer: r('.social-link-video-footer'), play: r('.social-link-play') };
      });
      assert.ok(bounds.media.width >= 144 && bounds.media.width <= 280);
      assert.ok(bounds.title.top >= bounds.media.bottom && bounds.footer.top >= bounds.title.bottom);
      assert.ok(bounds.play.top > bounds.media.top && bounds.play.bottom < bounds.media.bottom);
      if (id !== 'bili') assert.equal(bounds.media.height, 100, 'missing covers stay compact');
      await page.locator(`#${id} .social-link-play`).click();
    }
    assert.equal(await page.evaluate(() => opens.length), 3);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    await page.screenshot({ path: path.join(output, `media-${width}.png`), fullPage: true });
    await page.locator('#bili .social-link-cover').evaluate(img => img.dispatchEvent(new Event('error')));
    assert.equal(await page.locator('#bili .social-link-card').getAttribute('data-cover'), 'missing');
    console.log(`PASS ${width}: public poster, three media layouts, click targets, fallback, no overflow`);
  }
} finally { await browser.close(); }
