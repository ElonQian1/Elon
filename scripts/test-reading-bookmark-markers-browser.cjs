const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const root = path.resolve(__dirname, '..');
const files = { '/': 'scripts/fixtures/reading-bookmark-markers.html',
  '/assets/reading_positions.js': 'server/src/assets/reading_positions.js',
  '/assets/reading_bookmarks_ui.js': 'server/src/assets/reading_bookmarks_ui.js',
  '/assets/reading_bookmarks.css': 'server/src/assets/reading_bookmarks.css' };
const server = http.createServer((req, res) => {
  const file = files[new URL(req.url, 'http://localhost').pathname];
  if (!file) { res.writeHead(404); res.end(); return; }
  res.setHeader('Content-Type', file.endsWith('.js') ? 'text/javascript; charset=utf-8' : file.endsWith('.css') ? 'text/css' : 'text/html; charset=utf-8');
  res.end(fs.readFileSync(path.join(root, file)));
});
server.listen(process.argv.includes('--serve') ? 5187 : 0, '127.0.0.1', async () => {
  const url = `http://127.0.0.1:${server.address().port}/`;
  if (process.argv.includes('--serve')) { console.log(url); return; }
  const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
  const browser = await chromium.launch({ headless: true, channel: 'msedge' });
  try {
    for (const width of [320, 390, 1280]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = []; page.on('pageerror', e => errors.push(e.message));
      await page.goto(url);
      await page.getByRole('img', { name: '书签：周末继续看', exact: true }).waitFor();
      await page.getByRole('img', { name: '书签：待回顾', exact: true }).waitFor();
      async function clearGeometry() {
        const positions = await page.locator('.reading-message-marker').evaluateAll(nodes => nodes.map(n => {
          const r=n.getBoundingClientRect(),b=n.parentElement.querySelector('.bubble').getBoundingClientRect();
          return {outside:r.right<=b.left||r.left>=b.right,inside:r.left>=0&&r.right<=innerWidth};
        }));
        assert.ok(positions.length > 0 && positions.every(p => p.outside && p.inside), `${width}px: marker must stay outside bubble and inside viewport`);
      }
      await clearGeometry();
      await page.getByRole('button', { name:'书签', exact:true }).click();
      await page.locator('section').filter({ hasText:'周末继续看' }).getByRole('button',{name:'继续阅读',exact:true}).click();
      await page.getByRole('img',{name:'续读位置：周末继续看',exact:true}).waitFor();
      assert.equal(await page.locator('[data-message-id="one"] .reading-message-marker').count(),1);
      await clearGeometry();
      const long = await page.getByRole('img',{name:'续读位置：周末继续看'}).boundingBox();
      const viewport = await page.locator('#list').boundingBox();
      assert.ok(long.y >= viewport.y && long.y < viewport.y + viewport.height, 'long bubble resume marker stays visible');
      await page.evaluate(() => fixtureRows());
      await page.getByRole('img',{name:'书签：待回顾',exact:true}).waitFor();
      await page.evaluate(() => ui.latest());
      await page.getByRole('img',{name:'续读位置：周末继续看',exact:true}).waitFor({state:'detached'});
      assert.equal(await page.getByRole('img',{name:'续读位置：周末继续看',exact:true}).count(),0);
      await page.getByRole('button', {name:'书签',exact:true}).click();
      await page.locator('section').filter({hasText:'待回顾'}).getByRole('button',{name:'删除',exact:true}).click();
      await page.waitForFunction(() => !document.querySelector('[data-message-id="two"] .reading-message-marker'));
      await page.getByRole('button',{name:'撤销刚才的删除',exact:true}).click();
      await page.getByRole('img',{name:'书签：待回顾',exact:true}).waitFor();
      await page.getByRole('button',{name:'关闭',exact:true}).click();
      assert.equal(await page.locator('[data-message-id="one"] .bubble').textContent(),'这是原始标记。继续读到其他消息后，这里的书签仍然保留。');
      await page.evaluate(() => ui.close());
      assert.equal(await page.locator('.reading-message-marker').count(),0);
      assert.deepEqual(errors,[]);
      await page.close();
    }
    console.log('PASS: bookmark markers at 320/390/1280px, both directions, independent resume, long messages, recycled DOM, delete/undo and cleanup');
  } catch(error) {console.error(error);process.exitCode=1;} finally {await browser.close();server.close();}
});
