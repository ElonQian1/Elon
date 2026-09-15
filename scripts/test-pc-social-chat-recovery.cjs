// Actual React page + notification hook, isolated accounts and intercepted APIs.
const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const root = path.resolve(__dirname, '..')
const fixture = path.join(root, 'pc-frontend/.ai-tmp/social-recovery.html')
const makeMessage = (id, content) => ({ id, content, sender_user_id: 'peer', sender_name: '测试成员', created_at: '2026-09-15T08:00:00Z', outgoing: false, revision: 1 })

async function main() {
  const { createServer } = await import(pathToFileURL(path.join(root, 'pc-frontend/node_modules/vite/dist/node/index.js')).href)
  const server = await createServer({ root: path.join(root, 'pc-frontend'), configFile: path.join(root, 'pc-frontend/vite.config.ts'), server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
  await fs.mkdir(path.dirname(fixture), { recursive: true })
  await fs.writeFile(fixture, `<!doctype html><html><head><meta charset="utf-8"></head><body><div id="root" style="height:100vh"></div><script type="module">
import React from 'react'; import {createRoot} from 'react-dom/client'; import {MemoryRouter} from 'react-router-dom';
import FriendsPage from '/src/features/friends/FriendsPage.tsx'; import {useAuthStore} from '/src/store/auth.ts';
import {useNotifications} from '/src/features/notifications/useNotifications.ts'; import '/src/styles/globals.css';
import * as cache from '/src/features/friends/socialChatCache.ts';
window.cacheApi=cache; window.sockets=[];
class FakeSocket { static OPEN=1; constructor(){this.readyState=1;window.sockets.push(this);queueMicrotask(()=>this.onopen?.({}));} close(){this.readyState=3;this.onclose?.({});} }
window.WebSocket=FakeSocket;
window.signIn=id=>useAuthStore.getState().acceptSession(id,'2099-01-01',{id,account:id,nickname:'测试账号'});
window.signOut=()=>useAuthStore.getState().logout();
window.pushChat=type=>window.sockets.at(-1)?.onmessage?.({data:JSON.stringify({type,groupId:'g'})});
if(!useAuthStore.getState().token)window.signIn('account-a');
function App(){useNotifications();return React.createElement(MemoryRouter,null,React.createElement(FriendsPage));}
createRoot(document.getElementById('root')).render(React.createElement(App));</script></body></html>`)
  let browser
  try {
    await server.listen()
    browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } })
    const errors = []; page.on('pageerror', error => errors.push(error.message))
    let failLists = true, holdFriends = false, holdMessages = false, denyMessages = false, failSend = false, heldSend
    let groupMessages = [makeMessage('g1', '缓存中的群聊内容')], friendMessages = [makeMessage('f1', '好友第一条消息')]
    const held = [], requests = []
    await page.route('**/api/**', async route => {
      const req = route.request(), url = new URL(req.url()), pathname = url.pathname
      if (!pathname.startsWith('/api/')) return route.continue()
      requests.push({ path: pathname, query: url.search, method: req.method(), user: req.headers().authorization })
      if (pathname.endsWith('/messages')) {
        if (req.method() === 'POST') {
          if (!failSend) { heldSend = route; return }
          return route.fulfill({ status: 503, json: { error: '测试发送失败' } })
        }
        if (holdMessages) { held.push(route); return }
        return route.fulfill({ status: denyMessages ? 403 : 200, json: denyMessages ? { error: '无权访问' } : { messages: pathname.includes('/groups/') ? groupMessages : friendMessages } })
      }
      if (pathname === '/api/me/friends') {
        if (holdFriends) { held.push(route); return }
        return route.fulfill({ status: failLists ? 503 : 200, json: failLists ? { error: '暂时离线' } : { friends: [{ id: 'f', account: '测试好友', last_message: '好友预览' }] } })
      }
      if (pathname === '/api/me/groups') return route.fulfill({ status: failLists ? 503 : 200, json: failLists ? { error: '暂时离线' } : { groups: [{ id: 'g', name: '测试群聊', last_message: '群聊预览', member_count: 2 }] } })
      return route.fulfill({ json: {} })
    })
    const url = `http://127.0.0.1:${server.httpServer.address().port}/pc/.ai-tmp/social-recovery.html`
    const text = content => page.getByText(content, { exact: true })
    const seen = async content => { await text(content).waitFor({ state: 'visible', timeout: 10000 }) }
    const choose = async title => page.getByRole('button').filter({ has: page.locator('strong', { hasText: title }) }).first().click()
    const sync = async () => page.getByRole('button', { name: '重新同步', exact: true }).click()
    await page.goto(url)
    await page.getByText('会话同步失败，已保留现有列表；将自动重试', { exact: false }).waitFor({ timeout: 10000 }).catch(async error => {
      console.error({ errors, body: (await page.locator('body').innerText()).slice(0, 1000), requests: requests.slice(0, 6) }); throw error
    })
    assert.equal(await text('暂无好友或群聊，搜索手机号添加好友').count(), 0)
    failLists = false; holdFriends = true
    await sync(); await seen('缓存中的群聊内容')
    assert.equal(await text('正在加载好友…').count() + await text('好友列表暂不可用').count(), 1)
    holdFriends = false; await sync(); await choose('测试好友'); await seen('好友第一条消息')
    friendMessages.push(makeMessage('f2', '私聊即时更新'))
    await page.evaluate(() => window.pushChat('friend_message')); await seen('私聊即时更新')
    await page.locator('textarea').fill('好友草稿')
    await choose('测试群聊'); await seen('缓存中的群聊内容')
    await page.locator('textarea').fill('群聊草稿')
    await choose('测试好友'); assert.equal(await page.locator('textarea').inputValue(), '好友草稿')
    await choose('测试群聊'); assert.equal(await page.locator('textarea').inputValue(), '群聊草稿')
    holdMessages = true; failLists = true
    await page.reload(); await seen('缓存中的群聊内容')
    await page.screenshot({ path: path.join(root, '.ai-tmp/pc-social-cache.png') })
    assert.equal(await page.locator('textarea').inputValue(), '群聊草稿')
    await page.getByText('会话同步失败', { exact: false }).waitFor()
    await page.clock.install(); await page.clock.fastForward(12500)
    await page.getByText('消息同步失败，已保留现有内容', { exact: false }).waitFor()
    holdMessages = false; failLists = false
    groupMessages.push(makeMessage('g2', '回到前台后补齐的消息'))
    const socketsBefore = await page.evaluate(() => window.sockets.length)
    await page.evaluate(() => window.dispatchEvent(new Event('focus')))
    await seen('回到前台后补齐的消息')
    assert.ok(await page.evaluate(() => window.sockets.length) > socketsBefore, 'replace a stale OPEN socket on resume')
    groupMessages[0] = { ...groupMessages[0], content: '群聊修订后的文字', revision: 2, edited_at: '2026-09-15T09:00:00Z' }
    await page.evaluate(() => { for(let i=0;i<8;i++)window.pushChat('group_message_edited') })
    await page.clock.runFor(400); await seen('群聊修订后的文字')
    assert.equal(await text('群聊修订后的文字').count(), 1)
    await page.evaluate(() => Object.defineProperty(document, 'hidden', { configurable: true, value: true }))
    await sync(); await page.clock.runFor(500)
    assert.ok(requests.some(r => r.path.includes('/groups/g/messages') && r.query.includes('preserve_unread=true')))
    groupMessages.push(makeMessage('g3', '网络恢复补齐'))
    await page.evaluate(() => { Object.defineProperty(document,'hidden',{configurable:true,value:false}); window.dispatchEvent(new Event('online')) })
    await seen('网络恢复补齐')
    holdMessages = true; await sync(); await choose('测试好友')
    for (const route of held.splice(0)) await route.fulfill({ json: { messages: [makeMessage('stale','不应串入当前会话')] } }).catch(() => {})
    assert.equal(await text('不应串入当前会话').count(), 0)
    holdMessages = false; await sync(); await seen('私聊即时更新')
    failSend = true; await page.locator('textarea').fill('失败要保留的草稿'); await page.locator('textarea').press('Enter')
    await seen('测试发送失败'); assert.equal(await page.locator('textarea').inputValue(), '失败要保留的草稿')
    failSend = false; await page.locator('textarea').fill('回执与推送竞速'); await page.locator('textarea').press('Enter')
    await page.waitForFunction(() => document.body.textContent.includes('回执与推送竞速'))
    const sent = { ...makeMessage('sent','回执与推送竞速'), sender_user_id:'account-a', outgoing:true }
    friendMessages.push(sent); await sync(); await page.clock.runFor(400)
    assert.ok(heldSend)
    await heldSend.fulfill({ json: { message: sent } }); await page.clock.runFor(400)
    assert.equal(await text('回执与推送竞速').count(), 1)
    denyMessages = true; await sync(); await page.getByText('无法访问此会话', { exact: false }).waitFor()
    assert.equal(await text('私聊即时更新').count(), 0)
    denyMessages = false; failLists = true; holdMessages = true
    await page.evaluate(() => window.signIn('account-b'))
    assert.equal(await text('缓存中的群聊内容').count(), 0)
    assert.equal(await page.locator('textarea').count(), 0)
    await page.evaluate(() => window.signOut())
    assert.equal(await page.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('elon_social_cache_v1:')).length), 0)
    // Corruption/quota failure is contained; cache never stores pending optimistic sends.
    const cacheChecks = await page.evaluate(() => {
      const api = window.cacheApi, key = api.socialCacheKey('server', 'account')
      localStorage.setItem(key, '{broken'); const corrupt = api.readSocialCache(key)
      const snapshot = { friends: [], groups: [], active: {kind:'group',id:'g'}, messages: {'group:g': [{id:'tmp-1', content:'pending',created_at:'now'}]}, drafts: {} }
      api.writeSocialCache(key, snapshot); const saved = api.readSocialCache(key)
      const original = Storage.prototype.setItem; Storage.prototype.setItem=()=>{throw new Error('quota')}
      const quota = api.writeSocialCache(key,snapshot); Storage.prototype.setItem=original
      return { corrupt: corrupt.active, pending: saved.messages['group:g'].length, quota }
    })
    assert.deepEqual(cacheChecks, { corrupt: null, pending: 0, quota: false })
    assert.deepEqual(errors, [])
    console.log('PASS PC chat: failed/partial lists, realtime friend updates, per-chat drafts, cached reload, timeout, stale OPEN reconnect, edits, background unread, online recovery, stale responses, send failure, permission denial, account/logout isolation, corrupt/quota cache')
  } finally {
    await browser?.close(); await server.close(); await fs.rm(fixture, { force: true })
  }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
