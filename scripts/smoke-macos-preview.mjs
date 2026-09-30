#!/usr/bin/env node
// Run the bundled WKWebView against a loopback fixture, never a user's account.
import http from 'node:http'
import fs from 'node:fs/promises'
import { spawn } from 'node:child_process'

if (process.platform !== 'darwin') throw new Error('macOS smoke host required')
const binary = process.argv[2]
if (!binary) throw new Error('Bundled executable required')
let resolveReceipt
const receiptPromise = new Promise(resolve => { resolveReceipt = resolve })
const server = http.createServer(async (request, response) => {
  if (request.method === 'POST' && request.url === '/receipt') {
    let body = ''
    for await (const chunk of request) {
      body += chunk
      if (body.length > 4096) { response.writeHead(413).end(); return }
    }
    try { resolveReceipt(JSON.parse(body)) } catch { resolveReceipt({ error: 'invalid_receipt' }) }
    response.writeHead(204).end()
    return
  }
  response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
  response.end(`<!doctype html><meta charset="utf-8"><title>Mac renderer smoke</title>
    <main>一龙 macOS WKWebView fixture</main><script>
    window.addEventListener('load', async () => {
      let receipt;
      try {
        const providers = await window.__TAURI__.core.invoke('list_local_ai_web_providers');
        receipt = {renderer: true, ipc: true, providerCount: providers.length};
      } catch (error) { receipt = {error: String(error).slice(0, 300)}; }
      await fetch('/receipt', {method: 'POST', headers: {'content-type': 'application/json'},
        body: JSON.stringify(receipt)});
    });</script>`)
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const url = `http://127.0.0.1:${server.address().port}/pc`
const child = spawn(binary, [], {
  env: { ...process.env, ELON_DESKTOP_URL: url },
  stdio: ['ignore', 'pipe', 'pipe'],
})
let diagnostic = ''
for (const stream of [child.stdout, child.stderr]) {
  stream.on('data', chunk => { diagnostic = (diagnostic + chunk).slice(-4000) })
}
let timeout
try {
  const receipt = await Promise.race([
    receiptPromise,
    new Promise((_, reject) => {
      timeout = setTimeout(() => reject(new Error('WKWebView/IPC timeout')), 45000)
    }),
    new Promise((_, reject) => {
      child.once('error', reject)
      child.once('exit', code => reject(new Error(`App exited early: ${code}`)))
    }),
  ])
  if (!receipt.renderer || !receipt.ipc || receipt.providerCount < 2) {
    throw new Error(`Invalid renderer receipt: ${JSON.stringify(receipt)}`)
  }
  await fs.writeFile('.ai-tmp/macos-smoke.json', JSON.stringify({
    schema: 'elon.macos_preview.renderer_smoke.v1',
    platform: process.platform, architecture: process.arch,
    environment: 'loopback_fixture', ...receipt, user_acceptance: 'pending',
  }, null, 2) + '\n')
  console.log('MACOS_RENDERER_FIXTURE_SMOKE=passed')
} catch (error) {
  console.error(diagnostic)
  throw error
} finally {
  clearTimeout(timeout)
  child.kill('SIGTERM')
  setTimeout(() => child.kill('SIGKILL'), 2000).unref()
  server.closeAllConnections()
  server.close()
}
