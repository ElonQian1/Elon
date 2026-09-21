import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import fs from 'node:fs/promises'
import { createServer } from 'vite'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright')
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const out = path.resolve(root, '../.ai-tmp/group-ai-context-ui')
await fs.mkdir(out, { recursive: true })
const server = await createServer({ root, server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
await server.listen()
const port = server.httpServer.address().port
const browser = await chromium.launch({ channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge', headless: true })
try {
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } })
    const errors = []; page.on('pageerror', error => errors.push(error.message))
    let allowed = false, version = 1, patches = 0
    const sources = [{ id: 'one', sender_user_id: 'fixture-owner', sender_name: '甲', content: '**验收记录**\n\n|步骤|状态|\n|---|---|\n|校验|完成|', created_at: '2026-09-21T01:00:00Z' },
      { id: 'two', sender_user_id: 'fixture-peer', sender_name: '乙', content: '```ts\nconst approved = true\n```', created_at: '2026-09-21T01:01:00Z' }]
    await page.route('**/*', async route => {
      const url = new URL(route.request().url())
      if (url.pathname.endsWith('/ai-sources')) {
        if (route.request().method() === 'PATCH') {
          const body = route.request().postDataJSON(); assert.equal(body.version, version)
          allowed = body.allow_continue; version++; patches++
        }
        return route.fulfill({ json: { schema: 1, group_id: 'fixture-group', message_id: 'gai_fixture', requester_id: 'fixture-owner', provider: 'chatgpt_web', allow_continue: allowed, version, sources } })
      }
      if (url.pathname.endsWith('/ai-context')) return route.fulfill({ json: { group_id: 'fixture-group', document: { title: '精选讨论', messages: [{ role: 'user', content: 'Selected' }, { role: 'assistant', content: '**Answer**' }] } } })
      if (url.pathname.startsWith('/api/')) return route.fulfill({ json: {} })
      if (url.hostname !== '127.0.0.1') return route.abort()
      return route.continue()
    })
    await page.goto(`http://127.0.0.1:${port}/pc/scripts/fixtures/group-ai-reply-context.html`)
    await page.getByRole('button', { name: '使用 ChatGPT 继续讨论', exact: true }).waitFor()
    await page.screenshot({ path: path.join(out, `bubble-${width}.png`) })
    await page.getByRole('button', { name: /群聊的聊天记录/ }).click()
    await page.getByText('验收记录', { exact: true }).waitFor()
    assert.equal(await page.locator('dialog table').count(), 1)
    assert.match(await page.locator('dialog code').innerText(), /const approved = true/)
    await page.getByRole('checkbox').check()
    await page.waitForFunction(() => document.querySelector('dialog input[type=checkbox]')?.disabled === false)
    assert.equal(patches, 1)
    await page.screenshot({ path: path.join(out, `reader-${width}.png`) })
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth), false)
    await page.getByRole('button', { name: '返回群聊', exact: true }).click()
    await page.getByRole('button', { name: '使用 ChatGPT 继续讨论', exact: true }).click()
    await page.getByRole('button', { name: '打开我的 ChatGPT' }).waitFor()
    await page.getByRole('button', { name: '打开我的 ChatGPT' }).click()
    assert.equal(await page.getByRole('textbox', { name: '私人聊天输入框' }).inputValue(), '')
    await page.getByRole('button', { name: '开始私人讨论' }).click()
    await page.waitForFunction(() => document.querySelector('textarea')?.value.includes('Selected'))
    assert.match(await page.getByRole('textbox', { name: '私人聊天输入框' }).inputValue(), /\*\*Answer\*\*/)
    await page.getByRole('button', { name: '返回群聊', exact: true }).click()
    assert.equal(await page.locator('[data-route]').textContent(), '/friends?group=fixture-group')
    await page.goto(`http://127.0.0.1:${port}/pc/scripts/fixtures/group-ai-reply-context.html?draft=1`)
    await page.getByRole('button', { name: '使用 ChatGPT 继续讨论', exact: true }).click()
    await page.getByRole('button', { name: '打开我的 ChatGPT' }).click()
    await page.getByRole('button', { name: '开始私人讨论' }).click()
    await page.getByText('当前私人会话还有草稿或正在回复，请先处理；不会覆盖或发送。').waitFor()
    assert.equal(await page.getByRole('textbox', { name: '私人聊天输入框' }).inputValue(), 'Existing private draft')
    assert.equal(errors.length, 0, errors.join('\n'))
    await page.close()
  }
  console.log('GROUP_AI_CONTEXT_UI=passed; desktop/mobile source reader, consent and continuation entry verified with fixture transport')
} finally { await browser.close(); await server.close() }
