// Production React viewer, synthetic images, local-only Vite server. No account or writes.
const assert = require('node:assert/strict')
const path = require('node:path')
const fs = require('node:fs')
const { pathToFileURL } = require('node:url')
const engine = process.env.BROWSER_ENGINE || 'chromium'
const root = path.resolve(__dirname, '..')
const output = path.resolve(root, '../.ai-tmp/long-image-pc')
const svg = height => `<svg xmlns="http://www.w3.org/2000/svg" width="1080" height="${height}"><rect width="1080" height="${height}" fill="#fff"/><text x="50" y="80" font-size="40" fill="#111">LONG IMAGE START</text><path d="M20 120L1060 ${height - 150}" stroke="#28734f" stroke-width="12"/><text x="50" y="${height - 50}" font-size="40" fill="#111">END OF IMAGE</text></svg>`

async function main() {
  fs.mkdirSync(output, { recursive: true })
  const { createServer } = await import(pathToFileURL(path.join(root, 'node_modules/vite/dist/node/index.js')))
  const server = await createServer({ root, appType: 'custom', server: { host: '127.0.0.1', port: 0 }, plugins: [{ name: 'long-image-test', configureServer(vite) {
    vite.middlewares.use(async (req, res, next) => {
      if (req.url?.includes('/fixture-image/')) {
        res.setHeader('content-type', 'image/svg+xml'); res.end(svg(req.url.includes('normal') ? 800 : req.url.includes('ultra') ? 50000 : 6000)); return
      }
      if (!req.url?.includes('/image-acceptance') || req.url.includes('html-proxy')) return next()
      const html = await vite.transformIndexHtml('/pc/image-acceptance', `<!doctype html><html><head><meta name="viewport" content="width=device-width, initial-scale=1"></head><body><div id="root"></div><script type="module">
        import React from 'react'; import {createRoot} from 'react-dom/client';
        import Attachments from '/src/features/friends/SocialMessageAttachments.tsx';
        createRoot(document.getElementById('root')).render(React.createElement(Attachments,{attachments:['normal','tall','ultra'].map(kind=>({kind:'image',mime_type:'image/svg+xml',display_name:kind,url:location.origin+'/fixture-image/'+kind}))}));
      </script></body></html>`)
      res.setHeader('content-type', 'text/html'); res.end(html)
    })
  } }] })
  await server.listen()
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`
  const browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) })
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
    const errors = [], cases = []; page.on('pageerror', e => errors.push(e.message)); page.setDefaultTimeout(15000)
    await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort())
    await page.goto(origin + '/pc/image-acceptance')
    const dialog = () => page.getByRole('dialog'), stage = () => page.getByLabel('图片阅读区域', { exact: true })
    const image = () => dialog().locator('img'), scale = () => stage().getAttribute('data-scale').then(Number)
    async function open(name) { try { await page.getByRole('button', { name: '查看图片：' + name, exact: true }).click(); } catch (error) { console.error(JSON.stringify({ errors, body: await page.locator('body').innerText() })); throw error; } await page.waitForFunction(() => { const el = document.querySelector('dialog img'); return el?.style.visibility !== 'hidden' && el?.naturalWidth > 0 }) }
    async function close() { await page.getByRole('button', { name: '关闭图片预览', exact: true }).click(); await dialog().waitFor({ state: 'detached' }) }
    await open('normal'); assert.equal(await stage().getAttribute('data-reading'), 'false')
    let before = await scale(); await stage().hover(); await page.mouse.wheel(0, -200)
    await page.waitForFunction(value => Number(document.querySelector('[data-scale]').dataset.scale) > value, before)
    cases.push('ordinary-photo-overview-wheel-zoom')
    await close(); assert(await page.getByRole('button', { name: '查看图片：normal', exact: true }).evaluate(el => el === document.activeElement))
    cases.push('close-restores-trigger-focus')
    for (const name of ['tall', 'ultra']) {
      await open(name); assert.equal(await stage().getAttribute('data-reading'), 'true')
      let im = await image().boundingBox(), box = await stage().boundingBox()
      assert(Math.abs(im.width - 1000) < 2 && Math.abs(im.y - box.y) < 2)
      before = await scale(); await stage().hover(); await page.mouse.wheel(0, 450)
      await page.waitForFunction(y => document.querySelector('dialog img').getBoundingClientRect().y < y - 100, im.y)
      assert.equal(await scale(), before)
      await page.keyboard.down('Control'); await page.mouse.wheel(0, -200); await page.keyboard.up('Control')
      await page.waitForFunction(value => Number(document.querySelector('[data-scale]').dataset.scale) > value, before)
      await stage().focus(); await page.keyboard.press('End'); im = await image().boundingBox(); box = await stage().boundingBox()
      assert(Math.abs(im.y + im.height - box.y - box.height) < 2)
      await page.getByRole('button', { name: '回到图片顶部' }).click(); im = await image().boundingBox(); assert(Math.abs(im.y - box.y) < 2)
      await page.getByRole('button', { name: '整图', exact: true }).click(); im = await image().boundingBox(); assert(im.height <= box.height + 2)
      await page.getByRole('button', { name: '长图阅读', exact: true }).click()
      await page.screenshot({ path: path.join(output, `${engine}-${name}.png`) })
      await close(); cases.push(`${name}-auto-width-scroll-ctrlzoom-bottom-top-overview`)
    }
    await open('tall')
    for (const width of [320, 390, 1024]) {
      await page.setViewportSize({ width, height: 844 })
      await page.waitForFunction(w => Math.abs(document.querySelector('dialog img').getBoundingClientRect().width - Math.min(w, 1000)) < 2, width)
      const controls = await dialog().locator('button,a').evaluateAll(nodes => nodes.filter(el => el.getBoundingClientRect().height > 0).map(el => el.getBoundingClientRect().toJSON()))
      assert(controls.every(box => box.x >= -1 && box.right <= width + 1))
      cases.push(`responsive-reading-${width}`)
    }
    await stage().focus(); await page.keyboard.press('Escape'); await dialog().waitFor({ state: 'detached' })
    assert.deepEqual(errors, []); cases.push('escape-no-page-errors')
    console.log(JSON.stringify({ status: 'passed', engine, cases, productionWrites: 0 }))
  } finally { await browser.close(); await server.close() }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
