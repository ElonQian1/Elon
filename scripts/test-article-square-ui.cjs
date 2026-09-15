// Browser acceptance using synthetic accounts and a fully intercepted publishing service.
const fs = require('node:fs/promises');
const path = require('node:path');
const assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url');
const {chromium} = require('playwright');
const root=path.resolve(__dirname,'..'),temp=path.join(root,'pc-frontend','.ai-tmp');
const article={id:'article_fixture',revision:2,status:'draft',title:'一篇群聊研究笔记',author_name:'测试作者',document:{title:'一篇群聊研究笔记',summary:'群聊摘要',cover:null,blocks:[{type:'paragraph',text:'第一段正文。\n保留换行。'}]},media:{}};
async function main(){
  await fs.mkdir(temp,{recursive:true});
  await fs.writeFile(path.join(temp,'square-fixture.html'),'<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script type="module" src="./square-fixture.tsx"></script></body></html>');
  await fs.writeFile(path.join(temp,'square-fixture.tsx'),`import React from 'react';import{createRoot}from'react-dom/client';import '../src/styles/globals.css';import SquareCenter from '../src/features/articles/square/SquareCenter';window.__ELON_PC_BOOTSTRAP__={mode:'local',cloudBaseUrl:'http://43.139.149.158:8080'};localStorage.setItem('elon_auth',JSON.stringify({token:'fixture-session'}));createRoot(document.getElementById('root')!).render(<SquareCenter article={${JSON.stringify(article)}} onClose={()=>{}}/>);`);
  const {createServer}=await import(pathToFileURL(path.join(root,'pc-frontend/node_modules/vite/dist/node/index.js')).href);
  const server=await createServer({configFile:path.join(root,'pc-frontend/vite.config.ts'),root:path.join(root,'pc-frontend'),server:{host:'127.0.0.1',port:0,strictPort:false}});
  let browser;
  try{
    await server.listen();const base=`http://127.0.0.1:${server.httpServer.address().port}`;browser=await chromium.launch({channel:'chrome',headless:true});
    for(const mobile of [false,true]){
      const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900}}),page=await context.newPage();
      const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
      let account={bound:false,label:'',masked_key:'',generation:0,verified_at:null},jobs=[],keys=[],drop=true,mediaCount=0,resolveCount=0;const uploaded={},previewModes=[];
      await page.route('https://www.binance.com/**',route=>route.abort());
      await page.route(/^https?:\/\/[^/]+\/api\//,async route=>{
        const r=route.request(),url=new URL(r.url()),p=url.pathname;let data={};if(r.headers()['content-type']?.includes('application/json'))data=r.postDataJSON();
        assert.equal(url.protocol,'https:','all sensitive requests use HTTPS');
        let value={},status=200;
        if(p==='/api/auth/login')value={token:'fixture-session'};
        else{assert.equal(r.headers().authorization,'Bearer fixture-session');
          if(p.endsWith('/account')){
            if(r.method()==='PUT'){assert.equal(data.api_key,'fixture-publishing-key');account={bound:true,label:data.label,masked_key:'••••••key',generation:1,verified_at:null};}
            if(r.method()==='DELETE')account={...account,bound:false,generation:2};value=account;
          }else if(p.endsWith('/articles'))value={items:[article],next_offset:null};
          else if(p.endsWith('/articles/article_fixture'))value=article;
          else if(p.endsWith('/media')){assert.ok(Buffer.from(data.base64,'base64').length<=512*1024);value={id:'article_media_'+(++mediaCount),data_url:'data:image/jpeg;base64,'+data.base64};uploaded[value.id]=value.data_url;}
          else if(p.endsWith('/preview')){assert.equal(data.article_id,article.id);assert.equal(data.version,2);previewModes.push(data.mode);const media={};if(data.mode==='images'){assert.deepEqual(data.media_ids,['article_media_1','article_media_2']);media.article_media_2=uploaded.article_media_2;media.article_media_1=uploaded.article_media_1;}value={selection:data,title:article.title,text:'第一段正文。\n保留换行。',warnings:['摘要仅用于一龙卡片，不另加到币安正文中。'],preview_hash:'fixture-preview',generation:1,account_label:account.label,media};}
          else if(p.endsWith('/jobs')&&r.method()==='POST'){
            assert.equal(data.public_confirmed,true);assert.equal(data.conversion_confirmed,true);assert.equal(data.preview_hash,'fixture-preview');assert.equal(data.selection.mode,'text');keys.push(data.request_key);
            if(!jobs.length)jobs=[{id:'square_job_fixture',article_id:article.id,revision:2,title:article.title,mode:'text',status:'queued',scheduled_at:data.scheduled_at||Math.floor(Date.now()/1000),created_at:Math.floor(Date.now()/1000),updated_at:Math.floor(Date.now()/1000),attempts:0,message:'等待发布',post_url:null}];
            if(drop){drop=false;await route.abort('connectionfailed');return;}value=jobs[0];
          }else if(p.endsWith('/jobs'))value={items:jobs,next_offset:null};
          else if(p.endsWith('/resolve')){assert.equal(data.post_id,'123456');resolveCount++;jobs[0]={...jobs[0],status:'published',post_url:'https://www.binance.com/en/square/post/123456',message:'作者已核实'};value=jobs[0];}
          else{status=404;value={error:'unexpected fixture route'};}
        }
        await route.fulfill({status,contentType:'application/json',body:JSON.stringify(value)});
      });
      if(mobile){
        await page.route('https://43.139.149.158:8443/square*',async r=>r.fulfill({contentType:'text/html',body:await fs.readFile(path.join(root,'server/src/assets/article_square_page.html'),'utf8')}));
        await page.route('https://43.139.149.158:8443/assets/*',async r=>{const file=path.basename(new URL(r.request().url()).pathname);await r.fulfill({contentType:file.endsWith('.js')?'application/javascript':'text/css',body:await fs.readFile(path.join(root,'server/src/assets',file),'utf8')});});
        await page.goto('https://43.139.149.158:8443/square?article=article_fixture');
        await page.getByLabel('一龙账号',{exact:true}).fill('fixture');await page.getByLabel('密码',{exact:true}).fill('fixture-password');await page.getByRole('button',{name:'安全登录',exact:true}).click();
      }else await page.goto(base+'/pc/.ai-tmp/square-fixture.html');
      await page.getByLabel('账号备注',{exact:true}).fill('我的研究账号');await page.getByLabel('发帖凭证',{exact:true}).fill('fixture-publishing-key');
      await page.getByRole('button',{name:'保存绑定',exact:true}).click();await page.getByText(/凭证已保存，尚未通过实际发帖验证/).waitFor();
      assert.equal(await page.getByLabel('更换发帖凭证',{exact:true}).inputValue(),'');
      assert.equal(await page.evaluate(()=>JSON.stringify({...localStorage,...sessionStorage}).includes('fixture-publishing-key')),false);
      await page.getByRole('button',{name:'发布内容',exact:true}).click();
      await page.getByLabel('发布形式').selectOption('images');
      const png=await page.evaluate(()=>{const c=document.createElement('canvas');c.width=16;c.height=16;const x=c.getContext('2d');x.fillStyle='#38a88b';x.fillRect(0,0,16,16);return c.toDataURL().split(',')[1];});
      await page.getByLabel('添加图片',{exact:true}).setInputFiles({name:'fixture.png',mimeType:'image/png',buffer:Buffer.from(png,'base64')});
      await page.getByLabel('图片 1',{exact:true}).waitFor();
      const second=await page.evaluate(()=>{const c=document.createElement('canvas');c.width=17;c.height=17;const x=c.getContext('2d');x.fillStyle='#f6c95a';x.fillRect(0,0,17,17);return c.toDataURL().split(',')[1];});
      await page.getByLabel('添加图片',{exact:true}).setInputFiles({name:'second.png',mimeType:'image/png',buffer:Buffer.from(second,'base64')});
      await page.getByLabel('图片 2',{exact:true}).waitFor();
      await page.getByRole('button',{name:'生成币安版本预览',exact:true}).click();await page.getByText('发布到：我的研究账号',{exact:true}).waitFor();
      assert.equal(mediaCount,2);assert.deepEqual(previewModes,['images']);assert.deepEqual(await page.locator('[aria-label="币安版本预览"] img').evaluateAll(nodes=>nodes.map(n=>n.src)),[uploaded.article_media_1,uploaded.article_media_2]);
      for(const mode of ['article','images','video','text']){await page.getByLabel('发布形式').selectOption(mode);if(mode==='video')await page.getByText(/MP4\/WebM/).waitFor();}
      await page.getByRole('button',{name:'生成币安版本预览',exact:true}).click();await page.getByText('发布到：我的研究账号',{exact:true}).waitFor();
      const send=page.getByRole('button',{name:'确认公开发布',exact:true});assert.equal(await send.isDisabled(),true);
      await page.getByLabel('已核对文字转换和选定媒体',{exact:true}).check();assert.equal(await send.isDisabled(),true);
      await page.getByLabel('将此内容公开发布到上述币安账号',{exact:true}).check();
      await page.screenshot({path:path.join(root,'.ai-tmp',`square-${mobile?'mobile':'pc'}-preview.png`),fullPage:true});
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
      await send.click();await page.getByText(/安全连接未完成/).waitFor();assert.equal(await page.getByLabel('将此内容公开发布到上述币安账号',{exact:true}).isChecked(),true);
      await send.click();
      if(!mobile)await page.getByRole('button',{name:'查看发布记录',exact:true}).click();
      await page.getByRole('button',{name:'取消任务',exact:true}).waitFor();assert.equal(keys.length,2);assert.equal(keys[0],keys[1]);assert.equal(jobs.length,1);
      jobs[0]={...jobs[0],status:'uncertain',message:'请核实回执'};await page.getByRole('button',{name:'刷新记录',exact:true}).click();
      await page.getByRole('button',{name:'核实发布结果',exact:true}).click();await page.getByLabel(mobile?'帖子链接或数字ID':'已有帖子链接',{exact:true}).fill('https://www.binance.com/en/square/post/123456');
      await page.getByRole('button',{name:'记录已发布',exact:true}).click();await page.getByRole('link',{name:'打开币安帖子 ↗',exact:true}).waitFor();assert.equal(resolveCount,1);
      await page.getByRole('button',{name:'绑定账号',exact:true}).click();await page.getByRole('button',{name:'解绑账号',exact:true}).click();await page.getByLabel('发帖凭证',{exact:true}).waitFor();assert.equal(account.bound,false);
      assert.deepEqual(errors,[]);console.log(`SQUARE_UI_${mobile?'HTTPS_PWA':'PC'}=passed (bind, secret clear, image upload/selection/preview, four modes, preview consent, lost response, idempotency, history, reconcile, unbind, viewport)`);await context.close();
    }
    // A cleartext page cannot display credential fields; it points to the complete HTTPS origin.
    const page=await browser.newPage();await page.route('http://43.139.149.158:8080/square-fixture',r=>r.fulfill({contentType:'text/html',body:'<html><body></body></html>'}));await page.goto('http://43.139.149.158:8080/square-fixture');await page.addScriptTag({path:path.join(root,'server/src/assets/article_square.js')});await page.evaluate(()=>window.ElonSquare.open(()=>{throw Error('must not call API')}));assert.equal(await page.locator('input[type=password]').count(),0);assert.equal(await page.getByRole('link',{name:'打开安全页面 ↗'}).getAttribute('href'),'https://43.139.149.158:8443/square');await page.close();console.log('SQUARE_UI_HTTP_HANDOFF=passed');
  }finally{if(browser)await browser.close();await server.close();await fs.unlink(path.join(temp,'square-fixture.html'));await fs.unlink(path.join(temp,'square-fixture.tsx'));await fs.rmdir(temp).catch(()=>{});}
}
main().catch(e=>{console.error(e);process.exitCode=1;});
