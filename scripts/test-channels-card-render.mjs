// Opt-in public sample: render production shared cards, not a separate mock design.
import assert from 'node:assert/strict';
import { mkdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const output = path.resolve(process.argv[2] || '.ai-tmp/channels-cards');
await mkdir(output, { recursive: true });
const url = 'https://weixin.qq.com/sph/Aur6t4pfk3';
const response = await fetch('https://channels.weixin.qq.com/finder-preview/api/feed/get_feed_info?_pageUrl=https%3A%2F%2Fchannels.weixin.qq.com%2Ffinder-preview%2Fpages%2Fsph', {
  method: 'POST', headers: { 'Content-Type': 'application/json', Origin: 'https://channels.weixin.qq.com', Referer: 'https://channels.weixin.qq.com/finder-preview/pages/sph?id=Aur6t4pfk3' },
  body: JSON.stringify({ baseReq: { generalToken: '' }, shortUri: 'Aur6t4pfk3' }), signal: AbortSignal.timeout(15000),
});
assert.ok(response.ok); const data = await response.json(); assert.equal(data.errCode, 0);
async function raster(source) {
  const result = await fetch(source, { signal: AbortSignal.timeout(15000) }); assert.ok(result.ok);
  const mime = result.headers.get('content-type').split(';')[0].replace('image/jpg', 'image/jpeg'); assert.match(mime, /^image\/(jpeg|png|webp)$/);
  const bytes = Buffer.from(await result.arrayBuffer()); assert.ok(bytes.length < 1048576);
  return `data:${mime};base64,${bytes.toString('base64')}`;
}
const poster = await raster(data.data.feedInfo.coverUrl);
const avatar = await raster(data.data.authorInfo.headImgUrl);
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
  const page = await browser.newPage();
  const css = await readFile(new URL('../server/src/assets/social_links.css', import.meta.url), 'utf8');
  const js = await readFile(new URL('../server/src/assets/social_links.js', import.meta.url), 'utf8');
  for (const width of [1280, 360]) {
    await page.setViewportSize({ width, height: 740 });
    await page.setContent('<style>body{background:#101112;color:white;font-family:Arial,sans-serif;margin:24px}#host{width:220px;max-width:100%}</style><div id="host"></div>');
    await page.addStyleTag({ content: css }); await page.addScriptTag({ content: js });
    await page.evaluate(({ url, poster, avatar, data }) => {
      // Reduce public raster to the same 64px inline avatar contract as the server.
      const image = new Image(); image.src = avatar;
      return image.decode().then(() => {
        const canvas = document.createElement('canvas'); canvas.width = canvas.height = 64;
        canvas.getContext('2d').drawImage(image, 0, 0, 64, 64);
        globalThis.opens = 0;
        ElonSocialLinks.mount(document.querySelector('#host'), url, { channelsHandoff: true,
          api: async () => ({ schema: 1, url, title: data.data.feedInfo.description, author: data.data.authorInfo.nickname, cover_data_url: poster, image: null,
            author_avatar_data_url: canvas.toDataURL('image/jpeg', 0.72), status: 'ready' }), open: () => { globalThis.opens++; } });
      });
    }, { url, poster, avatar, data });
    await page.locator('.social-link-avatar img').waitFor({ state: 'visible' });
    await page.waitForFunction(() => [...document.querySelectorAll('img:not([hidden])')].every(i => i.complete && i.naturalWidth > 0));
    const bounds = await page.evaluate(() => {
      const box = s => { const r = document.querySelector(s).getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height, right: r.right, bottom: r.bottom }; };
      const image = document.querySelector('.social-link-cover');
      return { ratio: image.naturalWidth / image.naturalHeight, card: box('.social-link-card'), media: box('.social-link-media'), footer: box('.social-link-video-footer'), play: box('.social-link-play'), author: box('.social-link-source'), overflow: document.documentElement.scrollWidth > innerWidth };
    });
    assert.ok(Math.abs(bounds.media.width / bounds.media.height - Math.max(.5, Math.min(2, bounds.ratio))) < 0.01);
    assert.ok(bounds.media.height <= 281 && bounds.media.width <= 220);
    assert.ok(bounds.footer.y >= bounds.media.bottom); assert.equal(bounds.overflow, false);
    assert.ok(bounds.play.y > bounds.media.y && bounds.play.bottom < bounds.media.bottom);
    assert.ok(bounds.author.right <= bounds.card.right);
    await page.screenshot({ path: path.join(output, `channels-${width}.png`) });
    await page.locator('.social-link-play').click(); assert.equal(await page.evaluate(() => opens), 1);
    await page.locator('.social-link-card').focus(); await page.keyboard.press('Enter'); assert.equal(await page.evaluate(() => opens), 2);
    await page.evaluate(() => {
      const image = document.querySelector('.social-link-avatar img'); image.dispatchEvent(new Event('error'));
      document.querySelector('.social-link-source').textContent = 'VeryLongCreatorNameWithoutSpaces'.repeat(5);
    });
    assert.equal(await page.locator('.social-link-avatar img').isVisible(), false);
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
    console.log(`PASS channels ${width}: public poster/avatar, play, footer, keyboard, fallback, long name`);
  }
} finally { await browser.close(); }
