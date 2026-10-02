// Real mobile PWA renderer against synthetic authorized API fixtures; never uses an account.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const http = require('node:http');
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright');
const root = path.resolve(__dirname, '..'), assets = path.join(root, 'server/src/assets');
const output = path.join(root, '.ai-tmp/grid-share-pwa');
const runtime = ['ai_conversation_rich.js', 'grid_share_view.js', 'grid_share.js'];
const styles = ['orbital_mobile_theme.css', 'ai_conversation_share.css', 'grid_share.css'];
const grid = { schema: 'yilong.grid_share.v1', observed_at_ms: 1790842800000, show_amounts: true, fields: {
  symbol: 'QNTUSDT', direction: 'LONG', leverage: '10', lower: '275.59000000', upper: '326.29000000', count: '30', spacing: 'ARITH', status: 'WORKING',
  profit: '517.955700000', unrealizedPnl: '-162.88990000', investment: '1200.00000000', positionQty: '42.00000000', positionNotional: '11675.640000000000001', markPrice: '277.99130',
}, note: '<img src="https://evil.invalid" onerror="alert(1)">合成测试说明' };
const ref = { schema: 'elon.ai_conversation_share.v1', provider: 'binance', snapshot_id: 'ai_snapshot_fixture', group_id: 'grp_fixture', title: 'QNTUSDT 网格快照', grid };
const view = { snapshot_id: ref.snapshot_id, group_id: ref.group_id, owner_name: '合成数据', document: { title: ref.title, grid } };

async function main() {
  const mainHtml = await fs.readFile(path.join(assets, 'web_page.html'), 'utf8');
  for (const name of [...runtime, ...styles]) assert.ok(mainHtml.includes('/assets/' + name), 'page includes ' + name);
  assert.ok(mainHtml.indexOf('/assets/grid_share_view.js') < mainHtml.indexOf('/assets/grid_share.js'));
  for (const name of runtime.slice(1)) {
    const code = await fs.readFile(path.join(assets, name), 'utf8');
    assert.equal(/innerHTML\s*=|insertAdjacentHTML|\beval\(/.test(code), false);
    assert.ok(code.split('\n').length < 500);
  }
  const mainStyle = mainHtml.match(/<style>([\s\S]*?)<\/style>/)[1];
  const fixture = `<!doctype html><html lang="zh-CN" data-theme="dark"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><style>${mainStyle}</style>${styles.map(name=>`<link rel="stylesheet" href="/assets/${name}">`).join('')}<style>body{display:block;margin:0;padding:20px;box-sizing:border-box;background:var(--mobile-surface);color:var(--mobile-on-surface)}#fixture-note{font-size:12px;margin-bottom:16px}#bubble{max-width:min(100%,326px);margin:auto;border-radius:20px}</style>${runtime.map(name=>`<script src="/assets/${name}"></script>`).join('')}</head><body><p id="fixture-note">移动 PWA · 合成数据预览</p><div id="bubble" class="bubble ai-share-card-bubble"></div><script>
  const initial=${JSON.stringify(ref)};let active=true;window.apiCalls=[];
  window.draw=reference=>ElonGridShare.mount(document.querySelector('#bubble'),{content:'【一龙AI对话】\\n'+JSON.stringify(reference)},{groupId:'grp_fixture',isCurrent:()=>active,api:(url,options)=>{apiCalls.push({url,cache:options.cache,redirect:options.redirect});return fetch(url,{...options,headers:{Authorization:'Bearer synthetic-viewer'}});}});
  window.switchGroup=()=>{active=false;ElonGridShare.close();};window.draw(initial);
  </script></body></html>`;
  const state = { status: 200, delay: 0, view, requests: [] };
  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://127.0.0.1');
      if (url.pathname === '/') { res.setHeader('Content-Type', 'text/html; charset=utf-8'); return res.end(fixture); }
      if (/^\/assets\/[\w.-]+$/.test(url.pathname)) { const name=path.basename(url.pathname); res.setHeader('Content-Type',name.endsWith('.css')?'text/css':'text/javascript'); return res.end(await fs.readFile(path.join(assets,name))); }
      if (url.pathname === '/assets/grid-token-icons/qnt.png') {res.setHeader('Content-Type','image/png');return res.end(await fs.readFile(path.join(root,'android/app/src/main/assets/grid-token-icons/qnt.png')));}
      if (url.pathname.startsWith('/api/me/groups/')) {
        state.requests.push({url:req.url,authorization:req.headers.authorization,method:req.method});
        assert.equal(req.headers.authorization,'Bearer synthetic-viewer');assert.equal(url.search,'');
        const code=state.status,body=state.view;if(state.delay)await new Promise(resolve=>setTimeout(resolve,state.delay));
        res.statusCode=code;res.setHeader('Content-Type','application/json');res.setHeader('Cache-Control','private, no-store');return res.end(JSON.stringify(body));
      }
      res.statusCode=404;res.end();
    } catch(error){res.statusCode=500;res.end(String(error));}
  });
  let browser;
  try {
    await fs.mkdir(output,{recursive:true});await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const origin=`http://127.0.0.1:${server.address().port}`;
    if(process.argv.includes('--serve')){await fs.writeFile(path.join(output,'fixture.json'),JSON.stringify({origin,source:'production mobile PWA modules',synthetic:true}));console.log('GRID_PWA_FIXTURE='+origin);return;}
    browser=await chromium.launch({headless:true,channel:process.env.PLAYWRIGHT_CHANNEL||'msedge'});
    const page=await browser.newPage(),errors=[],external=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.route('**/*',route=>{if(!route.request().url().startsWith(origin)){external.push(route.request().url());return route.abort();}return route.continue();});
    const open=async()=>{await page.locator('.grid-share-card').click();await page.locator('.grid-share-details').waitFor();};
    const close=async()=>{await page.getByRole('button',{name:'关闭',exact:true}).click();assert.equal(await page.locator('dialog').count(),0);};
    for(const theme of ['light','dark']) for(const width of [320,390]) for(const scale of [1,1.5,2]){
      await page.setViewportSize({width,height:844});await page.goto(origin);
      await page.evaluate(({theme,scale})=>{document.documentElement.dataset.theme=theme;document.documentElement.style.fontSize=16*scale+'px';},{theme,scale});
      assert.ok((await page.locator('.grid-share-card').innerText()).includes('+517.9557'));
      assert.equal(await page.locator('.grid-share-logo img').evaluate(img=>img.complete&&img.naturalWidth>0),true);
      const colors=await page.locator('.grid-share-card').evaluate(card=>({positive:getComputedStyle(card.querySelector('.grid-share-profit')).color,negative:getComputedStyle(card.querySelector('.grid-share-stat b')).color,overflow:card.scrollWidth>card.clientWidth}));
      assert.notEqual(colors.positive,colors.negative);assert.equal(colors.overflow,false);
      await open();
      assert.ok((await page.locator('.grid-share-rows').innerText()).includes('11675.640000000000001'));
      for(const tab of ['收益','参数','持仓']){await page.getByRole('tab',{name:tab,exact:true}).click();assert.equal(await page.getByRole('tab',{name:tab,exact:true}).getAttribute('aria-selected'),'true');}
      assert.equal(await page.locator('.grid-share-reader').evaluate(dialog=>dialog.scrollWidth>dialog.clientWidth),false);
      assert.ok(await page.getByRole('tab',{name:'持仓',exact:true}).evaluate(node=>node.getBoundingClientRect().height>=48));
      if(width===390&&scale===1)await page.screenshot({path:path.join(output,theme+'-details.png')});
      await page.getByRole('tab',{name:'持仓',exact:true}).press('ArrowRight');assert.equal(await page.getByRole('tab',{name:'收益',exact:true}).getAttribute('aria-selected'),'true');
      await close();assert.equal(await page.locator('.grid-share-card').evaluate(node=>node===document.activeElement),true);
      if(width===390&&scale===1)await page.screenshot({path:path.join(output,theme+'-card.png')});
    }
    await page.goto(origin);
    const historical={...ref,title:'龙虾USDT 历史网格',grid:{...grid,fields:{...grid.fields,symbol:'龙虾USDT',recordKind:'HISTORY',status:'CANCELED',created:'1790800000000',end:'1790900000000',settlement:'UNKNOWN',positionState:'UNKNOWN'}}};
    state.view={...view,document:{title:historical.title,grid:historical.grid}};
    await page.evaluate(ref=>draw(ref),historical);
    assert.ok((await page.locator('.grid-share-card').innerText()).includes('龙虾USDT'));
    assert.ok((await page.locator('.grid-share-card').innerText()).includes('最终总盈亏未读取'));
    assert.ok((await page.locator('.grid-share-card').innerText()).includes('网格利润（非总盈亏）'));
    assert.equal(await page.locator('.grid-share-range-track').count(),0);
    for(const width of [320,390]){await page.setViewportSize({width,height:844});assert.equal(await page.locator('.grid-share-card').evaluate(el=>el.scrollWidth>el.clientWidth),false);}
    await open();await page.getByRole('tab',{name:'历史',exact:true}).click();
    assert.ok((await page.locator('.grid-share-rows').innerText()).includes('未确认'));
    assert.equal(await page.getByRole('tab',{name:'持仓',exact:true}).count(),0);
    await page.screenshot({path:path.join(output,'history-details.png')});await close();
    await page.evaluate(ref=>draw(ref),ref);state.view=view;
    const legacy={...ref,grid:{...grid,show_amounts:false}};
    state.view={...view,document:{...view.document,grid:legacy.grid}};
    await page.evaluate(ref=>draw(ref),legacy);assert.ok(!(await page.locator('.grid-share-card').innerText()).includes('517.9557'));
    await open();await page.getByRole('tab',{name:'收益',exact:true}).click();assert.ok(!(await page.locator('dialog').innerText()).includes('517.9557'));await close();
    await page.evaluate(ref=>draw(ref),ref);state.view=view;
    state.status=403;await page.locator('.grid-share-card').click();await page.getByText('分享已撤回或你已无权查看',{exact:true}).waitFor();state.status=200;await page.getByRole('button',{name:'重试',exact:true}).click();await page.locator('.grid-share-details').waitFor();await close();
    state.view={...view,group_id:'grp_other'};await page.locator('.grid-share-card').click();await page.getByText('分享格式不受支持',{exact:true}).waitFor();assert.equal(await page.locator('.grid-share-details').count(),0);await close();state.view=view;
    state.delay=200;await page.locator('.grid-share-card').click();await page.evaluate(()=>switchGroup());await page.waitForTimeout(250);assert.equal(await page.locator('dialog').count(),0);state.delay=0;
    const presentation=await page.evaluate(()=>({tiny:ElonGridShareView.compact('-0.00000000001',true),zero:ElonGridShareView.compact('0',true),unknown:ElonGridShareView.metric({show_amounts:true,fields:{}}),precedence:ElonGridShareView.metric({show_amounts:true,fields:{roi:'0',totalPnl:'2',profit:'3'}}).key}));
    assert.deepEqual(presentation,{tiny:'−<0.00000001',zero:'0',unknown:null,precedence:'roi'});
    assert.ok(state.requests.every(r=>r.method==='GET'));assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
    await fs.writeFile(path.join(output,'result.json'),JSON.stringify({passed:true,layouts:12,checks:['public amounts','exact details','local icon','financial colors','legacy hidden','ROI precedence','tiny/zero/unknown','tabs and focus','membership denial and retry','cross-group denial','source change abort','untrusted text'],requests:state.requests.length},null,2));
    console.log('PASS grid-share PWA: 12 theme/width/font layouts; financial fields, permissions, legacy and keyboard checks');
  } finally {if(browser)await browser.close();if(!process.argv.includes('--serve'))await new Promise(resolve=>server.close(resolve));}
}
main().catch(error=>{console.error(error);process.exitCode=1;});
