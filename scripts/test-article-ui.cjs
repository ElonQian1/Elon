// Deterministic browser contract tests; all content and accounts are synthetic.
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const { pathToFileURL } = require('node:url');
const { chromium } = require('playwright');
const root = path.resolve(__dirname, '..');
const tmp = path.join(root, 'pc-frontend', '.ai-tmp');
const fixture = { id: 'article_fixture', revision: 1, title: '', summary: '', status: 'draft', author_id: 'test_author', author_name: '测试作者', updated_at: '2026-09-15T08:00:00Z', document: { title: '', summary: '', cover: null, blocks: [] }, media: {} };
async function main() {
  await fs.mkdir(tmp, { recursive: true });
  await fs.writeFile(path.join(tmp, 'article-preview.html'), '<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script type="module" src="./article-preview.tsx"></script></body></html>');
  await fs.writeFile(path.join(tmp, 'article-preview.tsx'), `import React from 'react'; import {createRoot} from 'react-dom/client'; import '../src/styles/globals.css'; import ArticleWorkspace from '../src/features/articles/ArticleWorkspace'; import ArticleMessage from '../src/features/articles/ArticleMessage'; createRoot(document.getElementById('root')!).render(<><ArticleWorkspace groupId="g1" groups={[{id:'g1',name:'测试群'}]}/><ArticleMessage content={'【一龙文章】\\n'+JSON.stringify({schema:1,article_id:'article_fixture',revision:1,title:'已发布文章',summary:'卡片摘要'})}/></>);`);
  const { createServer } = await import(pathToFileURL(path.join(root, 'pc-frontend/node_modules/vite/dist/node/index.js')).href);
  const server = await createServer({ configFile: path.join(root, 'pc-frontend/vite.config.ts'), root: path.join(root, 'pc-frontend'), server: { host: '127.0.0.1', port: 0, strictPort: false } });
  let browser;
  try {
    await server.listen(); const base = `http://127.0.0.1:${server.httpServer.address().port}`;
    browser = await chromium.launch({ channel: 'chrome', headless: true });
    for (const mobile of [false, true]) {
      const context = await browser.newContext({ viewport: mobile ? { width: 390, height: 844 } : { width: 1280, height: 900 } });
      const page = await context.newPage(); let a = structuredClone(fixture), publications = 0, failSave = false; const media = {};
      const errors = []; page.on('pageerror', e => { errors.push(e.message); console.error('ARTICLE_PAGE_ERROR=' + e.message); });
      page.on('console', m => { if (m.type() === 'error') console.error('ARTICLE_CONSOLE=' + m.text()); });
      await page.route(/^https?:\/\/[^/]+\/api\//, async route => {
        const req = route.request(), url = new URL(req.url()); const p = url.pathname; const data = req.postDataJSON();
        let value = {}, status = 200;
        if (p === '/api/me/articles' && req.method() === 'GET') value = { items: [], next_offset: null };
        else if (p === '/api/me/articles' && req.method() === 'POST') value = a;
        else if (p === '/api/me/groups') value = { groups: [{ id: 'g1', name: '测试群' }] };
        else if (p.endsWith('/draft') && req.method() === 'PUT') {
          if (failSave) { failSave = false; status = 409; value = { error: '草稿已在其他设备修改，请保留当前内容并重新读取' }; }
          else { assert.equal(data.version, a.revision); a = { ...a, ...data.document, document: data.document, media: { ...media }, cover_data_url: media[data.document.cover], revision: a.revision + 1 }; value = a; }
        } else if (p.endsWith('/media')) { assert.ok(Buffer.from(data.base64, 'base64').length <= 512 * 1024); media.article_media_test = 'data:image/jpeg;base64,' + data.base64; value = { id: 'article_media_test', data_url: media.article_media_test };
        } else if (p.endsWith('/publish')) { assert.deepEqual(data.group_ids, ['g1']); publications++; value = { messages: publications === 1 ? [{}] : [], already_shared_groups: publications > 1 ? ['g1'] : [] }; }
        else if (p.includes('/revisions/')) value = { ...a, title: '已发布文章', status: 'published', document: { ...a.document, title: '已发布文章', blocks: [{ type: 'paragraph', text: '群内成员可以阅读完整正文。' }] } };
        await route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(value) });
      });
      if (mobile) {
        await page.route('**/article-mobile-fixture', route => route.fulfill({ contentType: 'text/html', body: '<html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body style="margin:0;background:#10151d;color:#eee"><button id="strip">群聊</button><div id="bubble"></div></body></html>' }));
        await page.goto(base + '/article-mobile-fixture');
        await page.addStyleTag({ path: path.join(root, 'server/src/assets/articles.css') });
        await page.addScriptTag({ path: path.join(root, 'server/src/assets/articles.js') });
        await page.evaluate(() => { window.fixtureApi = (p, o) => fetch(p, o); window.ElonArticles.install(window.fixtureApi, () => ({ id: 'g1' }), document.getElementById('strip')); });
      } else await page.goto(base + '/pc/.ai-tmp/article-preview.html');
      try { await page.getByRole('button', { name: '文章', exact: true }).click({ timeout: 15000 }); }
      catch (error) { console.error('ARTICLE_PAGE=' + page.url() + '\n' + (await page.locator('body').innerText()).slice(0, 1200)); throw error; }
      await page.getByRole('button', { name: '写文章', exact: true }).click();
      await page.getByLabel('标题', { exact: true }).fill('给群友的一篇文章');
      await page.getByRole('button', { name: '＋ 添加段落', exact: true }).click();
      await page.getByLabel(mobile ? '内容' : '第1段内容', { exact: true }).fill('这是完整正文。\n第二行保持换行。');
      const picture = await page.evaluate(() => { const c = document.createElement('canvas'); c.width = 960; c.height = 540; const ctx = c.getContext('2d'); const gradient = ctx.createLinearGradient(0, 0, 960, 540); gradient.addColorStop(0, '#284864'); gradient.addColorStop(1, '#b87d57'); ctx.fillStyle = gradient; ctx.fillRect(0, 0, 960, 540); ctx.fillStyle = '#ffffffcc'; ctx.font = '42px sans-serif'; ctx.fillText('Article test fixture', 70, 280); return c.toDataURL('image/png').split(',')[1]; });
      await page.locator('input[type=file]').first().setInputFiles({ name: 'test-cover.png', mimeType: 'image/png', buffer: Buffer.from(picture, 'base64') });
      await page.getByRole('button', { name: '移除封面', exact: true }).waitFor();
      failSave = true;
      await page.getByRole('button', { name: '保存草稿', exact: true }).click();
      await page.getByText(/草稿已在其他设备修改/).waitFor();
      assert.equal(await page.getByLabel('标题', { exact: true }).inputValue(), '给群友的一篇文章');
      await page.getByRole('button', { name: '保存草稿', exact: true }).click();
      await page.getByText('草稿已保存', { exact: true }).waitFor();
      await page.getByRole('button', { name: '预览', exact: true }).click();
      await page.getByRole('heading', { name: '给群友的一篇文章', exact: true }).waitFor();
      await page.getByRole('button', { name: '发布到群', exact: true }).click();
      await page.getByRole('button', { name: '确认发布', exact: true }).click();
      await page.getByText(mobile ? '文章已发布到群聊' : '文章已发布到所选群聊', { exact: true }).waitFor();
      assert.equal(publications, 1);
      await page.getByRole('button', { name: '发布到群', exact: true }).click();
      await page.getByRole('button', { name: '确认发布', exact: true }).click();
      await page.getByText('这些群已收到此版本，没有重复发送', { exact: true }).waitFor();
      await page.screenshot({ path: path.join(root, '.ai-tmp', `article-${mobile ? 'mobile' : 'pc'}-preview.png`), fullPage: true });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      if (mobile) {
        await page.keyboard.press('Escape');
        await page.evaluate(() => window.ElonArticles.mount(document.getElementById('bubble'), '【一龙文章】\n' + JSON.stringify({schema:1,article_id:'article_fixture',revision:1,title:'已发布文章',summary:'卡片摘要'}), window.fixtureApi, 'test_author'));
        await page.getByRole('button', { name: /已发布文章/ }).click();
      } else {
        await page.getByRole('button', { name: '返回文章列表', exact: true }).click();
        await page.getByRole('button', { name: '返回群聊', exact: true }).click();
        await page.getByRole('button', { name: '阅读文章：已发布文章', exact: true }).click();
      }
      await page.getByText('群内成员可以阅读完整正文。', { exact: true }).waitFor();
      assert.deepEqual(errors, []);
      console.log(`ARTICLE_UI_${mobile ? 'PWA' : 'PC'}=passed (draft conflict preserves text, save, preview, publish, retry, card, reader, viewport)`);
      await context.close();
    }
  } finally { if (browser) await browser.close(); await server.close(); await fs.unlink(path.join(tmp, 'article-preview.html')); await fs.unlink(path.join(tmp, 'article-preview.tsx')); await fs.rmdir(tmp).catch(() => {}); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
