import { test } from 'node:test'
import assert from 'node:assert/strict'
import http from 'node:http'
import { spawn } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { once } from 'node:events'
import { apkSource } from './transport.mjs'

const id = '00000000-0000-4000-8000-000000000001'
const mcp = value => ({ jsonrpc: '2.0', id: 1, result: { content: [{ type: 'text', text: JSON.stringify(value) }] } })
async function localServer(t, handler) {
  const server = http.createServer(async (req, res) => {
    let input = ''; for await (const bytes of req) input += bytes
    try { res.setHeader('content-type', 'application/json'); res.end(JSON.stringify(await handler(req.url, input ? JSON.parse(input) : null))) }
    catch { res.writeHead(500); res.end('{}') }
  }).listen(0, '127.0.0.1')
  await once(server, 'listening')
  t.after(() => { server.closeAllConnections(); server.close() })
  return `http://127.0.0.1:${server.address().port}`
}

test('stdio to Win queue completes a real JSON-RPC exchange and exports no host token', { timeout: 10000 }, async t => {
  let command, polls = 0, base
  base = await localServer(t, (url, body) => {
    if (url === '/api/health') return { service: 'elon-node-agent', status: 'ok' }
    if (url === '/api/project-docs/mcp/bootstrap') {
      assert.equal(body.profile, 'browser_research'); return { ok: true, mcp: { url: base + '/mcp?token=synthetic-local-token' } }
    }
    assert.equal(url, '/mcp?token=synthetic-local-token')
    const args = body.params.arguments
    if (args.action === 'describe') return mcp({ commands: { read_conversation: {} } })
    if (args.action === 'hosts') return mcp({ hosts: [{ instance_id: 'synthetic-host' }] })
    if (args.action === 'submit') { command = args.payload; return mcp({ terminal: false, action: { action_id: 'action1' } }) }
    if (args.action === 'action_status') {
      polls++; const input = JSON.parse(command.query)
      assert.equal(command.kind, 'read_conversation'); assert.equal(command.instance_id, 'synthetic-host')
      return mcp({ terminal: true, action: { status: 'succeeded', receipt: { result: { reader: {
        status: 'ready', request_id: input.request_id, page: { schema: 'yilong.web-conversation.snapshot.v1',
          conversation_id: id, revision: 'a'.repeat(32), blocks: [], has_more: false, next_cursor: null,
          block_offset: 0, total_blocks: 0, text_complete: true, multimodal_complete: true, gaps: [] },
      } } } } })
    }
    throw new Error('unexpected_action')
  })
  const child = spawn(process.execPath, [fileURLToPath(new URL('./stdio.mjs', import.meta.url))], {
    windowsHide: true, env: { ...process.env, ELON_NODE_ADMIN_URL: base, ELON_WEB_CONVERSATION_IDS: id }, stdio: ['pipe', 'pipe', 'pipe'],
  })
  t.after(() => child.kill())
  let output = '', error = ''
  child.stdout.on('data', bytes => { output += bytes }); child.stderr.on('data', bytes => { error += bytes })
  const requests = [
    { id: 1, method: 'initialize' }, { id: 2, method: 'tools/list' },
    { id: 3, method: 'tools/call', params: { name: 'web_conversation_read', arguments: { reference: id } } },
  ]
  child.stdin.end(requests.map(r => JSON.stringify({ jsonrpc: '2.0', ...r })).join('\n') + '\n')
  const [code] = await once(child, 'exit')
  assert.equal(code, 0); assert.equal(error, '')
  const replies = output.trim().split('\n').map(JSON.parse)
  assert.equal(replies.length, 3); assert.equal(replies[1].result.tools.length, 3)
  const page = JSON.parse(replies[2].result.content[0].text)
  assert.equal(page.conversation_id, id); assert.equal(page.source, 'win'); assert.equal(page.has_more, false)
  assert.equal(polls, 1); assert.doesNotMatch(output, /synthetic-local-token/)
})

test('APK transport recovers the native inactive error envelope and refuses a changed device session', { timeout: 6000 }, async t => {
  let token = 'synthetic-apk-token', active = false, opens = 0
  const base = await localServer(t, (url, body) => {
    if (url === '/health') return { auth_token: token }
    assert.equal(url, '/mcp')
    const args = body.params.arguments; assert.equal(args.auth_token, token)
    if (args.action === 'open_chatgpt_web') { opens++; active = true; return mcp({ control_ok: true }) }
    assert.equal(args.action, 'chatgpt_read_conversation')
    return active ? mcp({ status: 'ready', request_id: args.request_id, page: {} }) : {
      jsonrpc: '2.0', id: 1, result: { isError: true,
        structuredContent: { control_ok: false, error: 'chatgpt_web_chat_inactive' } },
    }
  })
  const source = await apkSource({ ELON_APK_MCP_URL: base }), input = { conversation_id: id, request_id: 'request-001' }
  assert.equal((await source.read(input)).status, 'pending')
  assert.equal((await source.read(input)).status, 'ready'); assert.equal(opens, 1)
  token = 'synthetic-replacement'
  await assert.rejects(source.read(input), /apk_session_changed/)
})

test('APK retries only known read readiness errors, never other errors or failed navigation', async t => {
  let error = 'adapter_generation_not_ready', opens = 0
  const base = await localServer(t, (url, body) => {
    if (url === '/health') return { auth_token: 'synthetic-apk-token' }
    if (body.params.arguments.action === 'open_chatgpt_web') opens++
    return { jsonrpc: '2.0', id: 1, result: { isError: true, structuredContent: { error } } }
  })
  const source = await apkSource({ ELON_APK_MCP_URL: base })
  const input = { conversation_id: id, request_id: 'request-001' }
  assert.equal((await source.read(input)).status, 'pending')
  error = 'bridge_not_ready'
  assert.equal((await source.read(input)).status, 'pending')
  error = 'permission_denied'
  await assert.rejects(source.read(input), /mcp_call_failed/)
  assert.equal(opens, 0)
  error = 'chatgpt_web_chat_inactive'
  await assert.rejects(source.read(input), /mcp_call_failed/)
  assert.equal(opens, 1)
})
