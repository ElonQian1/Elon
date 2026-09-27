const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const output = path.join(root, '.ai-tmp', 'chat-record-ui');
const doc = {
  card: { schema: 'chat_record_bundle_v1', record_id: 'record_test', group_id: 'group_test', title: '微信聊天记录', summary: '甲：项目讨论\n乙：两条转发记录', message_count: 3, total_count: 5 },
  owner_id: 'someone_else',
  document: { title: '微信聊天记录', raw_text: 'Original fixture text', warnings: [], messages: [
    { id:'one', parent_id:null, sender:'甲', time:'2026年09月27日 12:00', kind:'text', text:'项目讨论。\n保留换行和链接 https://example.com/report', filename:'', asset_id:null },
    { id:'two', parent_id:null, sender:'乙', time:'2026年09月27日 12:01', kind:'forward', text:'[聊天记录]', filename:'', asset_id:null },
    { id:'nested1', parent_id:'two', sender:'丙', time:'2026年09月26日 10:00', kind:'text', text:'第一条内部消息', filename:'', asset_id:null },
    { id:'nested2', parent_id:'two', sender:'丁', time:'2026年09月26日 10:01', kind:'text', text:'第二条内部消息 <script>window.compromised=true</script>', filename:'', asset_id:null },
    { id:'three', parent_id:null, sender:'甲', time:'2026年09月27日 12:02', kind:'image', text:'[图片] fixture.png', filename:'fixture.png', asset_id:'asset_test' }
  ] }
};
(async () => {
  fs.mkdirSync(output, { recursive: true });
  const browser = await chromium.launch({ headless: true, ...(process.env.PLAYWRIGHT_CHANNEL ? { channel: process.env.PLAYWRIGHT_CHANNEL } : {}) });
  try {
    for (const width of [1280, 390]) {
      const page = await browser.newPage({ viewport: { width, height: width === 390 ? 844 : 900 } });
      const failures = []; page.on('pageerror', e => failures.push(e.message));
      await page.setContent('<!doctype html><html lang="zh"><meta charset="utf-8"><body style="margin:0;background:#171717;color:#eee;font-family:Arial,sans-serif"><div id="bubble"></div></body></html>');
      await page.addStyleTag({ content: fs.readFileSync(path.join(root, 'server/src/assets/chat_records.css'), 'utf8') });
      await page.addScriptTag({ content: fs.readFileSync(path.join(root, 'server/src/assets/chat_records.js'), 'utf8') });
      await page.evaluate(value => {
        window.recordCurrent = true;
        const api = async route => {
          if (route.includes('/assets/')) return new Response(Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+nmZkAAAAASUVORK5CYII='), c => c.charCodeAt(0)), { headers: { 'content-type':'image/png' } });
          return new Response(JSON.stringify(value), { headers: { 'content-type':'application/json' } });
        };
        window.ElonChatRecords.mount(document.getElementById('bubble'), '【一龙聊天记录】\n' + JSON.stringify(value.card), { api, group:'group_test', owner:'reader', current: () => window.recordCurrent });
      }, doc);
      await page.locator('.chat-record-card').click();
      await page.getByRole('button', { name:'聊天记录 · 2 条', exact:true }).waitFor();
      assert.equal(await page.locator('.chat-record-feed article').count(), 3);
      await page.locator('.chat-record-asset').scrollIntoViewIfNeeded();
      await page.waitForFunction(() => [...document.querySelectorAll('.chat-record-asset img')].some(i => i.naturalWidth > 0));
      assert.equal(await page.locator('.chat-record-reader').evaluate(n => n.scrollWidth <= n.clientWidth + 1), true);
      await page.screenshot({ path:path.join(output, `records-${width}.png`) });
      await page.getByRole('button', { name:'聊天记录 · 2 条', exact:true }).click();
      assert.equal(await page.locator('.chat-record-feed article').count(), 2);
      assert.equal(await page.evaluate(() => window.compromised), undefined);
      await page.getByRole('button', { name:'返回', exact:true }).click();
      assert.equal(await page.locator('.chat-record-feed article').count(), 3);
      await page.getByRole('button', { name:'原始文本', exact:true }).click();
      assert.equal(await page.locator('.chat-record-feed pre').textContent(), 'Original fixture text');
      await page.getByRole('button', { name:'关闭', exact:true }).click();
      assert.equal(await page.locator('dialog').count(), 0);
      await page.locator('.chat-record-card').click();
      await page.evaluate(() => { window.recordCurrent = false; });
      await page.waitForFunction(() => !document.querySelector('dialog'));
      assert.deepEqual(failures, []); await page.close();
    }
    console.log('CHAT_RECORD_READER_UI=passed desktop/mobile nested/back/media/raw/XSS/account-change');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
