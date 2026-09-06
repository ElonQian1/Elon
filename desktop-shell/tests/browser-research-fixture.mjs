#!/usr/bin/env node
// Loopback-only synthetic acceptance site; no exchange access or real credentials.
import http from 'node:http'

const page = `<!doctype html><html lang="zh-CN"><meta charset="utf-8">
<title>网页研究验收样例</title><style>body{font:18px system-ui;margin:48px;max-width:800px}pre{white-space:pre-wrap}</style>
<h1>网页研究验收样例</h1><p>这是本机合成资料，验证同一内核支持其他网站。</p>
<button id="reload">重新读取合成数据</button><pre id="output">等待读取</pre>
<script src="/fixture.js"></script></html>`
const script = `const researchFixtureMarker = 'GENERIC_RESEARCH_FIXTURE_V1';
async function loadFixture() {
  const response = await fetch('/api/catalog/list', { method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({page: 1, rows: 3, category: 'synthetic', csrfToken: 'SYNTHETIC_SECRET'}) });
  document.getElementById('output').textContent = JSON.stringify(await response.json(), null, 2);
}
document.getElementById('reload').addEventListener('click', loadFixture);
loadFixture();`
const server = http.createServer((request, response) => {
  response.setHeader('cache-control', 'no-store')
  response.setHeader('x-content-type-options', 'nosniff')
  if (request.url === '/' && request.method === 'GET') {
    response.writeHead(200, {'content-type': 'text/html; charset=utf-8'}).end(page)
  } else if (request.url === '/fixture.js' && request.method === 'GET') {
    response.writeHead(200, {'content-type': 'application/javascript; charset=utf-8'}).end(script)
  } else if (request.url === '/api/catalog/list' && request.method === 'POST') {
    let bytes = 0
    request.on('data', chunk => { bytes += chunk.length; if (bytes > 4096) request.destroy() })
    request.on('end', () => {
      response.writeHead(200, {'content-type': 'application/json'}).end(JSON.stringify({
        code: 'OK', success: true, data: {total: 1, unexpectedCollection: [
          {itemId: 'fixture-001', symbol: 'ESK', token: 'ESK', quantity: '12.3400', label: '中文业务值', accessToken: 'SYNTHETIC_SECRET'},
        ]},
      }))
    })
  } else {
    response.writeHead(404, {'content-type': 'text/plain'}).end('Not found')
  }
})
server.listen(0, '127.0.0.1', () => {
  const origin = `http://127.0.0.1:${server.address().port}`
  process.stdout.write(JSON.stringify({schema: 'yilong.browser-research.site.v1', id: 'local-fixture',
    name: '无交易通用验收', entry_url: origin + '/', navigation_origins: [origin],
    resource_origins: [origin], api_origins: [origin], identity_origins: []}) + '\n')
})
const shutdown = () => { server.closeAllConnections(); server.close(() => process.exit(0)) }
process.on('SIGINT', shutdown)
process.on('SIGTERM', shutdown)
setTimeout(shutdown, 60 * 60 * 1000).unref()
