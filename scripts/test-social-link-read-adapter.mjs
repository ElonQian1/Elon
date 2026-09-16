import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const script = readFileSync(new URL('../android/app/src/main/assets/social_link_read_adapter.js', import.meta.url), 'utf8');
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) });
let count = 0;
try {
  const page = await browser.newPage();
  let markup = '';
  await page.route('**/*', r => r.request().resourceType() === 'document' ? r.fulfill({ contentType: 'text/html; charset=utf-8', body: markup }) : r.abort());
  const original = 'https://app.binance.com/uni-qr/cpos/123456?r=synthetic';
  async function read(url, html, source = url) {
    markup = html; await page.goto(url); await page.addScriptTag({ content: script });
    return page.evaluate(original => ElonSocialReadAdapter.read(original), source);
  }
  let result = await read('https://www.binance.com/en/square/post/123456', '<meta property="og:title" content="Public article | Binance Square"><meta property="og:image" content="https://public.bnbstatic.com/image/cms/content.png"><main><h1>Public article</h1><p>Visible text</p></main>', original);
  assert.equal(result.title, 'Public article'); assert.ok(result.image.endsWith('content.png')); count++;
  assert.equal(await page.evaluate(u => ElonSocialReadAdapter.readingUrl(u), original), 'https://www.binance.com/en/square/post/123456?r=synthetic'); count++;
  assert.equal(await read('https://www.binance.com/en/square/post/999999', '<main><h1>Other post</h1></main>', original), null); count++;
  assert.equal(await read('https://www.binance.com/en/square/post/123456', '<main><h1>Sign in to Binance</h1></main>', original), null); count++;
  assert.equal(await read('https://www.binance.com/en/square/post/123456', '<meta property="og:title" content="A stale title"><main>Loading…</main>', original), null); count++;
  result = await read('https://www.binance.com/en/square/post/123456', '<meta property="og:description" content="A text-only short post"><main>A text-only short post</main>', original);
  assert.equal(result.title, 'A text-only short post'); assert.equal(result.image, null); count++;
  const x = 'https://x.com/author/status/123456';
  result = await read(x, '<article><a href="/someone/status/999999"><time>Today</time></a><div data-testid="tweetText">Wrong recommendation</div></article><article><div data-testid="User-Name">An author</div><a href="/author/status/123456"><time>Today</time></a><div data-testid="tweetText">Public post excerpt</div><div data-testid="tweetPhoto"><img src="https://pbs.twimg.com/media/fixture.jpg"></div></article>');
  assert.equal(result.title, 'Public post excerpt'); assert.equal(result.image, 'https://pbs.twimg.com/media/fixture.jpg'); count++;
  assert.equal(await read(x, '<article><a href="/author/status/123456"><time>Today</time></a><div role="link"><div data-testid="tweetText">Quoted content</div></div></article>'), null); count++;
  assert.equal(await read(x, '<main><h1>Log in to X</h1></main>'), null); count++;
  const article = 'https://x.com/i/article/123456';
  result = await read(article, '<meta property="og:image" content="https://pbs.twimg.com/profile_images/a/avatar.jpg"><main><h1>A long-form article</h1></main>');
  assert.equal(result.title, 'A long-form article'); assert.equal(result.image, null); count++;
  result = await read('https://mp.weixin.qq.com/s/fixture', '<h1 id="activity-name">公开文章</h1><div id="js_name">公众号作者</div><div id="js_content">正文</div><meta property="og:image" content="http://mmbiz.qpic.cn/mmbiz_png/fixture">');
  assert.equal(result.title, '公开文章'); assert.match(result.image, /^https:/); count++;
  assert.equal(await read(x, '<main>Loading…</main>'), null);
  // A later read observes asynchronously rendered content; no stale empty result is cached.
  await page.evaluate(() => { document.body.innerHTML = '<article><a href="/author/status/123456"><time>Today</time></a><div data-testid="tweetText">Late content</div></article>'; });
  assert.equal((await page.evaluate(u => ElonSocialReadAdapter.read(u), x)).title, 'Late content'); count++;
  console.log(JSON.stringify({ cases: count, passed: true, adapter: 'production', fixtureOnly: true, officialSitesVerified: false }));
} finally { await browser.close(); }
