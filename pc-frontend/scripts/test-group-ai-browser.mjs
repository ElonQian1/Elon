import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { mkdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createServer } from 'vite'
const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const artifacts = path.resolve(root, '../.ai-tmp/group-ai')
const server = await createServer({ root, configFile: path.join(root, 'vite.config.ts'), server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
let browser
try {
  await server.listen()
  browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
  const page = await browser.newPage({ viewport: { width: 1280, height: 820 } })
  page.setDefaultTimeout(12000)
  const errors = [], requests = []
  page.on('pageerror', error => errors.push(error.message))
  await page.addInitScript(() => {
    window.__groupFixture = { commands: [], sent: false, ready: false, receipt: null }
    window.__TAURI__ = { core: { invoke: async (command, args) => {
      const f = window.__groupFixture
      if (command === 'list_local_ai_web_providers') return ['chatgpt', 'google-ai-mode'].map(id => ({
        id, displayName: id, desktopRuntimeVersion: 13, adapterVersion: 500,
        loginMode: 'manual_web', profileScope: 'local_owner_provider', rendererProtocol: 'yilong.ai.ui.v1',
        researchCaptureStatus: 'local_raw_prelaunch', researchCaptureRetentionDays: 30,
      }))
      if (command === 'group_ai_web_session') {
        f.commands.push(args)
        if (args.action === 'prepare') f.receipt = { action: 'private_protocol_probe', requestId: args.requestId, ok: true, detail: '{"code":"ready","stage":"ready"}' }
        if (args.action === 'send_prompt') f.sent = true
        const messages = f.sent && f.ready ? [
          { role: 'user', state: 'completed', content: [{ type: 'text', text: 'fixture selected prompt' }] },
          { role: 'assistant', state: 'completed', content: [{ type: 'markdown', text: '**AI answer fixture**\n\nComplete reply.' }] },
        ] : []
        return { loading: false, contextReady: true, semanticCacheStatus: 'live', currentUrl: 'https://chatgpt.com/?temporary-chat=true',
          commandResult: f.receipt, semanticEvent: { type: 'message_snapshot', url: 'https://chatgpt.com/', draft: '', composerReady: true,
            loginRequired: false, streaming: f.sent, privateStreamState: f.ready ? 'completed' : 'streaming', messages } }
      }
      return {}
    } } }
  })
  const message = (id, content, revision = 1) => ({ id, content, revision, sender_user_id: 'peer', sender_name: '测试成员', created_at: '2026-09-21T00:00:00Z' })
  const messages = [message('one', 'First selected fixture'), message('two', 'Second selected fixture', 3),
    { ...message('link-answer', 'https://www.bilibili.com/video/BV1xx411c7mD'), ai_reply: {
      schema: 1, provider: 'chatgpt_web', requester_id: 'peer', source_count: 1, allow_continue: true, version: 1,
      previews: [{ sender_name: '测试成员', text: 'Selected source' }],
    } }]
  let prepared
  await page.route('**/*', async route => {
    const req = route.request(), url = new URL(req.url()), p = url.pathname
    if (!p.startsWith('/api/')) return url.hostname === '127.0.0.1' ? route.continue() : route.abort()
    if (p === '/api/me/groups/g/messages/one/web-ai') {
      const body = req.postDataJSON(); requests.push({ p, body })
      prepared = { id: 'request', group_id: 'g', trigger_message_id: 'one', prompt: 'fixture selected prompt', state: 'prepared', engine: 'chatgpt_web', web_provider: 'chatgpt_web' }
      return route.fulfill({ json: { request: prepared } })
    }
    if (p === '/api/me/groups/g/web-ai/requests/request') {
      const body = req.postDataJSON(); requests.push({ p, body })
      if (body.action === 'dispatch') prepared = { ...prepared, state: 'dispatched', dispatch_permit: true }
      if (body.action === 'complete') {
        prepared = { ...prepared, state: 'completed', result_message_id: 'answer' }
        messages.push(message('answer', body.content))
      }
      return route.fulfill({ json: { request: prepared } })
    }
    if (p === '/api/me/groups') return route.fulfill({ json: { groups: [{ id: 'g', name: '群聊验收', member_count: 2 }, { id: 'g2', name: '另一个群', member_count: 2 }] } })
    if (p === '/api/me/friends') return route.fulfill({ json: { friends: [{ id: 'friend', account: '测试好友' }] } })
    if (p.endsWith('/messages')) return route.fulfill({ json: { messages: p.includes('/g/') ? messages : [message('other', 'Other conversation')] } })
    if (p.endsWith('/members')) return route.fulfill({ json: { members: [{ id: 'peer', display_name: '测试成员' }] } })
    return route.fulfill({ json: {} })
  })
  const choose = title => page.getByRole('button').filter({ has: page.locator('strong', { hasText: title }) }).first().click()
  const row = id => page.locator(`[data-message-id="${id}"]`)
  const menu = async id => { await row(id).getByRole('button', { name: '更多消息操作' }).click(); await page.getByRole('menu').waitFor() }
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/pc/tests/fixtures/group-ai.html`)
  await choose('群聊验收')
  assert.equal(await row('link-answer').getByRole('button', { name: '使用 ChatGPT 继续讨论', exact: true }).isVisible(), true)
  await menu('one')
  await page.getByRole('menuitem', { name: 'AI 回复…', exact: true }).click()
  await page.getByRole('dialog', { name: 'AI 回复到群聊' }).waitFor()
  await page.getByText('已选择 1 条消息', { exact: true }).waitFor()
  await page.getByRole('button', { name: '取消', exact: true }).click()
  await menu('one'); await page.getByRole('menuitem', { name: '多选', exact: true }).click()
  await row('two').getByRole('checkbox').check()
  await page.getByRole('button', { name: 'AI 分析', exact: true }).click()
  await page.getByText('已选择 2 条消息', { exact: true }).waitFor()
  const dialog = page.getByRole('dialog', { name: 'AI 回复到群聊' })
  await dialog.getByRole('textbox').fill('Summarize the selection')
  await mkdir(artifacts, { recursive: true })
  for (const width of [1280, 760]) {
    await page.setViewportSize({ width, height: 820 })
    const bounds = await dialog.boundingBox()
    assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width && bounds.y >= 0 && bounds.y + bounds.height <= 820)
    await page.screenshot({ path: path.join(artifacts, `group-ai-${width}.png`) })
  }
  await page.setViewportSize({ width: 1280, height: 820 })
  await dialog.getByRole('button', { name: '分析并发送到群聊', exact: true }).click()
  await dialog.waitFor({ state: 'hidden' })
  await page.getByText('AI 正在分析所选消息', { exact: true }).waitFor()
  await choose('另一个群')
  await page.getByLabel('群聊 AI 进度').getByText('群聊验收', { exact: true }).waitFor()
  await page.evaluate(() => { window.__groupFixture.ready = true })
  await page.getByText('AI 回答已发送到群聊', { exact: true }).waitFor()
  assert.equal(await row('answer').count(), 0)
  await choose('群聊验收')
  await row('answer').getByText('AI answer fixture', { exact: true }).waitFor()
  assert.deepEqual(requests[0].body.selected_context, { message_ids: ['one', 'two'], message_revisions: { one: 1, two: 3 }, question: 'Summarize the selection', allow_continue: false })
  const sends = await page.evaluate(() => window.__groupFixture.commands.filter(c => c.action === 'send_prompt'))
  assert.equal(sends.length, 1)
  assert.equal(requests.filter(r => r.body.action === 'complete').length, 1)
  assert.equal(requests.at(-1).body.content, '**AI answer fixture**\n\nComplete reply.')
  await page.screenshot({ path: path.join(artifacts, 'group-ai-delivered.png') })
  await choose('测试好友'); await menu('other')
  assert.equal(await page.getByRole('menuitem', { name: 'AI 回复…', exact: true }).count(), 0)
  assert.deepEqual(errors, [])
  console.log(JSON.stringify({ passed: true, singleMessage: true, multiSelect: true, revisionBound: true, returnToOriginalGroup: true,
    markdownPreserved: true, duplicateSends: 0, transport: 'mocked', realProviderVerified: false }))
} finally { await browser?.close(); await server.close() }
