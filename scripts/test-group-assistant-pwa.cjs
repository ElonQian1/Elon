// Production reader, synthetic first-party responses, no provider credentials or external requests.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const binding = { id: 'binding_test', title: '行业关注与更新', owner_name: '分享者', owned: true, state: 'ready', synced_at: '2026-09-23T00:00:00Z', update_count: 21 };
const update = { id: 'result_test', sequence: 21, created_at: '2026-09-23T00:00:00Z', content: '# 重点更新\n\n**结论**\n\n| 项目 | 状态 |\n|---|---|\n|服务|正常|\n\n```js\nconst result = true;\n```\n\n[来源](https://example.com/)\n\n<img src=x onerror="window.xss=true">' };
async function main() {
  const html = '<!doctype html><html><head><meta name="viewport" content="width=device-width, initial-scale=1"><link rel="stylesheet" href="/assets/group_assistant.css">' +
    ['ai_share_marked.js', 'ai_conversation_rich.js', 'group_assistant.js'].map(name => `<script src="/assets/${name}"></script>`).join('') +
    '</head><body style="background:#101113;color:#eee"><button id="entry">群 AI 助手</button><div id="summary"></div><textarea aria-label="群草稿">保留的草稿</textarea><script>' +
    'window.group={id:"group_test"};window.owner="fixture";ElonGroupAssistant.install((p,o)=>fetch(p,o),()=>group,()=>owner,document.getElementById("entry"),document.getElementById("summary"));</script></body></html>';
  let revoked = false, slow = false;
  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, 'http://localhost');
    if (url.pathname === '/') { res.setHeader('content-type', 'text/html; charset=utf-8'); return res.end(html); }
    if (/^\/assets\/[\w.-]+$/.test(url.pathname)) {
      res.setHeader('content-type', url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript');
      return res.end(await fs.readFile(path.join(assets, path.basename(url.pathname))));
    }
    if (slow) await new Promise(resolve => setTimeout(resolve, 300));
    res.setHeader('content-type', 'application/json');
    if (req.method === 'DELETE') { revoked = true; return res.end('{}'); }
    if (url.pathname.endsWith('/ai-assistant')) return res.end(JSON.stringify({ items: revoked ? [] : [binding] }));
    const before = url.searchParams.get('before');
    return res.end(JSON.stringify({ items: [before === '21' ? { ...update, id: 'older', sequence: 20 } : update], next_cursor: before === '21' ? null : 21 }));
  });
  let browser;
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' });
    const page = await browser.newPage(), errors = [], external = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.route('**/*', route => { if (!route.request().url().startsWith(origin)) { external.push(route.request().url()); return route.abort(); } return route.continue(); });
    const output = path.join(root, '.ai-tmp/group-assistant-pwa'); await fs.mkdir(output, { recursive: true });
    for (const viewport of [{ width: 390, height: 844 }, { width: 1280, height: 800 }, { width: 320, height: 640 }]) {
      revoked = false; await page.setViewportSize(viewport); await page.goto(origin);
      await page.getByRole('button', { name: '群 AI 助手', exact: true }).click();
      await page.getByRole('button', { name: binding.title, exact: true }).click();
      await page.getByRole('button', { name: '较早更新', exact: true }).click();
      await page.waitForFunction(() => document.querySelectorAll('.group-assistant-row').length === 2);
      await page.locator('.group-assistant-row button').first().click();
      await page.locator('dialog table').waitFor();
      assert.equal(await page.locator('dialog strong').innerText(), '结论');
      assert.equal(await page.locator('dialog code').innerText(), 'const result = true;');
      assert.equal(await page.locator('dialog img').count(), 0);
      assert.equal(await page.evaluate(() => Boolean(window.xss)), false);
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
      await page.screenshot({ path: path.join(output, `reader-${viewport.width}.png`), fullPage: true });
      await page.getByRole('button', { name: '关闭', exact: true }).click();
      assert.equal(await page.getByRole('textbox', { name: '群草稿' }).inputValue(), '保留的草稿');
    }
    await page.getByRole('button', { name: '群 AI 助手', exact: true }).click();
    page.once('dialog', d => d.accept());
    await page.getByRole('button', { name: '取消分享', exact: true }).click();
    await page.getByText('暂无关注事项', { exact: true }).waitFor();
    await page.getByRole('button', { name: '关闭', exact: true }).click();
    slow = true; await page.getByRole('button', { name: '群 AI 助手', exact: true }).click();
    await page.evaluate(() => { window.owner = 'other'; });
    await page.locator('dialog').waitFor({ state: 'detached' });
    assert.deepEqual(errors, []); assert.deepEqual(external, []);
    console.log('GROUP_ASSISTANT_PWA=passed viewports=3 pagination=passed revoke=passed account_switch=passed rich_text=passed');
  } finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
