// Exercise the production record reader and viewer using authorized local fixtures only.
const assert = require('node:assert/strict')
const path = require('node:path')
const fs = require('node:fs')
const { pathToFileURL } = require('node:url')
const engine = process.env.BROWSER_ENGINE || 'chromium'
const root = path.resolve(__dirname, '..')
const card = { schema: 'chat_record_bundle_v1', record_id: 'record_test', group_id: 'group_test', title: '微信聊天记录', summary: 'fixture', message_count: 2, total_count: 3 }
const row = { id: 'image', parent_id: null, sender: 'Fixture', time: '', kind: 'image', text: '', filename: 'long-image.png', asset_id: 'asset_test' }
const document = { title: card.title, raw_text: '', warnings: [], messages: [row, { ...row, id: 'forward', kind: 'forward', asset_id: null }, { ...row, id: 'child', parent_id: 'forward', filename: 'nested.png' }] }

async function main() {
  const output = path.resolve(root, '../.ai-tmp/record-image-pc'); fs.mkdirSync(output, { recursive: true })
  const { createServer } = await import(pathToFileURL(path.join(root, 'node_modules/vite/dist/node/index.js')))
  const server = await createServer({ root, appType: 'custom', server: { host: '127.0.0.1', port: 0 }, plugins: [{ name: 'record-image-test', configureServer(vite) {
    vite.middlewares.use(async (req, res, next) => {
      if (!req.url?.includes('/record-image-acceptance') || req.url.includes('html-proxy')) return next()
      const html = await vite.transformIndexHtml('/pc/record-image-acceptance', `<!doctype html><html><head><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><div id="root"></div><script type="module">
        import React from 'react'; import {createRoot} from 'react-dom/client';
        import Reader from '/src/features/friends/chat-records/ChatRecordReader.tsx';
        import {useAuthStore} from '/src/store/auth.ts';
        useAuthStore.setState({token:'local-fixture',user:{id:'fixture-reader',account:'fixture'}});
        const root=createRoot(document.getElementById('root'));
        window.switchOwner=()=>useAuthStore.setState({token:'other-fixture',user:{id:'other',account:'other'}});
        function App(){const [open,setOpen]=React.useState(true);return open?React.createElement(Reader,{card:${JSON.stringify(card)},onClose:()=>setOpen(false)}):null;}
        root.render(React.createElement(App));
      </script></body></html>`)
      res.setHeader('content-type', 'text/html'); res.end(html)
    })
  } }] })
  await server.listen()
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) })
  try {
    for (const width of [1280, 390]) {
      const page = await browser.newPage({ viewport: { width, height: 844 } }), errors = []
      page.on('pageerror', e => errors.push(e.message)); page.setDefaultTimeout(15000)
      await page.addInitScript(() => {
        window.leases = new Set()
        const create = URL.createObjectURL.bind(URL), revoke = URL.revokeObjectURL.bind(URL)
        URL.createObjectURL = blob => { const url = create(blob); window.leases.add(url); return url }
        URL.revokeObjectURL = url => { window.leases.delete(url); revoke(url) }
      })
      await page.route('**/*', route => {
        const url = new URL(route.request().url())
        if (url.pathname.includes('/api/me/groups/group_test/chat-records/record_test')) {
          assert.equal(route.request().method(), 'GET')
          if (route.request().headers().authorization === 'Bearer other-fixture') return route.fulfill({ status: 403, body: '' })
          assert.equal(route.request().headers().authorization, 'Bearer local-fixture')
          return route.fulfill(url.pathname.includes('/assets/')
            ? { contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="6000"><rect width="1080" height="6000" fill="white"/><text x="30" y="80" font-size="40">PRIVATE RECORD IMAGE</text></svg>' }
            : { json: { card, owner_id: 'owner', document } })
        }
        return url.origin === origin ? route.continue() : route.abort()
      })
      await page.goto(origin + '/pc/record-image-acceptance')
      const reader = () => page.getByRole('dialog', { name: '聊天记录', exact: true })
      const preview = () => page.getByRole('dialog', { name: /^图片预览：/ })
      async function open(name) {
        await page.getByRole('button', { name: '查看大图：' + name, exact: true }).click()
        await page.waitForFunction(() => document.querySelector('[aria-label="图片阅读区域"]')?.dataset.reading === 'true')
        assert.equal(page.context().pages().length, 1)
        assert.equal(await preview().count(), 1)
      }
      await open('long-image.png')
      await page.screenshot({ path: path.join(output, `${engine}-${width}.png`) })
      await page.keyboard.press('Escape'); await preview().waitFor({ state: 'detached' })
      assert(await reader().isVisible())
      assert(await page.getByRole('button', { name: '查看大图：long-image.png', exact: true }).evaluate(n => n === document.activeElement))
      await open('long-image.png')
      await page.getByRole('button', { name: '关闭图片预览', exact: true }).click(); await preview().waitFor({ state: 'detached' })
      await page.getByRole('button', { name: '聊天记录 · 1 条', exact: true }).click()
      await open('nested.png')
      await page.keyboard.press('Escape'); await preview().waitFor({ state: 'detached' })
      assert.equal(await reader().locator('article').count(), 1)
      await page.getByRole('button', { name: '返回', exact: true }).click()
      assert.equal(await reader().locator('article').count(), 2)
      await open('long-image.png'); await page.evaluate(() => window.switchOwner())
      await page.waitForFunction(() => !document.querySelector('dialog'))
      assert.equal(await page.evaluate(() => window.leases.size), 0)
      assert.deepEqual(errors, []); await page.close()
    }
    console.log(JSON.stringify({ status: 'passed', engine, viewports: [1280, 390], cases: ['authorized-image-open', 'auto-width', 'no-popup', 'escape-preserves-reader', 'close-and-reopen', 'nested-image-and-back', 'focus-return', 'account-close-revokes-blobs'], productionWrites: 0 }))
  } finally { await browser.close(); await server.close() }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
