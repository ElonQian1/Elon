// Production components and PWA actions; all traffic and messages are synthetic.
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const out = path.join(root, '.ai-tmp/record-menu');
fs.mkdirSync(out, { recursive: true });
const card = { schema: 'chat_record_bundle_v1', record_id: 'fixture-record', group_id: 'fixture-group',
  title: '测试聊天记录', summary: '甲：会议资料\n乙：收到', message_count: 2, total_count: 2 };
const source = { id: 'source', sender_name: '示例群友', revision: 1, content: '【一龙聊天记录】\n' + JSON.stringify(card) };
async function pointer(page, locator, type, extra = {}) {
  const box = await locator.boundingBox();
  await locator.dispatchEvent(type, { pointerId: 1, pointerType: 'touch', isPrimary: true, button: 0,
    clientX: box.x + 25, clientY: box.y + 20, bubbles: true, ...extra });
}
async function pc(browser) {
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } });
    page.setDefaultTimeout(12000);
    console.log(`PC record menu: ${width}px`);
    const errors = [], sends = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript({ path: path.join(root, 'server/src/assets/social_links.js') });
    await page.route(/^https?:\/\/[^/]+\/api\//, route => {
      const headers = { 'access-control-allow-origin': '*', 'access-control-allow-headers': '*' };
      if (route.request().method() === 'OPTIONS') return route.fulfill({ status: 204, headers });
      if (route.request().url().endsWith('/chat-records/fixture-record')) return route.fulfill({ headers,
        json: { card, owner_id: 'me', document: { title: card.title, messages: [], warnings: [], raw_text: '' } } });
      if (route.request().method() === 'POST' && route.request().url().endsWith('/messages')) {
        const body = route.request().postDataJSON(); sends.push(body);
        return route.fulfill({ headers, json: { message: { id: 'sent', content: body.content, outgoing: true,
          created_at: '2026-10-03T00:00:00Z', sender_user_id: 'me', quote: { ...source, attachments: [], message_id: source.id, unavailable: false } } } });
      }
      return route.fulfill({ headers, json: {} });
    });
    await page.goto(`http://127.0.0.1:${process.env.QUOTE_VITE_PORT || 5198}/pc/tests/fixtures/social-quotes.html?record=1`);
    const record = page.getByRole('button', { name: '查看聊天记录：测试聊天记录', exact: true });
    await record.waitFor();
    await record.click({ button: 'right' });
    for (const name of ['引用', '打开聊天记录', '复制标题', '多选', '添加阅读书签', '详细信息', '隐藏消息']) {
      assert.equal(await page.getByRole('menuitem', { name, exact: true }).count(), 1, `PC missing ${name}`);
    }
    assert.equal(await page.getByRole('menuitem', { name: '复制', exact: true }).count(), 0);
    await page.getByRole('menuitem', { name: '引用', exact: true }).click();
    const draft = page.getByLabel('待发送引用', { exact: true });
    assert.match(await draft.innerText(), /聊天记录.*测试聊天记录/);
    assert.doesNotMatch(await draft.innerText(), /record_id|schema/);
    await page.getByRole('button', { name: '取消引用', exact: true }).click();
    await pointer(page, record, 'pointerdown');
    await page.waitForTimeout(650);
    await page.getByRole('menu', { name: '消息操作' }).waitFor();
    await pointer(page, record, 'pointerup');
    await record.dispatchEvent('click', { bubbles: true });
    assert.equal(await page.getByRole('dialog').count(), 0, 'hold must not open reader');
    await page.screenshot({ path: path.join(out, `pc-menu-${width}.png`) });
    await page.getByRole('menuitem', { name: '引用', exact: true }).click();
    await page.getByPlaceholder('发送消息到 示例群聊…').fill('讨论这份记录');
    await page.getByRole('button', { name: '发送', exact: true }).click();
    await page.locator('[data-message-id="sent"]').waitFor().catch(async error => {
      throw Error(JSON.stringify({ error: error.message, errors, sends: sends.length, alerts: await page.getByRole('alert').allTextContents() }));
    });
    assert.deepEqual(sends[0].quote_source, { message_id: 'source', revision: 1 });
    assert.equal(sends[0].content, '讨论这份记录');
    const sentQuote = page.locator('[data-message-id="sent"]').getByLabel('引用的消息', { exact: true });
    assert.match(await sentQuote.innerText(), /测试聊天记录/);
    await sentQuote.getByRole('button').click();
    await pointer(page, record, 'pointerdown');
    await pointer(page, record, 'pointermove', { clientY: 4000 });
    await page.waitForTimeout(650);
    assert.equal(await page.getByRole('menu').count(), 0, 'scroll must cancel hold');
    await pointer(page, record, 'pointercancel');
    await record.click();
    await page.getByRole('dialog').waitFor();
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log('PASS PC: additive record menu, touch hold, scroll cancellation, tap, structured quote');
}
async function pwa(browser) {
  const page = await browser.newPage({ viewport: { width: 390, height: 900 } });
  await page.route('https://menu.test/**', route => route.fulfill({ contentType: 'text/html', body: '<div id="list"></div><div class="input-panel"><textarea></textarea></div>' }));
  await page.goto('https://menu.test/');
  for (const file of ['chat_records.js', 'social_message_actions.js']) await page.addScriptTag({ path: path.join(root, 'server/src/assets', file) });
  await page.evaluate(source => {
    const list = document.querySelector('#list'), input = document.querySelector('textarea');
    const block = document.createElement('div'); block.className = 'chat-message-content';
    const bubble = document.createElement('div'); bubble.className = 'bubble'; block.append(bubble); list.append(block);
    window.fixture = { source, sends: [], owner: 'me' };
    const f = window.fixture;
    f.actions = ElonSocialMessageActions.create({ list, input, owner: () => f.owner, userId: () => 'me',
      send: async (...args) => { f.sends.push(args); }, changed: () => {} });
    f.actions.update([source], 'group', { id: 'fixture-group' });
    ElonChatRecords.mount(bubble, source.content, { group: 'fixture-group', current: () => true,
      api: async () => { throw Error('Synthetic fixture'); } });
    f.actions.bind(block, source);
  }, source);
  const record = page.locator('.chat-record-card');
  await pointer(page, record, 'pointerdown'); await page.waitForTimeout(650);
  await pointer(page, record, 'pointerup'); await record.dispatchEvent('click', { bubbles: true });
  const menu = page.getByRole('dialog', { name: '消息操作' });
  await menu.waitFor();
  await page.evaluate(() => fixture.actions.update([{ ...fixture.source }], 'group', { id: 'fixture-group' }));
  assert.equal(await menu.count(), 1, 'unchanged refresh must not close menu');
  assert.doesNotMatch(await menu.innerText(), /record_id|schema/);
  assert.equal(await menu.getByRole('button', { name: '打开聊天记录', exact: true }).count(), 1);
  assert.equal(await menu.getByRole('button', { name: '复制', exact: true }).count(), 0);
  await menu.getByRole('button', { name: '引用', exact: true }).click();
  assert.match(await page.locator('.social-quote-compose').innerText(), /测试聊天记录/);
  await page.evaluate(() => fixture.actions.send('group', { id: 'fixture-group' }, '讨论记录'));
  assert.deepEqual(await page.evaluate(() => fixture.sends[0][4]), { message_id: 'source', revision: 1 });
  await record.click({ button: 'right' });
  await menu.waitFor();
  await page.evaluate(() => fixture.actions.update([{ ...fixture.source, recalled_at: 'now' }], 'group', { id: 'fixture-group' }));
  assert.equal(await menu.count(), 0, 'recall must invalidate menu');
  await page.close();
  console.log('PASS PWA: hold, quote, concise summary, unchanged refresh, recall invalidation');
}
(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'chrome' });
  try { if (process.argv.includes('--pwa-only')) await pwa(browser); else { await pc(browser); await pwa(browser); } }
  finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
