// Production FriendsPage and timeline hook, real wheel input, synthetic paginated API.
const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const root = path.resolve(__dirname, '..'), fixture = path.join(root, 'pc-frontend/.ai-tmp/history-pagination.html')
const message = n => ({ id: String(n).padStart(6, '0'), content: `历史消息 ${n}`, created_at: '2026-10-01', sender_user_id: 'peer', sender_name: '成员', timeline_cursor: String(n), revision: 1 })
async function main() {
  const { createServer } = await import(pathToFileURL(path.join(root, 'pc-frontend/node_modules/vite/dist/node/index.js')).href)
  const server = await createServer({ root: path.join(root, 'pc-frontend'), configFile: path.join(root, 'pc-frontend/vite.config.ts'), server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
  await fs.mkdir(path.dirname(fixture), { recursive: true })
  await fs.writeFile(fixture, `<!doctype html><html><head><meta charset="utf-8"></head><body><div id="root" style="height:100vh"></div><script type="module">
import React from 'react';import{createRoot}from'react-dom/client';import{MemoryRouter}from'react-router-dom';
import FriendsPage from '/src/features/friends/FriendsPage.tsx';import{useAuthStore}from'/src/store/auth.ts';import'/src/styles/globals.css';
useAuthStore.getState().acceptSession('synthetic','2099-01-01',{id:'history-user',account:'history-user',nickname:'测试'});
createRoot(document.getElementById('root')).render(React.createElement(MemoryRouter,null,React.createElement(FriendsPage)));
</script></body></html>`)
  let browser
  try {
    await server.listen(); browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } }), errors = [], requests = []
    page.on('pageerror', e => errors.push(e.message))
    page.on('console', e => { if (e.type() === 'error') console.error('browser:', e.text()) })
    page.on('requestfailed', req => console.error('request failed:', req.url(), req.failure()))
    let failOlder = false, holdSync = false, heldSync, holdOlder = false, heldOlder
    const payload = url => {
      const sync = url.searchParams.has('sync'), total = url.searchParams.get('kind') === 'friend' ? 55 : 1000
      const end = Number(url.searchParams.get('before') || total), start = Math.max(0, end - 50)
      return { schema: 'elon.message_timeline.v1', messages: sync ? [] : Array.from({ length: end - start }, (_, i) => message(start + i)), removed_ids: [], sync: url.searchParams.has('before') ? null : 'checkpoint', has_more: !sync && start > 0 }
    }
    await page.route('**/api/**', async route => {
      const req = route.request(), url = new URL(req.url())
      if (!url.pathname.startsWith('/api/')) return route.continue()
      if (url.pathname === '/api/me/message-timeline') {
        requests.push(url.search)
        if (holdSync && url.searchParams.has('sync')) { heldSync = route; return }
        if (holdOlder && url.searchParams.has('before')) { heldOlder = route; return }
        if (failOlder && url.searchParams.has('before')) { failOlder = false; return route.fulfill({ status: 503, json: { error: 'fixture offline' } }) }
        return route.fulfill({ json: payload(url) })
      }
      if (url.pathname === '/api/me/friends') return route.fulfill({ json: { friends: [{ id: 'f', account: '测试好友' }] } })
      if (url.pathname === '/api/me/groups') return route.fulfill({ json: { groups: [{ id: 'g', name: '测试群聊', member_count: 2 }] } })
      return route.fulfill({ json: {} })
    })
    await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/pc/.ai-tmp/history-pagination.html`)
    const choose = async title => page.getByRole('button').filter({ has: page.locator('strong', { hasText: title }) }).first().click()
    await choose('测试群聊').catch(async error => { console.error({ errors, body: (await page.locator('body').innerText()).slice(0, 1200), requests }); throw error }); await page.locator('[data-message-id="000999"]').waitFor()
    const feed = () => page.locator('[data-message-id]').first().locator('..')
    const olderCount = () => requests.filter(q => new URLSearchParams(q).has('before')).length
    async function edge() {
      const anchor = await feed().evaluate(node => { node.scrollTop = 0; const first = node.querySelector('[data-message-id]'); return { id: first.dataset.messageId, offset: first.getBoundingClientRect().top - node.getBoundingClientRect().top } })
      await feed().hover(); await page.mouse.wheel(0, -300); return anchor
    }
    assert.equal(olderCount(), 0)
    for (let n = 0; n < 7; n++) {
      const anchor = await edge()
      await page.waitForFunction(id => document.querySelector('[data-message-id]').dataset.messageId !== id, anchor.id, { timeout: 5000 })
      const delta = await page.locator(`[data-message-id="${anchor.id}"]`).evaluate(node => node.getBoundingClientRect().top - node.parentElement.getBoundingClientRect().top)
      assert.ok(Math.abs(delta - anchor.offset) < 3, `visible anchor moved ${delta - anchor.offset}px`)
      assert.ok(await page.locator('[data-message-id]').count() <= 300)
    }
    // A gesture during polling is deferred, then loaded once, without requiring a second gesture.
    holdSync = true; await page.getByRole('button', { name: '重新同步', exact: true }).click()
    await page.waitForTimeout(150); assert.ok(heldSync)
    const oldCount = olderCount(); const anchor = await edge(); assert.equal(olderCount(), oldCount)
    holdSync = false; await heldSync.fulfill({ json: payload(new URL(heldSync.request().url())) })
    await page.waitForFunction(id => document.querySelector('[data-message-id]').dataset.messageId !== id, anchor.id)
    assert.equal(olderCount(), oldCount + 1)
    // Repeated wheel events while this page is held do not enqueue duplicate pages.
    holdOlder = true; await edge(); await page.waitForTimeout(100); assert.ok(heldOlder)
    const heldCount = olderCount(); for (let n = 0; n < 4; n++) await page.mouse.wheel(0, -50)
    assert.equal(olderCount(), heldCount)
    const heldAnchor = await page.locator('[data-message-id]').first().getAttribute('data-message-id')
    holdOlder = false; await heldOlder.fulfill({ json: payload(new URL(heldOlder.request().url())) })
    await page.waitForFunction(id => document.querySelector('[data-message-id]').dataset.messageId !== id, heldAnchor)
    assert.equal(olderCount(), heldCount)
    failOlder = true; const failed = await edge()
    await page.getByText('消息同步失败，已保留现有内容', { exact: false }).waitFor()
    assert.equal(await page.locator('[data-message-id]').first().getAttribute('data-message-id'), failed.id)
    await page.getByRole('button', { name: '加载更早消息', exact: true }).click()
    await page.waitForFunction(id => document.querySelector('[data-message-id]').dataset.messageId !== id, failed.id)
    await choose('测试好友'); await page.locator('[data-message-id="000054"]').waitFor()
    await edge(); await page.locator('[data-message-id="000000"]').waitFor()
    assert.equal(await page.getByRole('button', { name: '加载更早消息', exact: true }).count(), 0)
    const endCount = olderCount(); await edge(); await page.waitForTimeout(100); assert.equal(olderCount(), endCount)
    assert.deepEqual(errors, [])
    console.log('PASS PC history: production page, wheel pagination, seven pages, <=300 rows, stable anchor, busy poll, dedup, retry, friend scope and end of history')
  } finally { await browser?.close(); await server.close(); await fs.rm(fixture, { force: true }) }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
