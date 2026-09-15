// Exercise the real desktop page using synthetic accounts and intercepted server APIs.
const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const root = path.resolve(__dirname, '..')
const tmp = path.join(root, 'pc-frontend/.ai-tmp/social-parity.html')
const message = (id, content, own = false, extra = {}) => ({ id, content, sender_user_id: own ? 'account-a' : 'peer', sender_name: own ? '测试账号' : '测试成员', outgoing: own, created_at: new Date().toISOString(), revision: 1, ...extra })
async function main() {
  const { createServer } = await import(pathToFileURL(path.join(root, 'pc-frontend/node_modules/vite/dist/node/index.js')).href)
  const server = await createServer({ root: path.join(root, 'pc-frontend'), configFile: path.join(root,'pc-frontend/vite.config.ts'), server: { host:'127.0.0.1', port:0 }, logLevel:'error' })
  await fs.mkdir(path.dirname(tmp), { recursive:true })
  await fs.writeFile(tmp, `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0"><div id="root" style="height:100vh"></div><script type="module">
import React from 'react'; import {createRoot} from 'react-dom/client'; import {MemoryRouter} from 'react-router-dom';
import FriendsPage from '/src/features/friends/FriendsPage.tsx'; import {useAuthStore} from '/src/store/auth.ts'; import '/src/styles/globals.css';
window.signIn=id=>useAuthStore.getState().acceptSession(id,'2099-01-01',{id,account:id,nickname:'测试账号'}); window.signOut=()=>useAuthStore.getState().logout(); window.signIn('account-a');
createRoot(document.getElementById('root')).render(React.createElement(React.StrictMode,null,React.createElement(MemoryRouter,null,React.createElement(FriendsPage))));</script></body></html>`)
  let browser
  try {
    await server.listen()
    browser = await chromium.launch({ headless:true, channel:process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
    const page = await browser.newPage({ viewport:{width:1280,height:820} })
    page.setDefaultTimeout(10000)
    const errors = []; page.on('pageerror', e => errors.push(e.message))
    let groups = [message('own','自己发错的文字',true), message('peer','成员修改后的文字',false,{revision:2,edited_at:new Date().toISOString()}), message('other','第二条可转发文字'), message('old','超过撤回期限',true,{created_at:'2026-01-01T00:00:00Z'}), message('gone','',false,{recalled_at:new Date().toISOString()}), message('article','【一龙文章】\n'+JSON.stringify({schema:1,article_id:'article_fixture',revision:1,title:'独立文章卡片',summary:'文章摘要'})), message('project','【一龙项目卡片】项目链接')]
    let friends = [message('private','私聊文字',true)]
    let recallDenied = true, forwardFailure = true, uploadsFail = true, holdMember = false, memberRoute, holdUpload = false, uploadRoute, holdSend = false, sendRoute
    const sends = [], uploads = [], revisions = [], searches = []
    const members = { members:[{id:'account-a',display_name:'测试账号'},{id:'peer',display_name:'测试成员'}], ai_members:[{id:'usr_elon_ai',display_name:'EL'}] }
    const asset = { attachment_id:'a-test',kind:'image',url:'/api/test-image',display_name:'图片.png',mime_type:'image/png' }
    await page.route('**/api/**', async route => {
      const req = route.request(), url = new URL(req.url()), p = url.pathname, method = req.method()
      if (!p.startsWith('/api/')) return route.continue()
      if (p === '/api/test-image') return route.fulfill({contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="220" height="110"><rect width="220" height="110" rx="12" fill="#294139"/><text x="24" y="63" fill="#e6f6ed" font-size="20">Test attachment</text></svg>'})
      if (p.includes('/chat-attachments')) {
        uploads.push({query:url.searchParams,body:req.postDataBuffer(),authorization:req.headers().authorization})
        if (holdUpload) { uploadRoute = route; return }
        return route.fulfill({status:uploadsFail ? 503 : 200,json:uploadsFail ? {error:'上传暂不可用'} : {attachment:{...asset,display_name:url.searchParams.get('display_name')}}})
      }
      if (p.endsWith('/messages/search')) { searches.push(req.postDataJSON()); return route.fulfill({json:{retrieval:{hits:[{message:message('historic','服务器中的历史命中')}]}}}) }
      if (p.endsWith('/revisions')) return route.fulfill({json:{message_id:'peer',current_revision:2,next_before_revision:null,revisions:[{revision:2,content:'成员修改后的文字',edited_by:'peer',created_at:new Date().toISOString()},{revision:1,content:'成员最初的原文',edited_by:'peer',created_at:new Date().toISOString()}]}})
      if (method === 'PATCH' && p.includes('/messages/')) {
        revisions.push(req.postDataJSON()); const id = p.split('/').at(-1)
        groups = groups.map(m => m.id === id ? {...m,content:req.postDataJSON().content,revision:2,edited_at:new Date().toISOString()} : m)
        return route.fulfill({json:{message:groups.find(m=>m.id===id)}})
      }
      if (method === 'DELETE' && p.includes('/messages/')) {
        if (recallDenied) return route.fulfill({status:403,json:{error:'服务端拒绝撤回'}})
        groups = groups.map(m => m.id === p.split('/').at(-1) ? {...m,content:'',attachments:null,recalled_at:new Date().toISOString()} : m)
        return route.fulfill({json:{ok:true}})
      }
      if (p.endsWith('/messages')) {
        if (method === 'POST') {
          const body = req.postDataJSON(); sends.push({p,body})
          if (holdSend) { sendRoute = route; return }
          if (body.content.includes('第二条可转发文字') && body.content.startsWith('转发自') && forwardFailure) return route.fulfill({status:503,json:{error:'目标暂时忙，请重试剩余消息'}})
          const saved = message(`sent-${sends.length}`,body.content,true,{attachments:body.attachments})
          if (p.includes('/groups/')) groups.push(saved); else friends.push(saved)
          return route.fulfill({json:{message:saved}})
        }
        return route.fulfill({json:{messages:p.includes('/groups/')?groups:friends}})
      }
      if (p === '/api/me/friends') return route.fulfill({json:{friends:[{id:'f',account:'测试好友'}]}})
      if (p === '/api/me/groups') return route.fulfill({json:{groups:[{id:'g',name:'测试群聊',member_count:3}]}})
      if (p.endsWith('/members')) { if(holdMember){memberRoute=route;return} return route.fulfill({json:members}) }
      if (p.endsWith('/summary-posts')) return route.fulfill({json:{posts:[{id:'sum',title:'清晰的置顶总结',pinned_at:new Date().toISOString()}]}})
      if (p.endsWith('/summary-posts/sum')) return route.fulfill({json:{post:{id:'sum',title:'清晰的置顶总结',summary:'# 可读的总结\n\n深色背景上的正文。'}}})
      return route.fulfill({json:{}})
    })
    const row = id => page.locator(`[data-message-id="${id}"]`)
    const choose = title => page.getByRole('button').filter({has:page.locator('strong',{hasText:title})}).first().click()
    const menu = async id => { await row(id).getByRole('button',{name:'更多消息操作'}).click(); await page.getByRole('menu').waitFor() }
    const act = name => page.getByRole('menuitem',{name,exact:true}).click()
    const close = () => page.getByRole('dialog').getByRole('button',{name:/^关闭/}).click()
    const sync = () => page.getByRole('button',{name:'重新同步',exact:true}).click()
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/pc/.ai-tmp/social-parity.html`)
    await choose('测试群聊'); await row('own').waitFor()
    assert.equal(await page.getByRole('button',{name:'文章',exact:true}).count(),1)
    await row('own').focus(); await page.keyboard.press('Shift+F10'); await page.getByRole('menu').waitFor()
    const bounds = await page.getByRole('menu').boundingBox(); assert.ok(bounds.x >= 0 && bounds.y >= 0 && bounds.y + bounds.height <= 820)
    await page.keyboard.press('End'); assert.equal(await page.locator(':focus').innerText(),'仅本机隐藏')
    await page.keyboard.press('Escape'); assert.equal(await row('own').getByRole('button',{name:'更多消息操作'}).evaluate(el=>el===document.activeElement),true)
    await menu('peer'); assert.equal(await page.getByRole('menuitem',{name:'编辑消息',exact:true}).count(),0)
    await act('查看修改记录'); await page.getByText('成员最初的原文',{exact:true}).waitFor(); await close()
    await menu('own'); await act('编辑消息')
    await page.getByRole('dialog').getByRole('textbox').fill('已纠正的文字')
    await page.getByRole('button',{name:'保存修改',exact:true}).click(); await row('own').getByText('已纠正的文字',{exact:true}).waitFor()
    await page.getByRole('dialog').waitFor({state:'hidden'})
    assert.equal(revisions[0].expected_revision,1)
    assert.equal(revisions[0].content,'已纠正的文字')
    await page.locator('textarea').fill('原有草稿')
    await menu('peer'); await act('引用回复')
    assert.equal(await page.locator('textarea').inputValue(),'原有草稿')
    await page.getByLabel('待发送引用').getByText('第 2 版',{exact:false}).waitFor()
    await page.getByRole('button',{name:'取消引用',exact:true}).click(); assert.equal(await page.locator('textarea').inputValue(),'原有草稿')
    await menu('peer'); await act('引用回复'); await page.getByRole('button',{name:'发送',exact:true}).click()
    await page.waitForFunction(()=>document.querySelector('textarea').value==='')
    assert.match(sends.at(-1).body.content,/> 引用 测试成员 · 第 2 版\n> 成员修改后的文字\n\n原有草稿/)
    assert.ok(await page.locator('blockquote').count() >= 1)
    await menu('peer'); await act('收藏到本机'); await page.getByRole('button',{name:'本机收藏',exact:true}).click()
    await page.getByRole('dialog').getByText('成员修改后的文字',{exact:true}).waitFor(); await close()
    await menu('peer'); await act('仅本机隐藏'); assert.equal(await row('peer').count(),0)
    await page.getByRole('button',{name:'恢复本机隐藏（1）',exact:true}).click(); await row('peer').waitFor()
    await menu('peer'); await act('多选'); await page.getByRole('checkbox',{name:'选择消息 other',exact:true}).check()
    await page.getByRole('button',{name:'转发所选',exact:true}).click()
    assert.equal(sends.filter(s=>s.body.content.startsWith('转发自')).length,0,'opening/choosing target never sends')
    await page.getByLabel('转发目标').selectOption('friend:f'); await page.getByRole('button',{name:'确认转发 2 条',exact:true}).click()
    await page.getByText('目标暂时忙，请重试剩余消息',{exact:true}).waitFor()
    assert.equal(sends.filter(s=>s.body.content.includes('成员修改后的文字')&&s.body.content.startsWith('转发自')).length,1)
    forwardFailure=false; await page.getByRole('button',{name:'继续发送剩余 1 条',exact:true}).click()
    await page.getByText('2 / 2 条已确认发送',{exact:true}).waitFor()
    assert.equal(sends.filter(s=>s.body.content.includes('成员修改后的文字')&&s.body.content.startsWith('转发自')).length,1)
    await close(); await page.getByRole('button',{name:'退出多选',exact:true}).click()
    await menu('article'); assert.equal(await page.getByRole('menuitem',{name:/编辑消息|引用回复|转发…|多选/}).count(),0); await page.keyboard.press('Escape')
    await menu('project'); assert.equal(await page.getByRole('menuitem',{name:'编辑消息',exact:true}).count(),0); await page.keyboard.press('Escape')
    await menu('gone'); assert.equal(await page.getByRole('menuitem',{name:/编辑消息|复制|引用回复/}).count(),0); await page.keyboard.press('Escape')
    await menu('old'); assert.equal(await page.getByRole('menuitem',{name:'撤回（限发送后 1 分钟）',exact:true}).isDisabled(),true); await page.keyboard.press('Escape')
    // Refresh recent timestamp immediately before the 60-second permission check.
    groups = groups.map(m=>m.id==='own'?{...m,created_at:new Date().toISOString()}:m); await sync()
    await menu('own'); await act('撤回消息…'); await page.getByRole('button',{name:'确认撤回',exact:true}).click()
    await page.getByText('服务端拒绝撤回',{exact:true}).waitFor(); assert.ok(await row('own').getByText('已纠正的文字',{exact:true}).count())
    recallDenied=false; await page.getByRole('button',{name:'确认撤回',exact:true}).click(); await row('own').getByText('你撤回了一条消息',{exact:true}).waitFor()
    await page.getByLabel('查找已加载消息').fill('第二条'); assert.equal(await page.locator('[data-message-id]').count(),1)
    await page.getByRole('button',{name:'清除查找',exact:true}).click()
    await page.getByRole('button',{name:'群历史检索',exact:true}).click(); await page.getByLabel('群历史关键词').fill('历史')
    await page.getByRole('button',{name:'搜索群历史',exact:true}).click(); await page.getByText('服务器中的历史命中',{exact:true}).waitFor(); assert.equal(searches[0].query,'历史'); await close()
    await page.getByRole('button',{name:'置顶 / 总结',exact:true}).click(); await page.getByRole('button',{name:'置顶 · 清晰的置顶总结',exact:true}).click()
    await page.getByText('深色背景上的正文。',{exact:true}).waitFor()
    const colors = await page.getByRole('dialog').evaluate(el=>({bg:getComputedStyle(el).backgroundColor,color:getComputedStyle(el).color}))
    assert.deepEqual(colors,{bg:'rgb(23, 28, 34)',color:'rgb(242, 245, 247)'})
    const dialogBounds=await page.getByRole('dialog').boundingBox(); assert.ok(Math.abs(dialogBounds.x+dialogBounds.width/2-640)<2,'dialog remains centered despite global margin reset')
    await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-summary-dark.png')}); await close()
    holdMember=true; await page.getByRole('button',{name:'@ 群成员',exact:true}).click(); await close(); holdMember=false
    await memberRoute.fulfill({json:members}); await page.waitForTimeout(100); assert.equal(await page.getByRole('dialog').count(),0,'closed member picker must not reopen after delayed response')
    await page.getByRole('button',{name:'@ 群成员',exact:true}).click(); await page.getByRole('dialog').getByRole('button',{name:'EL',exact:true}).click()
    assert.equal(await page.locator('textarea').inputValue(),'@EL '); await page.locator('textarea').fill('')
    await page.getByLabel('选择聊天附件').setInputFiles({name:'粘贴图.png',mimeType:'image/png',buffer:Buffer.from([1,2,3])})
    await page.getByRole('button',{name:'重试上传',exact:true}).waitFor(); assert.equal(await page.getByRole('button',{name:'发送',exact:true}).isDisabled(),true)
    uploadsFail=false; await page.getByRole('button',{name:'重试上传',exact:true}).click(); await page.getByText('粘贴图.png · 待发送',{exact:true}).waitFor()
    assert.equal(uploads.at(-1).authorization,'Bearer account-a'); assert.equal(uploads.at(-1).query.get('conversation_id'),'group-g'); assert.deepEqual([...uploads.at(-1).body],[1,2,3])
    await choose('测试好友'); assert.equal(await page.getByText('粘贴图.png · 待发送',{exact:true}).count(),0)
    await menu('private'); assert.equal(await page.getByRole('menuitem',{name:'编辑消息',exact:true}).count(),0); await page.keyboard.press('Escape')
    await choose('测试群聊'); await page.getByText('粘贴图.png · 待发送',{exact:true}).waitFor(); await page.getByRole('button',{name:'发送',exact:true}).click()
    await page.getByText('粘贴图.png · 待发送',{exact:true}).waitFor({state:'hidden'}); assert.equal(sends.at(-1).body.content,''); assert.equal(sends.at(-1).body.attachments[0].attachment_id,'a-test')
    // Clipboard and drag/drop use the same raw-file upload path and keep conversation scope.
    await page.locator('textarea').evaluate(el=>{const dt=new DataTransfer();dt.items.add(new File(['paste'],'paste.png',{type:'image/png'}));el.dispatchEvent(new ClipboardEvent('paste',{clipboardData:dt,bubbles:true,cancelable:true}))})
    await page.getByText('paste.png · 待发送',{exact:true}).waitFor(); await page.getByRole('button',{name:'移除附件 paste.png',exact:true}).click()
    holdUpload=true
    await page.getByRole('region',{name:'消息编辑器'}).evaluate(el=>{const dt=new DataTransfer();dt.items.add(new File(['drop'],'drop.txt',{type:'text/plain'}));el.dispatchEvent(new DragEvent('drop',{dataTransfer:dt,bubbles:true,cancelable:true}))})
    await page.getByText('drop.txt · 上传中…',{exact:true}).waitFor(); await choose('测试好友'); holdUpload=false
    await uploadRoute.fulfill({json:{attachment:{...asset,kind:'attachment',display_name:'drop.txt'}}}); await choose('测试群聊'); await page.getByText('drop.txt · 待发送',{exact:true}).waitFor()
    await page.getByRole('button',{name:'移除附件 drop.txt',exact:true}).click()
    await page.locator('textarea').fill('输入法草稿'); const before=sends.length
    await page.locator('textarea').evaluate(el=>el.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',isComposing:true,bubbles:true,cancelable:true})))
    assert.equal(sends.length,before); assert.equal(await page.locator('textarea').inputValue(),'输入法草稿'); await page.locator('textarea').fill('')
    await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-parity.png')})
    await page.setViewportSize({width:600,height:780}); await menu('peer')
    const narrow=await page.getByRole('menu').boundingBox(); assert.ok(narrow.x>=0&&narrow.x+narrow.width<=600&&narrow.y+narrow.height<=780)
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true)
    await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-parity-narrow.png')}); await page.keyboard.press('Escape'); await page.setViewportSize({width:1280,height:820})
    // Logout while forwarding: no next item may be sent under a subsequent account.
    await menu('peer'); await act('多选'); await page.getByRole('checkbox',{name:'选择消息 other',exact:true}).check()
    await page.getByRole('button',{name:'转发所选',exact:true}).click(); await page.getByLabel('转发目标').selectOption('friend:f'); holdSend=true
    await page.getByRole('button',{name:'确认转发 2 条',exact:true}).click(); await page.waitForTimeout(100); assert.ok(sendRoute)
    const beforeLogout=sends.length; await page.evaluate(()=>{window.signOut();window.signIn('account-b')})
    await sendRoute.fulfill({json:{message:message('late','late')}}).catch(()=>{}); await page.waitForTimeout(150)
    assert.equal(sends.length,beforeLogout); assert.equal(await page.evaluate(()=>Object.keys(localStorage).filter(k=>k.startsWith('elon_social_tools_v1:')).length),0)
    assert.deepEqual(errors,[])
    console.log('PASS real PC social page: menus/focus, edit permissions and audit, quote snapshot/draft, favorites/hide/restore, partial forwarding, special cards, recall failure/success, local/server search, dark summary, stale member picker, file/paste/drop upload and scope, IME, logout cancels batch')
  } finally { if(browser)await browser.close(); await server.close(); await fs.rm(tmp,{force:true}) }
}
main().catch(error=>{console.error(error);process.exitCode=1})
