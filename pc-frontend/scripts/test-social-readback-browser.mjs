import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const base = process.env.CARD_FIXTURE_URL || 'http://127.0.0.1:5196/pc/tests/fixtures/social-cards.html'
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) })
try {
  const page = await browser.newPage()
  const errors = []; page.on('pageerror', e => errors.push(e.message))
  await page.addInitScript(() => {
    window.__readCalls = []
    window.__TAURI__ = { core: { invoke: async (command, args) => {
      window.__readCalls.push({ command, args })
      if (command === 'get_internal_browser_tab_state' && args?.originalUrl) {
        const original = args.originalUrl
        const isX = original.includes('x.com')
        return { loaded: true, visible: true, readPreview: { schema: 1, original, url: isX ? original : 'https://www.binance.com/en/square/post/123456', article: true, title: isX ? 'Read X post' : 'Read Binance article', author: 'Public author', image: null } }
      }
      return {}
    } } }
  })
  await page.route('**/*', async route => {
    const u = new URL(route.request().url())
    if (u.pathname === '/api/me/link-preview') {
      const { url } = route.request().postDataJSON()
      return route.fulfill({ json: { schema: 1, url, title: '', author: '', image: null, status: 'unavailable' } })
    }
    if (u.origin === new URL(base).origin && !u.pathname.startsWith('/api/')) return route.continue()
    return route.abort()
  })
  await page.goto(base)
  const binance = page.locator('[data-message-id="fixture-4"] .social-link-card')
  await binance.click(); await page.locator('dialog[open]').waitFor()
  await binance.getByText('Read Binance article', { exact: true }).waitFor()
  await page.locator('dialog[open]').getByRole('button', { name: '关闭', exact: true }).click()
  await page.locator('dialog[open]').waitFor({ state: 'hidden' })
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('elon-social-read-preview-v1')).length), 1)
  await page.reload(); await binance.getByText('Read Binance article', { exact: true }).waitFor()
  const x = page.locator('[data-message-id="fixture-5"] .social-link-card')
  await x.click(); await x.getByText('Read X post', { exact: true }).waitFor()
  assert.ok(await page.getByRole('button', { name: '嵌入查看', exact: true }).count())
  assert.equal(await page.evaluate(() => window.__readCalls.filter(c => c.command === 'open_internal_browser_tab').at(-1).args.url), await x.getAttribute('href'))
  await page.getByRole('button', { name: '关闭', exact: true }).click(); await page.locator('dialog[open]').waitFor({ state: 'hidden' })
  const checks = await page.evaluate(async () => {
    const { cachedRead, rememberRead } = await import('/pc/src/features/friends/socialReadPreview.ts')
    const original = ElonSocialLinks.links('https://x.com/example/status/123456')[0]
    const value = { schema: 1, original: original.url, url: original.url, article: true, title: 'A post', author: '', image: null }
    rememberRead('scope-a', original, value)
    const isolated = cachedRead('scope-b', original) === null
    const wrongPost = rememberRead('scope-a', original, { ...value, url: 'https://x.com/example/status/999999' }) === null
    let rows = JSON.parse(localStorage.getItem('elon-social-read-preview-v1'))
    rows = rows.map(row => ({ ...row, saved: Date.now() - 24 * 3600000 - 1 }))
    localStorage.setItem('elon-social-read-preview-v1', JSON.stringify(rows))
    const expired = cachedRead('scope-a', original) === null
    for (let i = 100000; i < 100140; i++) {
      const item = ElonSocialLinks.links('https://x.com/example/status/' + i)[0]
      rememberRead('scope-a', item, { ...value, original: item.url, url: item.url })
    }
    return { isolated, wrongPost, expired, capacity: JSON.parse(localStorage.getItem('elon-social-read-preview-v1')).length }
  })
  assert.deepEqual(checks, { isolated: true, wrongPost: true, expired: true, capacity: 128 })
  assert.deepEqual(errors, [])
  console.log(JSON.stringify({ passed: true, checks, reloadRestoresPreview: true, xOriginalRoute: true, nativeTransport: 'test double', realSiteVerified: false }))
} finally { await browser.close() }
