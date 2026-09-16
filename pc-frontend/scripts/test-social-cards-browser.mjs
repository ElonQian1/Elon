import assert from 'node:assert/strict'
import { mkdir, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import path from 'node:path'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const base = process.env.CARD_FIXTURE_URL || 'http://127.0.0.1:5196/pc/tests/fixtures/social-cards.html'
const output = path.resolve(process.env.CARD_EVIDENCE_DIR || '../.ai-tmp/card-polish-browser')
await mkdir(output, { recursive: true })
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) })
const results = []
try {
  for (const ready of [false, true]) {
    const context = await browser.newContext({ viewport: { width: 1100, height: 1050 }, permissions: ['clipboard-read', 'clipboard-write'] })
    await context.addInitScript(() => {
      window.__cardNativeCalls = []
      // IPC transport double only: this is not proof that a real Tauri WebView opened.
      window.__TAURI__ = { core: { invoke: async (command, args) => { window.__cardNativeCalls.push({ command, args }); return {} } } }
    })
    const page = await context.newPage(); const errors = []
    page.on('pageerror', error => errors.push(error.message))
    await page.route('**/*', async route => {
      const url = new URL(route.request().url())
      if (url.pathname === '/api/me/link-preview') {
        const request = route.request().postDataJSON()
        // Only metadata is replaced. All rendering, menu, copy and quote code is production.
        await route.fulfill({ json: { schema: 1, url: request.url, title: ready ? '城市观察：关于日常生活的一段真实记录' : '',
          author: ready ? '示例作者' : '', image: ready ? 'https://cover.example/fixture.svg' : null, status: ready ? 'ready' : 'unavailable' } })
      } else if (url.hostname === 'cover.example') {
        await route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="112" height="112"><rect width="112" height="112" fill="#547569"/><path d="M0 90L35 45L62 75L87 28L112 67V112H0" fill="#b6ccb5"/></svg>' })
      } else if (url.origin === new URL(base).origin && !url.pathname.startsWith('/api/')) await route.continue()
      else await route.abort()
    })
    await page.goto(base); await page.locator('[data-message-id]').first().waitFor()
    await page.locator('.social-link-card').first().scrollIntoViewIfNeeded()
    assert.equal(await page.locator('.social-link-card').count(), 7)
    const card = id => page.locator(`[data-message-id="fixture-${id}"] .social-link-card`)
    for (let i = 0; i < 6; i++) {
      await card(i).scrollIntoViewIfNeeded()
      const row = page.locator(`[data-message-id="fixture-${i}"]`)
      assert.equal(await row.locator('[data-social-content] > [id][hidden]').count(), 1)
      assert.ok(await card(i).getAttribute('title'))
      assert.ok((await card(i).locator('.social-link-source').innerText()).length)
      await card(i).click({ button: 'right' })
      for (const name of ['打开链接', '复制链接', '复制', '引用', '转发…']) assert.equal(await page.getByRole('menuitem', { name, exact: true }).count(), 1)
      await page.keyboard.press('Escape')
    }
    assert.match(await card(4).innerText(), /币安广场/)
    assert.match(await card(3).innerText(), /从 01:20 开始/)
    assert.match(await page.locator('[data-message-id="fixture-6"]').innerText(), /附加评论必须保留/)
    if (!ready) {
      assert.match(await card(1).innerText(), /小红书 · 测试作者/)
      assert.match(await card(5).innerText(), /X · @example/)
    }
    await card(1).click({ button: 'right' }); await page.getByRole('menuitem', { name: '复制链接', exact: true }).click()
    assert.match(await page.evaluate(() => navigator.clipboard.readText()), /xsec_token=synthetic/)
    await card(1).click({ button: 'right' }); await page.getByRole('menuitem', { name: '复制', exact: true }).click()
    assert.match(await page.evaluate(() => navigator.clipboard.readText()), /^45 【城市散步/)
    const input = page.locator('textarea'); await input.fill('尚未发送的草稿')
    await card(3).focus(); await page.keyboard.press('Shift+F10')
    await page.getByRole('menuitem', { name: '引用', exact: true }).click()
    assert.match(await page.getByLabel('待发送引用').innerText(), /t=80&p=3/)
    assert.equal(await input.inputValue(), '尚未发送的草稿')
    await page.getByRole('button', { name: '取消引用' }).click()
    // Test actual viewer and explicit original link; external site content is not contacted.
    await card(4).focus(); await page.keyboard.press('Enter')
    await page.locator('dialog[open]').waitFor()
    await page.waitForFunction(() => window.__cardNativeCalls.some(call => call.command === 'open_internal_browser_tab'))
    const opened = await page.evaluate(() => window.__cardNativeCalls.find(call => call.command === 'open_internal_browser_tab').args.url)
    assert.equal(opened, await card(4).getAttribute('href'))
    assert.equal(await page.locator('dialog[open] a').filter({ hasText: '打开原文' }).getAttribute('href'), await card(4).getAttribute('href'))
    await page.locator('dialog[open]').getByRole('button', { name: '关闭', exact: true }).click()
    await page.waitForFunction(() => window.__cardNativeCalls.some(call => call.command === 'control_internal_browser_tab' && call.args?.action === 'close'))
    await card(3).click({ button: 'right' }); await page.getByRole('menuitem', { name: '打开链接', exact: true }).click()
    await page.waitForFunction(() => window.__cardNativeCalls.some(call => call.command === 'open_internal_browser_tab' && call.args?.url.includes('player.bilibili.com')))
    const video = await page.evaluate(() => window.__cardNativeCalls.find(call => call.args?.url?.includes('player.bilibili.com')).args.url)
    assert.match(video, /t=80&p=3/)
    await page.locator('dialog[open]').getByRole('button', { name: '关闭', exact: true }).click()
    assert.equal(await input.inputValue(), '尚未发送的草稿')
    // The image error path keeps the provider badge and clickable URL.
    if (ready) {
      await card(4).locator('img').evaluate(img => img.dispatchEvent(new Event('error')))
      assert.equal(await card(4).locator('img').isVisible(), false)
      assert.equal(await card(4).locator('.social-link-badge').isVisible(), true)
    }
    await page.locator('[data-message-id="fixture-0"]').scrollIntoViewIfNeeded()
    await page.screenshot({ path: path.join(output, ready ? 'windows-covers.png' : 'windows-fallback.png') })
    for (const zoom of [1, 1.5]) {
      await page.setViewportSize({ width: 640, height: 850 }); await page.evaluate(z => { document.documentElement.style.zoom = String(z) }, zoom)
      await card(4).scrollIntoViewIfNeeded(); await card(4).click({ button: 'right' })
      const menu = await page.getByRole('menu').boundingBox()
      assert.ok(menu.x >= 0 && menu.y >= 0 && menu.x + menu.width <= 641 && menu.y + menu.height <= 851)
      await page.screenshot({ path: path.join(output, `windows-menu-${ready}-${zoom}.png`) })
      await page.keyboard.press('Escape')
    }
    await page.evaluate(() => { document.documentElement.style.zoom = '1' })
    await page.getByRole('button', { name: '模拟消息更新' }).click()
    assert.match(await card(0).getAttribute('href'), /bilibili/)
    assert.equal(await input.inputValue(), '尚未发送的草稿')
    await page.setViewportSize({ width: 390, height: 844 })
    await page.goto(base + '?pwa'); await page.locator('.social-link-card').first().waitFor()
    assert.equal(await page.locator('.social-link-card').count(), 6)
    assert.match(await page.locator('.social-link-card').nth(3).innerText(), /01:20/)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    await page.screenshot({ path: path.join(output, ready ? 'pwa-covers.png' : 'pwa-fallback.png'), fullPage: true })
    assert.deepEqual(errors, [])
    results.push({ scenario: ready ? 'metadata-present' : 'metadata-unavailable', passed: true })
    await context.close()
  }
  await writeFile(path.join(output, 'report.json'), JSON.stringify({ renderer: 'production React SocialConversation and shared PWA cards', externalSitesVerified: false, nativeTauriVerified: false, results }, null, 2))
  console.log('PASS: real React/PWA cards, six platforms, original copy/quote, menus, keyboard, zoom, broken cover, update and draft retention')
} finally { await browser.close() }
