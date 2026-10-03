const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { createRequire } = require('node:module')
const { pathToFileURL } = require('node:url')
const workspace = path.resolve(__dirname, '../..')
const root = path.join(workspace, 'pc-frontend')
const artifacts = path.join(workspace, '.ai-tmp/consumer-entry-ui')
const pcRequire = createRequire(path.join(root, 'package.json'))
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')

async function main() {
  await fs.mkdir(artifacts, { recursive: true })
  const { createServer } = await import(pathToFileURL(pcRequire.resolve('vite')).href)
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.ts'),
    server: { host: '127.0.0.1', port: 5187 }, logLevel: 'error' })
  let browser
  const results = []
  try {
    await server.listen()
    const base = `http://127.0.0.1:${server.httpServer.address().port}`
    browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
    for (const scenario of [
      { name: 'local-visitor', local: true, authenticated: false },
      { name: 'cloud-visitor', local: false, authenticated: false },
      { name: 'local-signed-in', local: true, authenticated: true },
      { name: 'local-offline-start', local: true, authenticated: false, offline: true },
      { name: 'local-signed-in-offline', local: true, authenticated: true, offline: true },
    ]) {
      const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' })
      const page = await context.newPage()
      page.setDefaultTimeout(25000)
      const errors = [], requests = [], diagnostics = []
      let cloudOffline = Boolean(scenario.offline)
      page.on('pageerror', error => errors.push(error.message))
      page.on('requestfailed', req => diagnostics.push({ url: req.url(), failure: req.failure()?.errorText }))
      page.on('console', message => { if (message.type() === 'error') diagnostics.push({ console: message.text() }) })
      await page.addInitScript(({ local, authenticated, base }) => {
        if (local) window.__ELON_PC_BOOTSTRAP__ = { mode: 'local', cloudBaseUrl: 'https://cloud.example', localNodeBaseUrl: base }
        if (authenticated) localStorage.setItem('elon_auth', JSON.stringify({ state: {
          token: 'isolated-browser-fixture-token', expiresAt: '2099-01-01T00:00:00Z',
          user: { id: 'fixture-owner', account: '验收账号', nickname: '工作台验收', role: 'user' },
        }, version: 0 }))
        localStorage.setItem('elon_project_prewarm_enabled', 'false')
      }, { ...scenario, base })
      await page.routeWebSocket('**/ws/app**', socket => socket.close())
      await page.route('**/*', async route => {
        const req = route.request(), url = new URL(req.url()), pathname = url.pathname
        if (req.method() === 'OPTIONS') return route.fulfill({ status: 204,
          headers: { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Methods': '*', 'Access-Control-Allow-Headers': '*' } })
        if (pathname.startsWith('/api/') || pathname === '/health') {
          requests.push({ origin: url.origin, path: pathname, method: req.method() })
          const localEndpoint = ['/api/status', '/api/health', '/api/local-tasks', '/api/self-evolution', '/api/full-access/grants'].includes(pathname)
          if (cloudOffline && !localEndpoint) return route.abort('internetdisconnected')
          let json = {}
          if (pathname === '/api/status') json = { service: 'elon-node-agent', logged_in: scenario.authenticated,
            owner_user_id: scenario.authenticated ? 'fixture-owner' : '', connected: !cloudOffline,
            agent_id: 'node-fixture', version: '0.0.0-test', local_admin_token: 'fixture-admin', local_admin_token_header: 'x-elon-local-admin-token' }
          else if (pathname === '/api/runtime' || pathname === '/health') json = { service: 'elon-server', status: 'ok' }
          else if (pathname === '/api/me') json = { user: { id: 'fixture-owner', account: '验收账号', nickname: '工作台验收', role: 'user' } }
          else if (pathname === '/api/auth/trust-current-device') json = { expires_at: '2099-01-01T00:00:00Z' }
          else if (pathname === '/api/me/friends/recommendations') json = { recommendations: [
            { id: 'fixture-peer-a', account: '验收用户甲', nickname: '验收用户甲', is_online: true },
            { id: 'fixture-peer-b', account: '验收用户乙', nickname: '验收用户乙', is_online: false },
          ], total_count: 2 }
          else if (pathname === '/api/me/nodes') json = { nodes: [] }
          else if (pathname === '/api/me/presence') json = { user_id: 'fixture-owner', status: 'online' }
          else if (pathname === '/api/projects') json = { projects: [] }
          else if (pathname === '/api/local-tasks') json = { tasks: [], pending_sync_count: 0 }
          else if (pathname === '/api/self-evolution') json = { items: [] }
          else if (pathname === '/api/full-access/grants') json = { grants: [] }
          else if (pathname.includes('conversations')) json = { conversations: [], messages: [] }
          else if (pathname.includes('models')) json = { options: [], models: [] }
          return route.fulfill({ json, headers: { 'Access-Control-Allow-Origin': '*' } })
        }
        if (url.origin === base) return route.continue()
        return route.abort('blockedbyclient')
      })
      await page.goto(`${base}/pc/`)
      try { await page.waitForURL('**/pc/ai') } catch (error) {
        await page.screenshot({ path: path.join(artifacts, `${scenario.name}-failure.png`) })
        console.error(JSON.stringify({ url: page.url(), errors, body: (await page.locator('body').innerText()).slice(0, 2500) }))
        throw error
      }
      await page.locator('[data-ai-surface="production-home"]').waitFor()
      await page.getByRole('button', { name: '收起右侧用户栏', exact: true }).waitFor()
      assert.equal(await page.getByRole('navigation', { name: '全局工作区导航' }).isVisible(), true)
      assert.equal(await page.getByText('全站用户', { exact: true }).count() > 0 || scenario.authenticated, true)
      assert.equal(await page.getByRole('button', { name: 'AI 工作区', exact: true }).isVisible(), true)
      assert.equal(await page.getByRole('heading', { name: '本机任务', exact: true }).count(), 0)
      assert.equal(await page.getByRole('button', { name: '管理', exact: true }).count(), 0)
      if (scenario.authenticated && !scenario.offline) {
        await page.getByText('验收用户甲', { exact: true }).waitFor()
        await page.getByText('验收用户乙', { exact: true }).waitFor()
        assert.ok(requests.some(request => request.path === '/api/me' && request.origin === 'https://cloud.example'))
        assert.ok(requests.some(request => request.path === '/api/me/friends/recommendations' && request.origin === 'https://cloud.example'))
      }
      if (scenario.offline) await page.getByRole('button', { name: '重试连接', exact: true }).waitFor()
      else {
        await page.waitForFunction(() => document.body.innerText.includes('正在检查云端连接') === false)
        try { await page.getByRole('button', { name: '重试连接', exact: true }).waitFor({ state: 'hidden', timeout: 3000 }) }
        catch (error) { console.error(JSON.stringify({ scenario: scenario.name, requests, diagnostics })); throw error }
      }
      await page.screenshot({ path: path.join(artifacts, `${scenario.name}.png`) })
      if (scenario.name === 'local-signed-in-offline') {
        assert.equal(await page.getByText('验收用户甲', { exact: true }).count(), 0, 'offline cold start does not invent live directory data')
        cloudOffline = false
        await page.getByRole('button', { name: '重试连接', exact: true }).click()
        await page.getByText('验收用户甲', { exact: true }).waitFor()
        await page.getByText('验收用户乙', { exact: true }).waitFor()
        assert.equal(new URL(page.url()).pathname, '/pc/ai')
        await page.screenshot({ path: path.join(artifacts, 'local-signed-in-recovered.png') })
      }
      if (scenario.name === 'local-signed-in') {
        const composer = page.getByPlaceholder('输入消息，Enter 发送，Shift+Enter 换行')
        await composer.fill('离线恢复保留的验收草稿')
        cloudOffline = true
        await page.evaluate(() => window.dispatchEvent(new Event('offline')))
        await page.getByRole('button', { name: '重试连接', exact: true }).waitFor()
        assert.equal(new URL(page.url()).pathname, '/pc/ai')
        assert.equal(await composer.inputValue(), '离线恢复保留的验收草稿')
        await page.screenshot({ path: path.join(artifacts, 'local-offline-draft.png') })
        cloudOffline = false
        await page.getByRole('button', { name: '重试连接', exact: true }).click()
        await page.getByRole('button', { name: '重试连接', exact: true }).waitFor({ state: 'hidden' })
        assert.equal(await composer.inputValue(), '离线恢复保留的验收草稿')
        assert.equal(new URL(page.url()).pathname, '/pc/ai')
        await page.getByRole('button', { name: '收起右侧用户栏', exact: true }).click()
        await page.reload()
        await page.locator('[data-user-panel-collapsed="true"]').waitFor()
        await page.getByRole('button', { name: '展开右侧用户栏', exact: true }).first().click()
        await page.getByText('验收用户甲', { exact: true }).waitFor()
        await page.goto(`${base}/pc/local-tasks`)
        await page.getByRole('heading', { name: '本机任务', exact: true }).waitFor()
        assert.equal(new URL(page.url()).pathname, '/pc/local-tasks')
        assert.ok(requests.some(request => request.path === '/api/local-tasks' && request.origin === base))
      }
      assert.deepEqual(errors, [], `${scenario.name} has no uncaught browser errors`)
      results.push({ scenario: scenario.name, result: 'passed', uncaughtErrors: errors.length })
      await context.close()
      console.log(`PASS browser fixture ${scenario.name}`)
    }
    await fs.writeFile(path.join(artifacts, 'results.json'), JSON.stringify({ coverage: 'Real Edge browser and product React app with isolated API fixtures; no Tauri or real cloud authentication', results }, null, 2))
  } finally {
    await browser?.close()
    await server.close()
  }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
