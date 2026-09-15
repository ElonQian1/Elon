const fs = require('node:fs/promises'), path = require('node:path'), http = require('node:http'), assert = require('node:assert/strict')
const { pathToFileURL } = require('node:url')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright')
const root = path.resolve(__dirname, '..'), pc = path.join(root, 'pc-frontend'), assets = path.join(root, 'server/src/assets')
const fixture = path.join(root, 'scripts/fixtures/social-source-qr.png')
const url = 'https://mp.weixin.qq.com/s/synthetic-test?scene=90'
async function main() {
  const { createServer } = await import(pathToFileURL(path.join(pc, 'node_modules/vite/dist/node/index.js')).href)
  const server = await createServer({ root: pc, configFile: path.join(pc, 'vite.config.ts'), optimizeDeps: { include: ['jsqr'] }, server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
  const entry = path.join(pc, '.ai-tmp/source-browser.html'); await fs.mkdir(path.dirname(entry), { recursive: true })
  await fs.writeFile(entry, `<!doctype html><html><head><meta charset="utf-8"></head><body style="background:#07090d;color:#d0d0d0"><div id="root"></div><script type="module">
import React from 'react'; import {createRoot} from 'react-dom/client';
import SocialMessageAttachments from '/src/features/friends/SocialMessageAttachments.tsx';
import {scanImageLinks} from '/src/features/friends/source-links/scanImageLinks.ts';
import {TextSourceCard} from '/src/features/friends/source-links/SourceLinkView.tsx';
window.scanImageLinks=scanImageLinks;
createRoot(document.getElementById('root')).render(React.createElement('div',null,
React.createElement(SocialMessageAttachments,{attachments:[{kind:'image',url:'/fixture.png',source_link:{version:1,url:${JSON.stringify(url)},method:'qr'}}]}),
React.createElement(TextSourceCard,{text:'https://example.com/article'})));
</script></body></html>`)
  let browser, mobile
  try {
    await server.listen(); const origin = 'http://127.0.0.1:' + server.httpServer.address().port
    browser = await chromium.launch({ channel: 'msedge', headless: true }); const page = await browser.newPage({viewport:{width:1100,height:800}})
    const errors = []; page.on('pageerror', e => errors.push(e.message))
    await page.route('**/fixture.png', route => route.fulfill({contentType:'image/png',path:fixture}))
    await page.route('https://**', route => route.abort())
    await page.goto(origin + '/pc/.ai-tmp/source-browser.html')
    await page.waitForLoadState('networkidle')
    const original = page.getByRole('link',{name:/阅读原文/}); await original.waitFor(); assert.equal(await original.getAttribute('href'),url)
    assert.equal(await page.getByRole('link',{name:/打开链接/}).count(),1)
    const links = await page.evaluate(async()=>window.scanImageLinks(await (await fetch('/fixture.png')).blob()))
    assert.deepEqual(links.map(x=>x.url).sort(),[url,'https://example.com/other'].sort())
    await page.getByRole('button',{name:/查看图片/}).click(); await page.getByRole('button',{name:'识别二维码'}).click()
    await page.getByText('选择要打开的链接').waitFor(); assert.equal(await page.getByRole('dialog').getByRole('link',{name:/阅读原文/}).count(),1)
    await fs.mkdir(path.join(root,'.ai-tmp/source-proof'),{recursive:true}); await page.screenshot({path:path.join(root,'.ai-tmp/source-proof/pc.png')})
    let requests = 0, payload
    mobile = http.createServer(async(req,res)=>{
      const p = new URL(req.url,'http://localhost').pathname
      if(p.startsWith('/assets/') && !p.includes('..')) { try {res.setHeader('content-type','application/javascript');res.end(await fs.readFile(path.join(assets,p.slice(8))));}catch{res.writeHead(404);res.end();} return }
      if(p==='/fixture.png'){res.setHeader('content-type','image/png');res.end(await fs.readFile(fixture));return}
      res.setHeader('content-type','text/html');res.end('<meta charset="utf-8"><script src="/assets/social_source_links.js"></script><script src="/assets/social_source_compose.js"></script><div id="messages"></div><button id="share">添加图片</button>')
    })
    await new Promise(r=>mobile.listen(0,'127.0.0.1',r)); const m = await browser.newPage({viewport:{width:412,height:915}})
    m.on('pageerror',e=>errors.push(e.message)); await m.route('https://**',route=>route.abort())
    await m.goto('http://127.0.0.1:'+mobile.address().port)
    await m.exposeFunction('sentFixture', body=>{requests++;payload=body})
    await m.evaluate(()=>{
      const owner='synthetic'; const opts={api:async()=>new Response(JSON.stringify({attachment:{kind:'image',url:'/fixture.png'}}),{headers:{'content-type':'application/json'}}),owner:()=>owner,userId:'test',kind:'group',contact:{id:'g',name:'测试群'},send:async(kind,contact,content,attachments)=>window.sentFixture({content,attachments})}
      document.querySelector('#share').onclick=()=>ElonSourceCompose.open(opts)
    })
    const chooser = m.waitForEvent('filechooser'); await m.getByRole('button',{name:'添加图片'}).click(); await (await chooser).setFiles(fixture)
    const selector=m.getByRole('combobox',{name:'选择图片原文链接'}); await selector.waitFor(); assert.equal(await selector.locator('option').count(),3)
    await selector.selectOption(url); await m.screenshot({path:path.join(root,'.ai-tmp/source-proof/mobile.png')})
    await m.getByRole('button',{name:'发送',exact:true}).click(); await m.getByRole('dialog').waitFor({state:'detached'})
    assert.equal(requests,1); assert.equal(payload.attachments[0].source_link.url,url); assert.equal(payload.attachments[0].source_link.method,'qr')
    assert.deepEqual(errors,[])
    console.log('PASS: real PC/PWA worker multi-QR decode, reading link, manual recognition, preview selection, single confirmed synthetic send')
  } finally { await browser?.close(); await server.close(); await fs.rm(entry,{force:true}); await new Promise(resolve=>mobile?mobile.close(resolve):resolve()) }
}
main().catch(error=>{console.error(error);process.exitCode=1})
