// Real attachment component and message layout, synthetic images, no account or network writes.
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const root = path.resolve(__dirname, '..')
const output = path.resolve(root, '../.ai-tmp/social-image-thumbnails')
const engine = process.env.BROWSER_ENGINE || 'chromium'
const sizes = { landscape: [1920, 1200], wide: [2560, 664], portrait: [800, 1200], tall: [1080, 6000], ultra: [1080, 50000], small: [16, 16] }

async function main() {
  fs.mkdirSync(output, { recursive: true })
  const { createServer } = await import(pathToFileURL(path.join(root, 'node_modules/vite/dist/node/index.js')))
  const server = await createServer({ root, appType: 'custom', server: { host: '127.0.0.1', port: 0 }, plugins: [{ name: 'thumbnail-acceptance', configureServer(vite) {
    vite.middlewares.use(async (req, res, next) => {
      if (req.url?.startsWith('/fixture-image/')) {
        const [width, height] = sizes[req.url.split('/').pop()] || sizes.landscape
        res.setHeader('content-type', 'image/svg+xml')
        res.end(`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="100%" height="100%" fill="#4e8068"/><path d="M0 0L${width} ${height}M${width} 0L0 ${height}" stroke="#fff" stroke-width="8"/><rect x="4" y="4" width="${width - 8}" height="${height - 8}" fill="none" stroke="#fff" stroke-width="8"/></svg>`)
        return
      }
      if (!req.url?.includes('/thumbnail-acceptance') || req.url.includes('html-proxy')) return next()
      const html = await vite.transformIndexHtml('/pc/thumbnail-acceptance', `<!doctype html><html><head><meta name="viewport" content="width=device-width, initial-scale=1"><style>body{margin:0;padding:16px;background:#212225;color:#eee;font:14px sans-serif;box-sizing:border-box}*{box-sizing:border-box}</style></head><body><div id="root"></div><script type="module">
        import React from 'react'; import {createRoot} from 'react-dom/client';
        import Attachments from '/src/features/friends/SocialMessageAttachments.tsx';
        import styles from '/src/features/friends/FriendsPage.module.css';
        const rows = ${JSON.stringify(Object.keys(sizes))}.flatMap((kind,index)=>[false,true].map(own=>{
          const name=kind+(own?'-out':'-in');
          return React.createElement('article',{key:name,'data-case':name,className:styles.msgRow+' '+(own?styles.ownRow:'')},
            React.createElement('div',{className:styles.avatar},'A'),
            React.createElement('div',{className:styles.msgBody,'data-body':true},
              React.createElement('div',{className:styles.msgMeta},'Fixture sender'),
              index%2===0 && React.createElement('div',{className:styles.msgContent},'Image attachment with a longer message alongside it.'),
              React.createElement(Attachments,{attachments:[{kind:'image',mime_type:'image/svg+xml',display_name:name,url:location.origin+'/fixture-image/'+kind}]})))
        }));
        createRoot(document.getElementById('root')).render(rows);
      </script></body></html>`)
      res.setHeader('content-type', 'text/html'); res.end(html)
    })
  } }] })
  await server.listen()
  let browser
  try {
    const origin = `http://127.0.0.1:${server.httpServer.address().port}`
    browser = await require('playwright')[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) })
    const page = await browser.newPage(), errors = [], measurements = []
    page.on('pageerror', error => errors.push(error.message))
    await page.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort())
    for (const width of [1440, 390, 320]) for (const zoom of [1, 2]) {
      await page.setViewportSize({ width, height: 900 })
      await page.goto(origin + '/pc/thumbnail-acceptance')
      await page.waitForSelector('[data-preview-image] img')
      await page.locator('[data-preview-image] img').evaluateAll(images => images.forEach(image => image.loading = 'eager'))
      await page.waitForFunction(() => [...document.querySelectorAll('[data-preview-image] img')].every(image => image.naturalWidth > 0))
      await page.locator('body').evaluate((body, value) => { body.style.zoom = String(value) }, zoom)
      const rows = await page.locator('[data-case]').evaluateAll(nodes => nodes.map(row => {
        const button = row.querySelector('[data-preview-image]'), image = button.querySelector('img')
        const b = button.getBoundingClientRect(), i = image.getBoundingClientRect(), p = row.querySelector('[data-body]').getBoundingClientRect()
        return { name: row.dataset.case, button: b.toJSON(), image: i.toJSON(), parent: p.toJSON(), background: getComputedStyle(button).backgroundColor, ratio: image.naturalWidth / image.naturalHeight }
      }))
      await page.screenshot({ path: path.join(output, `${engine}-${width}-${zoom}.png`) })
      for (const row of rows) {
        measurements.push({ width, zoom, name: row.name, buttonWidth: row.button.width, imageWidth: row.image.width })
        assert(Math.abs(row.button.width - Math.max(row.image.width, 44 * zoom)) <= 1 && Math.abs(row.button.height - Math.max(row.image.height, 44 * zoom)) <= 1,
          `thumbnail has empty wrapper space: ${JSON.stringify(measurements.at(-1))}`)
        assert.equal(row.background, 'rgba(0, 0, 0, 0)', `opaque image wrapper: ${row.name}`)
        assert(Math.abs(row.image.width / row.image.height - row.ratio) < 0.03, `distorted image: ${row.name}`)
        assert(row.image.width <= 320 * zoom + 1 && row.image.height <= 300 * zoom + 1)
        assert(row.button.x >= row.parent.x - 1 && row.button.right <= row.parent.right + 1, `parent overflow: ${row.name}`)
        if (row.name.endsWith('-out')) assert(Math.abs(row.button.right - row.parent.right) <= 1, `outgoing alignment: ${row.name}`)
        assert(row.button.x >= -1 && row.button.right <= width + 1, `viewport overflow: ${row.name}`)
      }
    }
    await page.setViewportSize({ width: 1280, height: 900 })
    await page.locator('body').evaluate(body => { body.style.zoom = '1' })
    const trigger = page.getByRole('button', { name: '查看图片：wide-out', exact: true })
    await trigger.click(); await page.getByRole('dialog').waitFor()
    await page.getByRole('button', { name: '关闭图片预览', exact: true }).click()
    await page.getByRole('dialog').waitFor({ state: 'detached' })
    assert(await trigger.evaluate(button => button === document.activeElement))
    assert.deepEqual(errors, [])
    console.log(JSON.stringify({ status: 'passed', engine, measuredThumbnails: measurements.length, preview: 'opened-and-closed', productionWrites: 0 }))
  } finally { await browser?.close(); await server.close() }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
