// Actual PWA group renderer + message actions with synthetic, local-only snapshots.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const output = path.join(root, '.ai-tmp/grid-share-layout-reply');
const runtime = ['ai_conversation_rich.js', 'grid_share_view.js', 'grid_share.js', 'ai_conversation_share.js', 'social_message_actions.js', 'social_chat_view.js'];
const styles = ['orbital_mobile_theme.css', 'ai_conversation_share.css', 'social_message_actions.css', 'grid_share.css'];
const grid = { schema: 'yilong.grid_share.v1', observed_at_ms: 1790865693000, show_amounts: true, fields: {
  symbol: 'QNTUSDT', direction: 'LONG', leverage: '10', count: '30', spacing: 'ARITH', status: 'WORKING',
  profit: '479.92684725', unrealizedPnl: '-301.26979411', investment: '683.11265602', positionQty: '16.90000000',
  positionNotional: '4512.13100000', lower: '275.59000000', upper: '326.29000000', markPrice: '266.99',
} };
const ref = { schema: 'elon.ai_conversation_share.v1', provider: 'binance', group_id: 'grp_fixture', snapshot_id: 'ai_snapshot_fixture', title: 'QNTUSDT 网格快照', grid };
const message = { id: 'grid_message', revision: 2, content: '【一龙AI对话】\n' + JSON.stringify(ref), outgoing: true, sender_user_id: 'viewer', sender_name: '分享者', created_at: '2026-10-01T14:41:33Z' };
async function main() {
  const html = await fs.readFile(path.join(assets, 'web_page.html'), 'utf8');
  const mainStyle = html.match(/<style>([\s\S]*?)<\/style>/)[1];
  const appendBubble = html.slice(html.indexOf('  function appendBubble('), html.indexOf('  // ====== WebSocket'));
  const fixture = `<!doctype html><html lang="zh-CN" data-theme="dark"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>${mainStyle}</style>${styles.map(name => `<link rel="stylesheet" href="/assets/${name}">`).join('')}<style>body{display:block;margin:0;background:var(--mobile-surface)}#fixture-label{margin:12px;font-size:12px;color:var(--mobile-on-surface-variant)}#chatList{height:calc(100dvh - 170px);overflow:auto;padding:12px;box-sizing:border-box}.input-panel{position:relative}#draft{width:100%;min-height:48px;box-sizing:border-box}</style>${runtime.map(name => `<script src="/assets/${name}"></script>`).join('')}</head><body><p id="fixture-label">真实消息行 · 合成数据</p><div id="chatList"></div><div class="input-panel"><textarea id="draft" aria-label="群聊草稿"></textarea><button id="send">发送</button></div><script>
    const chatList=document.getElementById('chatList'),input=document.getElementById('draft');let lastChatTimelineMs=0;
    let currentUser={id:'viewer',nickname:'查看者'},currentGroup={id:'grp_fixture',members:[]},messages=[];
    const formatChatTimelineLabel=()=>'',formatChatExactTime=()=>'',withToken=v=>v,avatarInitial=()=>'';
    const groupMentions={bindAvatar(){}};
    function createFriendAvatarElement(){const avatar=document.createElement('span');avatar.className='chat-avatar';return avatar;}
    window.ElonArticles={mount(){}};window.ElonGroupMessageRevisions={mount(){}};
    window.ElonSocialMessageTransfer={copy:async()=>{},forward:()=>()=>{}};
    ${appendBubble}
    const api=(url,options={})=>fetch(url,{...options,headers:{Authorization:'Bearer synthetic'}});
    const actions=ElonSocialMessageActions.create({list:chatList,input,owner:()=>currentUser.id,userId:()=>currentUser.id,api,changed(){},
      send:async(kind,contact,content,attachments,quote_source)=>{const response=await api('/api/me/groups/'+contact.id+'/messages',{method:'POST',body:JSON.stringify({content,attachments,quote_source})});if(!response.ok)throw Error('发送失败');messages.push(await response.json());render();}});
    const renderer=ElonSocialChatView.create({list:chatList,append:appendBubble,api,actions,user:()=>currentUser,time:()=>0,friendName:()=>'',resetTimeline(){lastChatTimelineMs=0;},changed(){}});
    function render(){renderer.render(messages,'group',currentGroup,false);}
    window.draw=outgoing=>{renderer.reset();messages=[{...${JSON.stringify(message)},outgoing}];render();};
    window.recall=()=>{messages[0]={...messages[0],recalled_at:'now'};render();};
    window.switchGroup=()=>{currentGroup={id:'other',members:[]};renderer.render([],'group',currentGroup,false);};
    document.getElementById('send').onclick=()=>actions.send('group',currentGroup,input.value);
    window.quoteUnavailable=()=>{messages[1].quote={...messages[1].quote,unavailable:true};render();};
    window.draw(true);
  </script></body></html>`;
  const posts = [], requests = [];
  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://127.0.0.1');
      if (url.pathname === '/') { res.setHeader('Content-Type', 'text/html; charset=utf-8'); return res.end(fixture); }
      if (/^\/assets\/[\w.-]+$/.test(url.pathname)) return res.end(await fs.readFile(path.join(assets, path.basename(url.pathname))));
      if (url.pathname === '/assets/grid-token-icons/qnt.png') { res.setHeader('Content-Type', 'image/png'); return res.end(await fs.readFile(path.join(root, 'android/app/src/main/assets/grid-token-icons/qnt.png'))); }
      if (url.pathname.startsWith('/api/me/groups/')) {
        assert.equal(req.headers.authorization, 'Bearer synthetic'); requests.push(req.method + ' ' + url.pathname);
        res.setHeader('Content-Type', 'application/json');
        if (req.method === 'POST') {
          let body='';for await (const chunk of req) body+=chunk;const value=JSON.parse(body);posts.push(value);
          assert.deepEqual(value.quote_source,{message_id:message.id,revision:2});
          return res.end(JSON.stringify({id:'reply_message',outgoing:true,content:value.content,sender_name:'查看者',quote:{message_id:message.id,sender_name:'分享者',content:message.content,revision:2}}));
        }
        return res.end(JSON.stringify({snapshot_id:ref.snapshot_id,group_id:ref.group_id,owner_name:'分享者',document:{grid}}));
      }
      res.statusCode=404;res.end();
    } catch (error) { res.statusCode=500;res.end(String(error)); }
  });
  let browser;
  try {
    await fs.mkdir(output,{recursive:true});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const origin=`http://127.0.0.1:${server.address().port}`;
    if(process.argv.includes('--serve')){await fs.writeFile(path.join(output,'fixture.json'),JSON.stringify({origin,synthetic:true,route:'production message row'}));console.log('GRID_MESSAGE_FIXTURE='+origin);return;}
    browser=await chromium.launch({headless:true,channel:'msedge'});
    const page=await browser.newPage({hasTouch:true}), errors=[], external=[];
    page.on('pageerror',e=>errors.push(e.message));
    await page.route('**/*',route=>route.request().url().startsWith(origin)?route.continue():(external.push(route.request().url()),route.abort()));
    for(const theme of ['dark','light'])for(const width of [280,320,390])for(const scale of [1,2])for(const own of [true,false]){
      await page.setViewportSize({width,height:844});await page.goto(origin);
      await page.evaluate(({theme,scale,own})=>{document.documentElement.dataset.theme=theme;document.documentElement.style.fontSize=16*scale+'px';draw(own);},{theme,scale,own});
      await page.locator('.grid-share-card').scrollIntoViewIfNeeded();await page.evaluate(()=>new Promise(requestAnimationFrame));
      const overflow=await page.locator('.grid-share-card').evaluate(card=>{
        const bubble=card.parentElement,b=bubble.getBoundingClientRect();
        return {card:card.getBoundingClientRect().right>b.right+1 || card.getBoundingClientRect().left<b.left-1,
          children:[...card.querySelectorAll('*')].filter(n=>n.getBoundingClientRect().right>b.right+1 || n.getBoundingClientRect().left<b.left-1).map(n=>n.className),
          page:document.documentElement.scrollWidth>innerWidth};
      });
      assert.deepEqual(overflow,{card:false,children:[],page:false},JSON.stringify({theme,width,scale,own}));
      const cardText=await page.locator('.grid-share-card').innerText();
      assert.ok(cardText.includes('683.11265602'),JSON.stringify({theme,width,scale,own,cardText,errors}));
      if(width===390&&scale===1&&own)await page.screenshot({path:path.join(output,theme+'-message.png')});
    }
    await page.setViewportSize({width:390,height:844});await page.goto(origin);
    const card=page.locator('.grid-share-card'),menu=page.getByRole('dialog',{name:'消息操作',exact:true});
    await card.dispatchEvent('pointerdown',{pointerType:'touch',clientX:160,clientY:170});
    await card.dispatchEvent('pointermove',{pointerType:'touch',clientX:160,clientY:205});
    await page.waitForTimeout(600);assert.equal(await menu.count(),0);await card.dispatchEvent('pointercancel');
    await card.dispatchEvent('pointerdown',{pointerType:'touch',clientX:160,clientY:170});await menu.waitFor();
    await card.dispatchEvent('pointerup');await card.dispatchEvent('click');assert.equal(await page.locator('.grid-share-reader').count(),0);
    assert.ok((await menu.innerText()).includes('[网格快照] QNTUSDT · 做多 10×'));assert.ok(!(await menu.innerText()).includes('snapshot_id'));
    await menu.getByRole('button',{name:'引用',exact:true}).click();
    assert.ok((await page.locator('.social-quote-compose').innerText()).includes('[网格快照] QNTUSDT · 做多 10×'));
    await page.getByLabel('群聊草稿').fill('这条网格现在处于什么位置？');await page.locator('#send').click();
    await page.locator('[data-message-id="reply_message"]').waitFor();assert.equal(posts.length,1);assert.deepEqual(posts[0].quote_source,{message_id:message.id,revision:2});
    const quote=page.getByTitle('查看引用消息');await quote.scrollIntoViewIfNeeded();await page.evaluate(()=>new Promise(requestAnimationFrame));assert.ok((await quote.innerText()).includes('[网格快照] QNTUSDT · 做多 10×'));
    await page.evaluate(()=>{window.jumpCount=0;document.querySelector('[data-message-id="grid_message"]').scrollIntoView=()=>jumpCount++;});
    await page.waitForTimeout(850);await quote.click();assert.equal(await page.evaluate(()=>jumpCount),1);
    await page.evaluate(()=>quoteUnavailable());await page.getByTitle('查看引用消息').scrollIntoViewIfNeeded();assert.ok((await page.getByTitle('查看引用消息').innerText()).includes('原消息已撤回或不可用'));
    await page.goto(origin);await card.click();await page.getByRole('dialog',{name:'网格快照详情',exact:true}).waitFor();await page.getByRole('button',{name:'关闭',exact:true}).click();
    await card.press('Shift+F10');await menu.waitFor();await menu.getByRole('button',{name:'引用',exact:true}).click();
    await page.getByLabel('取消引用').click();assert.equal(await page.locator('.social-quote-compose').isVisible(),false);
    await card.click({button:'right'});await menu.getByRole('button',{name:'多选',exact:true}).click();
    assert.equal(await page.getByLabel('选择消息').isChecked(),true);await card.click();assert.equal(await page.getByLabel('选择消息').isChecked(),false);assert.equal(await page.locator('.grid-share-reader').count(),0);
    await page.getByRole('button',{name:'取消多选',exact:true}).click();await card.click({button:'right'});await menu.getByRole('button',{name:'引用',exact:true}).click();await page.evaluate(()=>switchGroup());assert.equal(await page.locator('.social-quote-compose').isVisible(),false);
    assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
    await fs.writeFile(path.join(output,'result.json'),JSON.stringify({passed:true,layouts:24,posts:posts.length,synthetic:true}));
    console.log('PASS grid message PWA: 24 real message-row layouts, touch hold/scroll/tap, quote source/send/return/unavailable, keyboard and selection');
  } finally {await browser?.close();if(!process.argv.includes('--serve'))await new Promise(resolve=>server.close(resolve));}
}
main().catch(error=>{console.error(error);process.exitCode=1;});
