// Functional browser fixture; this is not Android visual acceptance evidence.
const { chromium } = require('playwright');
const path = require('node:path');
const assert = require('node:assert/strict');

(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    await page.setContent('<style>:root{--bg:#101010;--panel:#242424;--ink:#ddd;--ink-soft:#aaa}</style><textarea id="draft"></textarea><span id="avatar">甲</span>');
    await page.addStyleTag({ path: path.join(__dirname, '../server/src/assets/group_mentions.css') });
    await page.addScriptTag({ path: path.join(__dirname, '../server/src/assets/group_mentions.js') });
    await page.evaluate(() => {
      window.group = { id: 'g1' }; window.failLoad = false; window.calls = 0;
      window.input = document.querySelector('#draft');
      window.controller = ElonGroupMentions.create({
        input, getGroup: () => window.group, getSelfId: () => 'self',
        api: async () => {
          window.calls++;
          if (window.failLoad) throw new Error('offline');
          return { ok: true, json: async () => ({
            members: Array.from({ length: 115 }, (_, index) => ({ id: 'u' + index, display_name: '群友' + index })),
            ai_members: [{ id: 'usr_elon_ai', display_name: 'EL' }],
          }) };
        },
      });
      controller.bindAvatar(document.querySelector('#avatar'), { id: 'u1', display_name: '群友甲' }, 'g1');
    });
    const input = page.locator('#draft');
    const closed = () => page.locator('dialog').waitFor({ state: 'detached' });
    async function open(prefix = '你好 ') { await input.fill(prefix); await input.press('End'); await input.pressSequentially('@'); await page.locator('dialog[open]').waitFor(); }
    await open();
    await page.getByPlaceholder('搜索群友或群 AI').fill('群友114');
    await page.getByRole('button', { name: '群友114 群友', exact: true }).click();
    await closed();
    assert.equal(await input.inputValue(), '你好 @群友114 ');
    assert.equal(await input.evaluate(el => el.selectionStart), 10);
    await open('');
    await page.getByRole('button', { name: '多选', exact: true }).click();
    await page.getByPlaceholder('搜索群友或群 AI').fill('群ai');
    await page.getByRole('button', { name: /EL 群 AI/ }).click();
    await page.getByPlaceholder('搜索群友或群 AI').fill('群友114');
    await page.getByRole('button', { name: /群友114 群友/ }).click();
    await page.getByRole('button', { name: '完成（2）', exact: true }).click();
    await closed();
    assert.equal(await input.inputValue(), '@EL @群友114 ');
    await open(); await page.getByRole('button', { name: '取消', exact: true }).click();
    await closed();
    assert.equal(await input.inputValue(), '你好 @');
    await page.evaluate(() => { window.failLoad = true; });
    await open('重试 ');
    await page.getByRole('button', { name: '重新加载', exact: true }).waitFor();
    await page.evaluate(() => { window.failLoad = false; });
    await page.getByRole('button', { name: '重新加载', exact: true }).click();
    await page.getByPlaceholder('搜索群友或群 AI').fill('不存在');
    await page.getByRole('status').filter({ hasText: '没有匹配' }).waitFor();
    await page.getByRole('button', { name: '取消', exact: true }).click();
    await closed();
    await input.fill('正文尾部'); await input.evaluate(el => el.setSelectionRange(2, 2));
    const box = await page.locator('#avatar').boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await page.mouse.down();
    await page.waitForTimeout(600); await page.mouse.up();
    assert.equal(await input.inputValue(), '正文 @群友甲 尾部');
    await open('旧草稿 '); await page.evaluate(() => { window.group = { id: 'g2' }; input.value = '新群草稿'; });
    await page.getByRole('button', { name: /EL 群 AI/ }).click();
    await closed();
    assert.equal(await input.inputValue(), '新群草稿');
    await input.fill('user'); await input.press('End'); await input.pressSequentially('@');
    assert.equal(await page.locator('dialog[open]').count(), 0);
    process.stdout.write('GROUP_MENTION_BROWSER=passed cases=8 viewport=390x844\n');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
