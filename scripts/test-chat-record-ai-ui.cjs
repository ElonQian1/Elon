// Exercises the production group feed and selection dialog with synthetic data only.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  const output = path.join(root, '.ai-tmp', 'record-ai-ui');
  fs.mkdirSync(output, { recursive: true });
  try {
    for (const width of [1280, 390]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      const errors = []; page.on('pageerror', e => errors.push(e.message));
      await page.addInitScript(() => {
        window.ElonSocialLinks = { compact: () => false, prepareBubble: () => {}, mount: () => () => {} };
      });
      await page.route(/^https?:\/\/[^/]+\/api\//, route => route.fulfill({ status: 200, contentType: 'application/json', body: '{}' }));
      await page.goto(`http://127.0.0.1:${process.env.RECORD_VITE_PORT || 5199}/pc/tests/fixtures/social-cards.html?records`);
      const entry = page.getByRole('button', { name: 'AI 分析记录', exact: true });
      await entry.click();
      const dialog = page.getByRole('dialog', { name: 'AI 回复到群聊' });
      await dialog.getByText('已选择 1 条消息').click();
      assert.match(await dialog.innerText(), /微信聊天记录 · 共 3 条/);
      assert.match(await dialog.innerText(), /将发送记录全文、嵌套记录/);
      assert.ok(!(await dialog.innerText()).includes('chat_record_bundle_v1'));
      await dialog.getByRole('button', { name: '取消', exact: true }).click();
      await entry.click({ button: 'right' });
      await page.getByRole('menuitem', { name: 'AI 分析记录…' }).click();
      await dialog.waitFor();
      await dialog.getByRole('button', { name: '取消', exact: true }).click();
      await entry.click({ button: 'right' });
      await page.getByRole('menuitem', { name: '多选', exact: true }).click();
      assert.equal(await page.getByRole('checkbox', { name: '选择消息 fixture-record' }).isChecked(), true);
      assert.equal(await page.getByRole('button', { name: '转发所选', exact: true }).count(), 0);
      await page.getByRole('button', { name: 'AI 分析', exact: true }).click();
      await dialog.waitFor();
      await page.screenshot({ path: path.join(output, `selection-${width}.png`), fullPage: true });
      assert.deepEqual(errors, []);
      await page.close();
    }
    console.log('PASS: record direct action, context menu, multi-select, preview and narrow layout');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
