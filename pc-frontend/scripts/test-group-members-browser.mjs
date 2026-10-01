import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const base = process.env.GROUP_MEMBERS_FIXTURE_URL || 'http://127.0.0.1:5196/pc/tests/fixtures/group-members.html'
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) })
try {
  const page = await browser.newPage()
  const errors = []; page.on('pageerror', error => errors.push(error.message))
  let count = 16; let denied = false; let malformed = false; let requests = 0
  await page.route('**/api/me/groups/fixture-group/members', route => {
    requests++
    return route.fulfill({ status: denied ? 403 : 200, json: denied ? { error: 'not a member' } : malformed ? {} : {
      members: Array.from({ length: count }, (_, index) => ({ id: `member-${index + 1}`, display_name: `成员${index + 1}` })),
      ai_members: [{ id: 'virtual-ai', display_name: 'EL' }],
    } })
  })
  await page.route('**/api/users/*/avatar', route => route.fulfill({ status: 404, body: '' }))
  await page.goto(base)
  const open = () => page.getByRole('button', { name: /查看全部群成员/ }).click()
  const close = () => page.getByRole('button', { name: '关闭群成员', exact: true }).click()
  await open()
  const roster = page.getByRole('list', { name: '完整群成员名单' })
  await page.getByRole('status').filter({ hasText: '共 16 位成员' }).waitFor()
  assert.equal(await roster.getByRole('listitem').count(), 16)
  assert.equal(await roster.locator('[data-member-id="member-1"]').count(), 1)
  assert.equal(await roster.locator('[data-member-id="member-16"]').count(), 1)
  assert.equal(await roster.locator('[data-member-id="virtual-ai"]').count(), 0)
  if (process.env.GROUP_MEMBERS_SCREENSHOT) await page.screenshot({ path: process.env.GROUP_MEMBERS_SCREENSHOT })
  const search = page.getByRole('searchbox', { name: '搜索群成员' })
  await search.fill('成员16')
  assert.equal(await roster.getByRole('listitem').count(), 1)
  await search.fill('不存在'); await page.getByText('未找到匹配的群成员').waitFor()
  await close(); count = 17; await open()
  await page.getByRole('status').filter({ hasText: '共 17 位成员' }).waitFor()
  assert.equal(await roster.getByRole('listitem').count(), 17)
  await close(); denied = true; await open()
  await page.getByRole('alert').waitFor()
  assert.equal(await roster.count(), 0)
  denied = false; await page.getByRole('button', { name: '重新加载' }).click()
  await page.getByRole('status').filter({ hasText: '共 17 位成员' }).waitFor()
  await close(); malformed = true; await open(); await page.getByRole('alert').waitFor()
  malformed = false; count = 0; await page.getByRole('button', { name: '重新加载' }).click()
  await page.getByText('暂无群成员').waitFor()
  await page.keyboard.press('Escape')
  assert.equal(await page.getByRole('dialog').count(), 0)
  assert.equal(requests, 6)
  assert.deepEqual(errors, [])
  console.log(JSON.stringify({ passed: true, fullRoster: 17, search: true, retry: true, refresh: true, empty: true, malformed: true }))
} finally { await browser.close() }
