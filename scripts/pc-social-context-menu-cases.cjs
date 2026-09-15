const assert = require('node:assert/strict')
const path = require('node:path')
const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=', 'base64')
function mediaMessages(message) {
  return [message('media', '', false, { attachments: [
    { kind:'image', url:'/api/menu-image', display_name:'测试图片.png', mime_type:'image/png' },
    { kind:'audio', url:'/api/menu-audio', display_name:'测试语音.wav', mime_type:'audio/wav', transcription:'这是转写文字' },
    { kind:'attachment', url:'/api/menu-file', display_name:'测试文件.txt', mime_type:'text/plain' }
  ] })]
}
async function routeMedia(route) {
  const url = new URL(route.request().url())
  if (url.pathname === '/api/menu-image') { await route.fulfill({contentType:'image/png',body:png}); return true }
  if (url.pathname === '/api/menu-file') { await route.fulfill({contentType:'text/plain',body:'download fixture'}); return true }
  if (url.pathname === '/api/menu-audio') {
    const audio = Buffer.alloc(44 + 16000 * 10)
    audio.write('RIFF'); audio.writeUInt32LE(audio.length-8,4); audio.write('WAVEfmt ',8); audio.writeUInt32LE(16,16)
    audio.writeUInt16LE(1,20); audio.writeUInt16LE(1,22); audio.writeUInt32LE(8000,24); audio.writeUInt32LE(16000,28)
    audio.writeUInt16LE(2,32); audio.writeUInt16LE(16,34); audio.write('data',36); audio.writeUInt32LE(audio.length-44,40)
    await route.fulfill({contentType:'audio/wav',body:audio}); return true
  }
  return false
}
async function run({page,row,root}) {
  await page.context().grantPermissions(['clipboard-read','clipboard-write'])
  const menu = page.getByRole('menu'), act = label => page.getByRole('menuitem',{name:label,exact:true}).click()
  const rightClick = async target => { await target.click({button:'right'}); await menu.waitFor() }
  await rightClick(row('own').locator('[data-social-content]'))
  assert.equal(await menu.count(),1)
  assert.equal(await menu.getByRole('separator').count(),2)
  assert.equal(await menu.getByRole('menuitem',{name:'复制富文本',exact:true}).count(),0)
  await act('复制'); assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),'自己发错的文字')
  await row('own').locator('[data-social-content]').evaluate(node=>{
    const text = node.querySelector('[id]').firstChild, range=document.createRange()
    range.setStart(text,0); range.setEnd(text,4); const selection=getSelection(); selection.removeAllRanges(); selection.addRange(range)
  })
  // Keyboard invocation retains a precise selection, independently of pointer word-selection policy.
  await row('own').focus(); await page.keyboard.press('Shift+F10'); await act('复制选中文字')
  assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),'自己发错')
  await rightClick(row('peer').locator('[data-social-content]'))
  assert.equal(await menu.getByRole('menuitem',{name:'复制选中文字',exact:true}).count(),0)
  await act('复制'); assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),'成员修改后的文字')
  await page.evaluate(()=>getSelection().removeAllRanges())
  await rightClick(row('peer').locator('[data-social-content]')); await page.keyboard.press('Escape')
  assert.equal(await row('peer').evaluate(node=>node===document.activeElement),true)
  // Repeated primary/secondary triggers must never leave two portals alive.
  await row('own').getByRole('button',{name:'更多消息操作'}).click()
  await rightClick(row('peer').locator('[data-social-content]')); assert.equal(await menu.count(),1)
  await page.keyboard.press('End'); assert.match(await page.locator(':focus').innerText(),/隐藏消息/)
  await page.keyboard.press('Home'); await page.keyboard.press('ArrowDown'); assert.match(await page.locator(':focus').innerText(),/引用/)
  await page.keyboard.press('Escape')
  await rightClick(row('own').locator('[data-social-content]')); await page.locator('textarea').click(); assert.equal(await menu.count(),0)
  assert.equal(await page.locator('textarea').evaluate(el=>el.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true}))),true)
  await rightClick(row('own').locator('[data-social-content]')); await page.evaluate(()=>window.dispatchEvent(new Event('blur'))); assert.equal(await menu.count(),0)
  const image = row('media').locator('[data-social-attachment="0"]')
  const audio = row('media').locator('[data-social-attachment="1"]')
  const file = row('media').locator('[data-social-attachment="2"]')
  await rightClick(image); assert.equal(await menu.getByRole('menuitem',{name:'播放语音'}).count(),0)
  await act('复制图片'); await row('media').getByRole('status').waitFor({timeout:4000}); assert.equal(await row('media').getByRole('status').innerText(),'已复制图片')
  assert.ok(await page.evaluate(async()=>(await navigator.clipboard.read())[0].types.includes('image/png')))
  await rightClick(image); await act('查看图片'); await page.getByRole('dialog',{name:'图片预览：测试图片.png'}).waitFor()
  await page.getByRole('dialog').locator('img').evaluate(el=>el.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true})))
  assert.equal(await menu.count(),0); await page.getByRole('button',{name:'关闭图片预览'}).click()
  await rightClick(audio); assert.equal(await menu.getByRole('menuitem',{name:'复制图片'}).count(),0)
  await act('播放语音'); await page.waitForFunction(()=>!document.querySelector('[data-message-id="media"] audio').paused)
  await rightClick(audio); await act('暂停语音'); assert.equal(await audio.locator('audio').evaluate(el=>el.paused),true)
  await rightClick(audio); await act('复制转写文字'); assert.equal(await page.evaluate(()=>navigator.clipboard.readText()),'这是转写文字')
  await page.route('**/api/menu-file',route=>route.fulfill({status:503,body:'temporarily unavailable'}))
  await rightClick(file); await act('下载文件'); await row('media').getByText('附件读取失败，请稍后重试',{exact:true}).waitFor()
  await page.unroute('**/api/menu-file')
  await rightClick(file); const downloadPromise=page.waitForEvent('download'); await act('下载文件'); const download=await downloadPromise
  assert.equal(download.suggestedFilename(),'测试文件.txt')
  const downloadPath=await download.path(); assert.equal(await require('node:fs/promises').readFile(downloadPath,'utf8'),'download fixture')
  // Exercise edge anchoring and desktop zoom with actual contextmenu events.
  for (const zoom of [1,1.5,2]) {
    await page.evaluate(value=>document.documentElement.style.zoom=String(value),zoom)
    await row('own').scrollIntoViewIfNeeded()
    await row('own').locator('[data-social-content]').evaluate(el=>el.dispatchEvent(new MouseEvent('contextmenu',{clientX:1278,clientY:818,bubbles:true,cancelable:true})))
    await menu.waitFor(); const bounds=await menu.boundingBox()
    assert.ok(bounds.x>=0&&bounds.y>=0&&bounds.x+bounds.width<=1280&&bounds.y+bounds.height<=820,`menu fits at zoom ${zoom}: ${JSON.stringify(bounds)}; ${await menu.evaluate(el=>JSON.stringify({style:el.getAttribute('style'),position:getComputedStyle(el).position,zoom:getComputedStyle(document.documentElement).zoom}))}`)
    await page.keyboard.press('Escape')
  }
  await page.evaluate(()=>document.documentElement.style.zoom='1')
  await page.setViewportSize({width:600,height:780}); await rightClick(row('peer').locator('[data-social-content]'))
  const bounds=await menu.boundingBox(); assert.ok(bounds.x+bounds.width<=600&&bounds.y+bounds.height<=780)
  await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-menu-narrow.png')}); await page.keyboard.press('Escape')
  await page.setViewportSize({width:1280,height:820}); await rightClick(row('peer').locator('[data-social-content]'))
  await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-menu-desktop.png')}); await page.keyboard.press('Escape')
  await rightClick(row('media').locator('[data-social-attachment="1"]'))
  await page.screenshot({path:path.join(root,'.ai-tmp/pc-social-menu-audio.png')}); await page.keyboard.press('Escape')
  await rightClick(row('own').locator('[data-social-content]'))
  await row('own').evaluate(el=>{el.parentElement.scrollTop+=80}); await menu.waitFor({state:'hidden'})
}
module.exports={mediaMessages,routeMedia,run}
