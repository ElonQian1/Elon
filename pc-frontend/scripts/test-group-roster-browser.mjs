import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { mkdir } from 'node:fs/promises'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const base = process.env.GROUP_ROSTER_URL || 'http://127.0.0.1:5187/pc/tests/fixtures/'
const output = process.env.GROUP_ROSTER_OUTPUT || '../.ai-tmp/group-roster-browser'
await mkdir(output, { recursive: true })
const browser = await chromium.launch({ headless: true, ...(process.env.CARD_BROWSER ? { executablePath: process.env.CARD_BROWSER } : {}) })
const errors = []
let count = 172, revision = 1, denied = false, role = 'owner', commands = []
async function mock(page) {
  await page.addInitScript(() => Object.defineProperty(crypto, 'randomUUID', { value: undefined }))
  page.on('pageerror', error => errors.push(error.message))
  await page.route('**/api/**', async route => {
    const request = route.request(), url = new URL(request.url())
    if (!url.pathname.startsWith('/api/')) return route.continue()
    if (url.pathname.endsWith('/roster')) {
      if (denied) return route.fulfill({ status: 403, json: { error: '你已不在这个群聊中' } })
      const other = url.pathname.includes('/other-group/'), total = other ? 2 : count
      const people = Array.from({ length: total }, (_, index) => ({ id: `member-${String(index).padStart(4, '0')}`, display_name: `${other ? '另一群' : '成员'}${String(index).padStart(4, '0')}`, role: index === 0 ? 'owner' : index === 1 ? 'admin' : 'member', joined_at: '2026-10-01T10:00:00Z' }))
      let rows = people.filter(person => person.display_name.includes(url.searchParams.get('q') || ''))
      if (url.searchParams.get('filter') === 'admins') rows = rows.filter(person => person.role !== 'member')
      const offset = Number(url.searchParams.get('cursor') || 0), limit = 50
      return route.fulfill({ json: { group_id: other ? 'other-group' : 'fixture-group', name: other ? '另一群' : '杀蟑螂', total_count: total, matched_count: rows.length, revision,
        viewer_id: role === 'owner' ? 'member-0000' : 'member-0002', viewer_role: role, invitation_policy: 'members', permissions: { invite: true, manage: role !== 'member', owner: role === 'owner' }, pending_count: 1,
        members: rows.slice(offset, offset + limit), next_cursor: offset + limit < rows.length ? String(offset + limit) : null } })
    }
    if (url.pathname.endsWith('/membership')) { commands.push(request.postDataJSON()); revision++; return route.fulfill({ json: { ok: true, changed: 1, message: '操作已完成', exited: false } }) }
    if (url.pathname.endsWith('/invitations')) return route.fulfill({ json: { requests: [{ id: 'invite-one', actor_name: '申请者', members: [{ display_name: '待入群好友' }] }], total_count: 1, next_offset: null } })
    if (url.pathname === '/api/me/friends') return route.fulfill({ json: { friends: [{ id: 'friend-1', nickname: '待邀请好友', account: '待邀请好友' }] } })
    return route.fulfill({ status: 404, json: {} })
  })
}
try {
  const desktop = await browser.newPage({ viewport: { width: 1440, height: 960 } }); await mock(desktop)
  await desktop.goto(`${base}group-roster.html`)
  const members = desktop.getByRole('list', { name: '群成员名单' })
  await members.getByRole('listitem').nth(49).waitFor({ state: 'attached' }); assert.equal(await members.getByRole('listitem').count(), 50)
  for (let i = 0; i < 3; i++) await desktop.getByRole('button', { name: /加载更多/ }).click()
  await members.getByRole('listitem').nth(171).waitFor({ state: 'attached' }); assert.equal(await members.getByRole('listitem').count(), 172)
  const search = desktop.getByRole('searchbox', { name: '搜索群成员' })
  await search.fill('成员0171'); await desktop.getByText('找到 1 人 · 全群 172 人').waitFor()
  assert.equal(await members.getByRole('listitem').count(), 1)
  await search.fill('不存在'); await desktop.getByText('未找到匹配的群成员').waitFor()
  await search.fill(''); await members.getByRole('listitem').nth(49).waitFor({ state: 'attached' })
  await desktop.getByRole('button', { name: '管理成员', exact: true }).click()
  await desktop.getByRole('checkbox', { name: '选择成员0002', exact: true }).check()
  await desktop.getByRole('button', { name: '移出所选成员（1）' }).click()
  await desktop.getByRole('dialog', { name: '移出所选成员' }).getByRole('button', { name: '移出所选成员', exact: true }).click()
  await desktop.getByText('操作已完成', { exact: true }).waitFor(); assert.equal(commands[0].action, 'remove'); assert.deepEqual(commands[0].user_ids, ['member-0002']); assert.ok(commands[0].request_id)
  await desktop.getByRole('button', { name: '成员0001 管理员' }).click()
  await desktop.getByRole('button', { name: '取消管理员', exact: true }).waitFor()
  await desktop.getByRole('button', { name: '关闭群成员资料', exact: true }).click()
  await desktop.screenshot({ path: `${output}/desktop.png` })
  await desktop.getByRole('button', { name: '切换测试群' }).click(); await desktop.getByText('另一群0001').waitFor(); assert.equal(await members.getByRole('listitem').count(), 2)
  denied = true; await desktop.getByRole('button', { name: '刷新', exact: true }).click(); await desktop.getByRole('alert').waitFor(); assert.equal(await members.count(), 0)
  denied = false; await desktop.getByRole('button', { name: '重新加载' }).click(); await desktop.getByText('另一群0001').waitFor()
  await desktop.close()

  const mobile = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true }); await mock(mobile); role = 'member'
  await mobile.goto(`${base}group-roster.html`); await mobile.getByRole('dialog', { name: '群成员', exact: true }).waitFor()
  assert.equal(await mobile.getByRole('button', { name: '管理成员', exact: true }).count(), 0)
  assert.equal(await mobile.getByRole('button', { name: '解散群聊', exact: true }).count(), 0)
  await mobile.getByRole('button', { name: '关闭群成员', exact: true }).click(); await mobile.getByRole('button', { name: '查看群成员', exact: true }).click()
  await mobile.getByRole('list', { name: '群成员名单' }).waitFor(); await mobile.screenshot({ path: `${output}/desktop-narrow.png` }); await mobile.close()

  const pwa = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true }); await mock(pwa); role = 'owner'
  await pwa.goto(`${base}group-roster-pwa.html`); await pwa.getByRole('button', { name: '查看群成员', exact: true }).click()
  await pwa.getByRole('button', { name: '查看全部成员（172）' }).waitFor(); assert.equal(await pwa.locator('.gr-tile').count(), 15)
  await pwa.screenshot({ path: `${output}/pwa-preview.png` })
  await pwa.getByRole('button', { name: '查看全部成员（172）' }).click()
  await pwa.getByRole('searchbox', { name: '搜索群成员' }).fill('成员0171')
  await pwa.getByText('找到 1 人 · 全群 172 人').waitFor(); assert.equal(await pwa.locator('.gr-person').count(), 1)
  await pwa.locator('.gr-person').click(); await pwa.getByRole('button', { name: '在群里 @TA' }).click(); await pwa.getByText('已插入 @成员0171').waitFor()
  await pwa.getByRole('button', { name: '查看群成员', exact: true }).click(); await pwa.getByRole('button', { name: '查看全部成员（172）' }).waitFor()
  await pwa.getByRole('button', { name: '查看全部成员（172）' }).click(); await pwa.screenshot({ path: `${output}/pwa-directory.png` })
  const overflow = await pwa.evaluate(() => document.querySelector('.gr-host').scrollWidth > innerWidth + 1); assert.equal(overflow, false)
  await pwa.getByRole('button', { name: '邀请成员', exact: true }).click(); await pwa.getByText('待邀请好友', { exact: true }).waitFor()
  await pwa.getByRole('checkbox').check(); await pwa.getByRole('button', { name: '邀请（1）', exact: true }).click(); await pwa.getByRole('dialog').last().getByRole('button', { name: '邀请成员', exact: true }).click()
  await pwa.getByText('操作已完成', { exact: true }).waitFor(); assert.equal(commands.at(-1).action, 'invite')
  await pwa.close()
  const widePwa = await browser.newPage({ viewport: { width: 1440, height: 960 } }); await mock(widePwa)
  await widePwa.goto(`${base}group-roster-pwa.html`); await widePwa.locator('.gr-person').first().waitFor()
  const bounds = await widePwa.evaluate(() => ({ member: document.querySelector('.gr-host').getBoundingClientRect().toJSON(), input: document.querySelector('.input-bar').getBoundingClientRect().toJSON(), chat: document.querySelector('#chatPage').getBoundingClientRect().toJSON() }))
  assert.ok(bounds.chat.height > 800); assert.ok(bounds.member.left >= bounds.input.right - 1)
  await widePwa.screenshot({ path: `${output}/pwa-wide.png` }); await widePwa.close(); assert.deepEqual(errors, [])
  console.log(JSON.stringify({ passed: true, members: 172, pagination: true, serverSearch: true, groupIsolation: true, permissions: true, mutation: true, retry: true, pwaPreview: true, pwaMention: true, screenshots: output }))
} finally { await browser.close() }
