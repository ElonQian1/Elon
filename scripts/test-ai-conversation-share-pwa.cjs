// Isolated API fixtures exercise the production PWA group renderer; no account or vendor requests.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const runtime = ['ai_share_marked.js', 'ai_conversation_rich.js', 'ai_conversation_share.js', 'ai_conversation_reader.js', 'social_chat_view.js'];
const output = path.join(root, '.ai-tmp/ai-conversation-share-pwa');
const schema = 'elon.ai_conversation_share.v1', prefix = '【一龙AI对话】\n';
const imageId = 'article_media_fixture';
const ref = { schema, snapshot_id: 'snapshot_fixture', group_id: 'group_fixture', title: '分享内容：代码与图表', summary: '重点结论与参考内容', provider: 'chatgpt', message_count: 4, sender_name: '分享者甲', cover_asset_id: imageId };
const message = { id: 'group_message_fixture', content: prefix + JSON.stringify(ref), sender_user_id: 'author', sender_name: '分享者甲', outgoing: false, created_at: '2026-09-15T10:00:00Z', attachments: [{ kind: 'image', url: 'https://remote.invalid/not-a-share-asset.png' }] };
const imagePart = { type: 'image', label: '分享图片', asset_id: imageId, media_type: 'image/png', caption: '保留的图片' };
const document = { schema, provider: 'chatgpt', title: ref.title, summary: ref.summary, cover_asset_id: imageId, messages: [
  { id: 'shared_1', role: 'user', content: '请解释这段代码。', created_at_ms: 0, gap_before: false, parts: [] },
  { id: 'shared_2', role: 'assistant', content: '# 重点结论\n\n**安全文本**与普通 api_key 示例。\n\n- 第一项\n- 第二项\n\n> 引用内容\n\n| 项目 | 数值 |\n| --- | --- |\n| A | 12 |\n\n```js\nconst api_key = "example";\n```\n\n[文档](https://example.org/docs?q=search#section)\n\n[危险](javascript:alert(1))\n\n[凭证](https://example.org/?token=secret)\n\n[用户信息](https://name:password@example.org/)\n\n![禁止远程图](https://remote.invalid/secret.png)\n\n<img src="https://remote.invalid/html.png" onerror="window.xss=true">', created_at_ms: 0, gap_before: false, parts: [
    imagePart,
    { type: 'code', label: '完整代码', text_block: { version: 1, id: 'block_code', kind: 'code', title: '代码片段', language: 'js', content: 'console.log("完整代码");', complete: true } },
    { type: 'writing_block', label: '正文', text_block: { version: 1, id: 'block_writing', kind: 'writing', title: '完整正文', language: '', content: '这是**完整正文**。', complete: true } },
    { type: 'math', label: '公式', caption: 'E = mc^2' },
    { type: 'citation', label: '引用来源', caption: '公开文档' },
    { type: 'unavailable', label: '此内容未包含在分享中' },
    { type: 'rich_card', label: '行情', card: { kind: 'chart', title: '测试曲线', symbol: 'FIXTURE', primary_value: '12.00', secondary_value: '+2.00', trend: 'positive', periods: [{ id: 'day', label: '一天', selected: true }], metrics: [{ label: '开盘', value: '10.00' }], series: [{ key: 'price', label: '数值' }], points: [{ label: '09:00', values: [10] }, { label: '10:00', values: [12] }, { label: '11:00', values: [11] }] } },
  ] },
  { id: 'shared_3', role: 'user', content: '再说明边界。', created_at_ms: 0, gap_before: true, parts: [] },
  { id: 'shared_4', role: 'assistant', content: Array.from({ length: 35 }, (_, i) => `第 ${i + 1} 段说明，仍然是选中的消息内容。`).join('\n\n'), created_at_ms: 0, gap_before: false, parts: [] },
] };
const inertSchemes = ['file:///tmp/example', 'sandbox:/mnt/data/example', 'javascript:alert(1)', 'vbscript:msgbox(1)', 'data:text/plain,example', 'blob:https://example.org/id'];
document.messages[1].content += '\n\n```js\n' + inertSchemes.map(value => `const example = ${JSON.stringify(value)};`).join('\n') + '\n```\n\n'
  + inertSchemes.map((value, index) => `Prose example: \`${value}\`\n\n[scheme-${index}](<${value}>)`).join('\n\n');
const view = { snapshot_id: ref.snapshot_id, group_id: ref.group_id, owner_id: 'author', owner_name: '分享者甲', created_at: '2026-09-15T10:00:00Z', document };
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=', 'base64');

async function main() {
  const html = await fs.readFile(path.join(assets, 'web_page.html'), 'utf8');
  const mainStyle = html.match(/<style>([\s\S]*?)<\/style>/)[1];
  const appendBubble = html.slice(html.indexOf('  function appendBubble('), html.indexOf('  // ====== WebSocket'));
  for (const name of [...runtime.slice(0, 4), 'ai_conversation_share.css']) assert.ok(html.includes(`/assets/${name}`), `missing page include: ${name}`);
  for (const name of runtime.slice(1)) {
    const code = await fs.readFile(path.join(assets, name), 'utf8');
    assert.equal(/innerHTML\s*=|insertAdjacentHTML|\beval\(/.test(code), false, name + ' must not interpret untrusted HTML');
    assert.ok(code.split('\n').length < 500, name + ' stays modular');
  }
  const fixture = `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>${mainStyle}</style><link rel="stylesheet" href="/assets/orbital_mobile_theme.css"><link rel="stylesheet" href="/assets/ai_conversation_share.css"><style>body{display:block;margin:0}#chatList{height:calc(100dvh - 88px);overflow:auto;padding:16px}.chat-message-block{margin-bottom:16px}#draft{height:64px;width:100%;display:block}</style>${runtime.map(name => `<script src="/assets/${name}"></script>`).join('')}</head><body><div id="chatList"></div><textarea id="draft" aria-label="群聊草稿">尚未发送的群聊草稿</textarea><script>
  const chatList=document.getElementById('chatList'); let lastChatTimelineMs=0;
  let currentUser={id:'viewer',nickname:'查看者乙'}, currentGroup={id:'group_fixture',members:[]};
  const formatChatTimelineLabel=()=>'',formatChatExactTime=()=>'',withToken=v=>v,avatarInitial=()=>'';
  const groupMentions={bindAvatar(){}};
  function createFriendAvatarElement(){const avatar=document.createElement('span');avatar.className='chat-avatar';return avatar;}
  window.ElonArticles={mount(){}};window.ElonGroupMessageRevisions={mount(){window.revisionMounts++;}};
  window.revisionMounts=0;window.apiCalls=[];window.createdUrls=[];window.revokedUrls=[];
  const createObjectURL=URL.createObjectURL.bind(URL),revokeObjectURL=URL.revokeObjectURL.bind(URL);
  URL.createObjectURL=b=>{const u=createObjectURL(b);window.createdUrls.push(u);return u;};URL.revokeObjectURL=u=>{window.revokedUrls.push(u);revokeObjectURL(u);};
  ${appendBubble}
  const api=(url,options={})=>{window.apiCalls.push({url,...options});return fetch(url,{...options,headers:{Authorization:'Bearer fixture-viewer'}});};
  const renderer=ElonSocialChatView.create({list:chatList,append:appendBubble,api,user:()=>currentUser,time:()=>0,friendName:()=>'',resetTimeline(){lastChatTimelineMs=0;},changed(){}});
  window.draw=message=>{window.currentMessage=message;const filler=Array.from({length:20},(_,i)=>({id:'filler-'+i,content:'已有群消息 '+i+'\\n保留群聊滚动位置',sender_user_id:'author',sender_name:'分享者甲'}));renderer.render([...filler,message],'group',currentGroup,false);};
  window.resetAccount=()=>{renderer.reset();chatList.replaceChildren();currentUser={id:'other',nickname:'另一账号'};};
  window.switchGroup=()=>{currentGroup={id:'new_group',members:[]};renderer.render([{id:'new',content:'新群消息',sender_user_id:'author'}],'group',currentGroup,false);};
  window.readerOpen=()=>ElonAiConversationReader.open({api,ref:${JSON.stringify(ref)},list:chatList,senderName:'分享者甲',messageId:'group_message_fixture'});
  </script></body></html>`;
  const state = { code: 200, media: 200, reads: 0, mediaReads: 0, requests: [], slow: 0 };
  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://127.0.0.1');
      if (url.pathname === '/') { res.setHeader('Content-Type', 'text/html; charset=utf-8'); return res.end(fixture); }
      if (/^\/assets\/[\w.-]+$/.test(url.pathname)) {
        const name = path.basename(url.pathname); res.setHeader('Content-Type', name.endsWith('.css') ? 'text/css' : 'text/javascript'); return res.end(await fs.readFile(path.join(assets, name)));
      }
      if (url.pathname.startsWith('/api/me/groups/')) {
        state.requests.push({ url: req.url, authorization: req.headers.authorization, method: req.method });
        assert.equal(req.headers.authorization, 'Bearer fixture-viewer'); assert.equal(url.search, '');
        res.setHeader('Cache-Control', 'private, no-store');
        if (url.pathname.endsWith('/assets/' + imageId)) { state.mediaReads++; res.statusCode = state.media; res.setHeader('Content-Type', state.media === 200 ? 'image/png' : 'application/json'); return res.end(state.media === 200 ? png : '{}'); }
        state.reads++; const code = state.code;
        if (state.slow) await new Promise(resolve => setTimeout(resolve, state.slow));
        res.statusCode = code; res.setHeader('Content-Type', 'application/json'); return res.end(JSON.stringify(code === 200 ? view : { error: 'fixture error' }));
      }
      res.statusCode = 404; res.end();
    } catch (error) { res.statusCode = 500; res.end(String(error)); }
  });
  let browser;
  try {
    await fs.mkdir(output, { recursive: true });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' });
    const page = await browser.newPage(), errors = [], external = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/*', route => {
      if (!route.request().url().startsWith(origin) && !route.request().url().startsWith('blob:')) { external.push(route.request().url()); return route.abort(); }
      return route.continue();
    });
    const wait = predicate => page.waitForFunction(predicate);
    const open = async () => { await page.locator('.ai-share-card').click(); await page.locator('.ai-share-message').first().waitFor(); };
    const close = async () => { await page.getByRole('button', { name: '返回群聊', exact: true }).click(); await page.locator('.ai-share-reader').waitFor({ state: 'detached' }); await wait(() => !history.state?.elonAiSnapshot); };
    for (const viewport of [{ width: 390, height: 844 }, { width: 1280, height: 800 }, { width: 320, height: 640 }]) {
      state.code = 200; state.media = 200;
      await page.setViewportSize(viewport); await page.goto(origin); await page.evaluate(seed => draw(seed), message);
      await page.locator('.ai-share-card').scrollIntoViewIfNeeded();
      await wait(() => !!document.querySelector('.ai-share-cover img:not([hidden])'));
      const scroll = await page.locator('#chatList').evaluate(node => node.scrollTop);
      await page.screenshot({ path: path.join(output, `card-${viewport.width}.png`) });
      await open();
      await wait(() => !!document.querySelector('.ai-share-image img:not([hidden])'));
      assert.equal(await page.locator('.ai-share-message').count(), 4);
      assert.equal(await page.locator('.ai-share-gap').textContent(), '中间内容未分享');
      assert.match(await page.locator('.ai-share-reader .bubble.user .ai-share-speaker').first().textContent(), /分享者甲/);
      assert.equal(await page.locator('.ai-share-reader').textContent().then(t => t.includes('查看者乙')), false);
      assert.equal(await page.locator('.ai-share-reader .bubble.ai strong').first().textContent(), '安全文本');
      assert.ok(await page.locator('.ai-share-reader pre code').count() >= 2);
      assert.ok(await page.locator('.ai-share-reader table').count() >= 2);
      assert.equal(await page.locator('.ai-share-reader a').count(), 1);
      assert.match(await page.locator('.ai-share-reader a').getAttribute('href'), /q=search#section/);
      const preservedCode = await page.locator('.ai-share-reader pre code').allTextContents();
      for (const [index, value] of inertSchemes.entries()) {
        assert.ok(preservedCode.some(text => text.includes(value)), `code must preserve ${value}`);
        const inertLabel = page.getByText(`scheme-${index}`, { exact: true });
        assert.equal(await inertLabel.evaluate(node => node.tagName), 'SPAN', `${value} must not become an actionable link`);
      }
      assert.equal(await page.locator('.ai-share-reader img').count(), 1);
      assert.equal(await page.evaluate(() => !!window.xss), false);
      const geometry = await page.locator('.ai-share-reader').evaluate(node => ({ width: node.getBoundingClientRect().width, height: node.getBoundingClientRect().height, overflow: node.scrollWidth > node.clientWidth, buttons: [...node.querySelectorAll('header button')].map(n => n.getBoundingClientRect().width) }));
      assert.equal(geometry.width, viewport.width); assert.equal(geometry.height, viewport.height); assert.equal(geometry.overflow, false); assert.ok(geometry.buttons.every(w => w >= 48));
      const pixels = await page.locator('canvas').evaluate(canvas => Array.from(canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height).data).filter((_, i) => i % 4 === 3).some(v => v > 0));
      assert.ok(pixels, 'rich chart must contain rendered pixels');
      await page.screenshot({ path: path.join(output, `reader-${viewport.width}.png`) });
      await page.getByRole('button', { name: '搜索分享内容', exact: true }).click();
      await page.getByRole('searchbox', { name: '搜索分享内容' }).fill('完整正文');
      await wait(() => document.querySelector('.ai-share-search output').textContent === '1 条匹配');
      assert.equal(await page.locator('.ai-share-message:visible').count(), 1); assert.ok(await page.locator('mark').count());
      await page.getByRole('searchbox').fill('不存在的匹配文本'); await page.getByText('没有匹配内容').waitFor();
      await page.getByRole('button', { name: '清除搜索' }).click(); await wait(() => [...document.querySelectorAll('.ai-share-message')].every(n => !n.hidden));
      await close();
      assert.equal(await page.getByRole('textbox', { name: '群聊草稿' }).inputValue(), '尚未发送的群聊草稿');
      assert.equal(await page.locator('#chatList').evaluate(node => node.scrollTop), scroll);
      assert.equal(await page.evaluate(() => revisionMounts), 20, 'share cards must not get a generic edit action');
      await open(); await page.goBack(); await page.locator('dialog').waitFor({ state: 'detached' });
      assert.equal(await page.locator('#chatList').evaluate(node => node.scrollTop), scroll);
      console.log(`PASS ${viewport.width}px: production group renderer, protected image, sender identity, rich content, search, layout, browser back and draft/scroll`);
    }
    state.code = 503; await page.locator('.ai-share-card').click(); await page.getByText('读取失败，请检查网络后重试').waitFor();
    state.code = 200; await page.getByRole('button', { name: '重新读取' }).click(); await page.locator('.ai-share-message').first().waitFor();
    state.code = 403; await page.evaluate(() => window.dispatchEvent(new Event('focus'))); await page.getByText('无权查看，仅当前群成员可读').first().waitFor();
    assert.equal(await page.locator('.ai-share-message').count(), 0); assert.equal(await page.locator('.ai-share-reader img').count(), 0);
    await wait(() => window.createdUrls.every(u => window.revokedUrls.includes(u)));
    state.code = 200; await page.getByRole('button', { name: '重新读取' }).click(); await page.locator('.ai-share-message').first().waitFor(); await close();
    for (const code of [401, 403, 404, 410]) {
      state.code = code; const before = state.reads; await page.locator('.ai-share-card').click();
      await wait(() => !document.querySelector('.ai-share-reader-content').hasAttribute('aria-busy'));
      assert.ok(state.reads > before, 'every open must authorize again'); assert.equal(await page.locator('.ai-share-message').count(), 0);
      assert.equal(await page.getByRole('button', { name: '重新读取' }).isVisible(), [401, 403].includes(code));
      await close();
    }
    state.code = 200; state.media = 503; await open(); await page.getByRole('button', { name: '重试图片' }).last().waitFor();
    state.media = 200; await page.getByRole('button', { name: '重试图片' }).last().click(); await wait(() => !!document.querySelector('.ai-share-image img:not([hidden])'));
    await page.evaluate(seed => draw({ ...seed, recalled_at: '2026-09-15T11:00:00Z' }), message);
    await page.getByText('分享已撤销、撤回或不存在').first().waitFor(); assert.equal(await page.locator('.ai-share-message').count(), 0); await close();
    await page.evaluate(seed => draw(seed), message); await open(); await page.evaluate(() => switchGroup());
    await page.locator('dialog').waitFor({ state: 'detached' }); await wait(() => !history.state?.elonAiSnapshot);
    assert.equal(await page.locator('#chatList').evaluate(node => node.scrollTop), 0, 'old reader must not restore scroll into a new group');
    await page.goto(origin);
    await page.evaluate(seed => draw(seed), message); await open();
    await page.evaluate(() => resetAccount()); await page.locator('dialog').waitFor({ state: 'detached' });
    await wait(() => window.createdUrls.every(u => window.revokedUrls.includes(u)));
    assert.equal(await page.locator('.ai-share-card').count(), 0);
    state.slow = 300; await page.evaluate(() => readerOpen()); await page.keyboard.press('Escape');
    await page.locator('dialog').waitFor({ state: 'detached' }); await new Promise(resolve => setTimeout(resolve, 350));
    assert.equal(await page.locator('.ai-share-message').count(), 0); state.slow = 0;
    assert.equal(await page.evaluate(({ prefix, ref }) => ElonAiConversationShare.reference(prefix + JSON.stringify({ ...ref, group_id: 'other_group' }), ref.group_id), { prefix, ref }), null, 'cross-group markers rejected');
    assert.ok(state.requests.every(r => r.method === 'GET'), 'reader must never write');
    assert.deepEqual(external, [], 'no external images or vendor authentication requests'); assert.deepEqual(errors, []);
    console.log('PASS retries, 401/403/404/410, revocation purge, image retry, recalled message, account reset, cancellation, read-only and XSS/URL boundaries');
  } finally { if (browser) await browser.close(); await new Promise(resolve => server.close(resolve)); }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
