const assert = require('node:assert/strict');
const path = require('node:path');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
async function main() {
  const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' });
  try {
    const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.setContent('<style>#list{height:360px;overflow:auto;overflow-anchor:none}#list>div{height:40px}button{min-height:48px}</style><div id="list"></div>');
    for (const name of ['message_timeline', 'social_chat_recovery']) await page.addScriptTag({ path: path.resolve(__dirname, '../server/src/assets', name + '.js') });
    await page.evaluate(async () => {
      const list = document.getElementById('list'), cache = new Map();
      const message = n => ({ id: String(n).padStart(6,'0'), created_at:'2026-10-01', content:'测试消息 ' + n, timeline_cursor: String(n) });
      window.newMessages = []; window.reads = []; window.requests = [];
      window.chat = ElonSocialChatRecovery.create({ list, cache: {get:key=>cache.get(key), put:(key,value)=>cache.set(key,value), remove:key=>cache.delete(key), clear:()=>cache.clear()},
        session:()=> 'fixture-session', userId:()=> 'fixture-user', status:()=>{}, directory:()=>{},
        api: async (path, options) => {
          const url = new URL(path, 'https://test.invalid'); window.requests.push(path);
          if (path.endsWith('/window')) return new Response(JSON.stringify({schema:'elon.message_timeline.v1',messages:JSON.parse(options.body).message_ids.map(id=>message(Number(id))),removed_ids:[],sync:'recovered',has_more:false}));
          if (options.method === 'POST') { window.reads.push(JSON.parse(options.body)); return new Response('{}'); }
          if (window.expired && url.searchParams.has('sync')) { window.expired=false; return new Response(JSON.stringify({schema:'elon.message_timeline.v1',messages:[],removed_ids:[],reset:true})); }
          let messages = [], more = false;
          if (url.searchParams.has('sync')) { messages = window.newMessages; window.newMessages = []; }
          else { const end = Number(url.searchParams.get('before') || 100000); messages = Array.from({length:Math.min(50,end)},(_,i)=>message(end-Math.min(50,end)+i)); more = end > 50; }
          return new Response(JSON.stringify({schema:'elon.message_timeline.v1',messages,removed_ids:[],has_more:more,before:messages[0]?.timeline_cursor,sync:url.searchParams.has('before')?null:'live'}));
        }, render:(rows,kind,contact,scroll)=> {
          const old = list.scrollTop;
          list.replaceChildren(...rows.map(message=> { const node=document.createElement('div'); node.dataset.messageId=message.id; node.textContent=message.content; return node; }));
          list.scrollTop=scroll ? list.scrollHeight : old;
        }
      });
      await window.chat.open('group',{id:'fixture'});
    });
    assert.equal(await page.locator('#list>div').count(), 50);
    for (let i=0;i<5;i++) {
      const anchor = await page.evaluate(() => { const list=document.getElementById('list'); list.scrollTop=0; return list.firstChild.dataset.messageId; });
      await page.getByRole('button',{name:'加载更早消息',exact:true}).click();
      await page.waitForFunction(previous=>document.querySelector('#list>div').dataset.messageId!==previous, anchor);
      const offset = await page.locator(`[data-message-id="${anchor}"]`).evaluate(node=>node.getBoundingClientRect().top-document.getElementById('list').getBoundingClientRect().top);
      assert.ok(Math.abs(offset)<2, `history anchor moved ${offset}px`);
      assert.ok(await page.locator('#list>div').count()<=150);
    }
    const before = await page.locator('#list>div').allTextContents();
    const readCount = await page.evaluate(()=>window.reads.length);
    await page.evaluate(async()=> { window.newMessages=[{id:'100000',created_at:'2026-10-02',content:'new',timeline_cursor:'100000'}]; await window.chat.refresh(); });
    assert.deepEqual(await page.locator('#list>div').allTextContents(),before);
    assert.equal(await page.evaluate(()=>window.reads.length),readCount);
    const offsetBefore = await page.locator('#list').evaluate(node=>node.scrollTop);
    await page.evaluate(async()=> { window.expired=true; await window.chat.refresh(); });
    await page.waitForFunction(()=>window.requests.some(path=>path.endsWith('/window')));
    assert.deepEqual(await page.locator('#list>div').allTextContents(),before);
    assert.equal(await page.locator('#list').evaluate(node=>node.scrollTop),offsetBefore);
    assert.equal(await page.evaluate(()=>window.reads.length),readCount);
    await page.getByRole('button',{name:'回到最新消息',exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll('#list>div').length===50);
    assert.deepEqual(errors,[]);
    await page.evaluate(()=>window.chat.destroy());
    console.log('PASS PWA timeline: 100000-row source, five historical pages, <=150 DOM rows, stable visible anchor, background changes and read receipt isolation, return to latest');
  } finally { await browser.close(); }
}
main().catch(error=>{console.error(error);process.exitCode=1;});
