import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { webcrypto } from 'node:crypto'
import vm from 'node:vm'
import { createService } from './service.mjs'
import { assetContent } from './assets.mjs'

const id = '00000000-0000-4000-8000-000000000001', other = '00000000-0000-4000-8000-000000000002'
const source = name => readFileSync(new URL('../../android/app/src/main/assets/' + name, import.meta.url), 'utf8')
const kotlin = readFileSync(new URL('../../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebConversationRead.kt', import.meta.url), 'utf8')
const scripts = [...kotlin.matchAll(/"(chatgpt_web_[a-z_]+\.js)"/g)].map(match => match[1])
const tick = () => new Promise(resolve => setImmediate(resolve))
function fixture(options = {}) {
  let account = 'Bearer synthetic-reader-account', now = Date.now()
  const calls = [], data = options.data || Buffer.from('Synthetic attachment text\n中文😀')
  const msg = { id: 'visible-message', author: { role: 'user' }, content: { parts: ['A test attachment'] },
    metadata: options.shared ? { shared_library_file_references: [{ library_file_id: 'libfile_synthetic',
      name: 'fixture.txt', mime_type: 'text/plain', size_bytes: data.length, display_path: '/Shared/fixture.txt', entrypoint: 'library' }] }
      : { attachments: [{ id: 'file-synthetic', name: 'fixture.txt', mime_type: options.mime || 'text/plain' }] } }
  if (options.image) {
    msg.content.parts.push({ content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-synthetic' })
  }
  const messages = options.messages || [msg]
  const window = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' }, crypto: webcrypto,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    AbortController, setTimeout, clearTimeout, btoa, atob,
    fetch: async (path, init) => {
      const url = new URL(path, window.location.origin)
      calls.push({ path: url.pathname, method: init?.method, redirect: init?.redirect })
      let response
      if (url.pathname === `/backend-api/conversations/${id}`) response = Response.json({ id, messages })
      else if (url.pathname === '/backend-api/files/download/file-synthetic') {
        response = Response.json({ status: 'success', file_id: options.wrongId ? 'file-other' : 'file-synthetic',
          download_url: 'https://files.oaiusercontent.com/fixture?sig=synthetic-secret' })
        if (options.changeAccount) account = 'Bearer synthetic-other-account'
      } else {
        response = new Response(data, { headers: { 'content-type': options.responseMime || options.mime || 'text/plain',
          'content-length': String(options.length ?? data.length) } })
      }
      Object.defineProperty(response, 'url', { value: options.redirect && url.hostname !== 'chatgpt.com' ? 'https://invalid.example/' : url.href })
      return response
    } }
  const context = vm.createContext({ window, URL, URLSearchParams, AbortController, Uint8Array, TextDecoder, TextEncoder,
    Date: class extends Date { static now() { return now } }, setTimeout, clearTimeout })
  for (const name of scripts) vm.runInContext(source(name), context)
  const host = { source: 'apk', read: async input => window.__elonConversationReader.run(input) }
  const service = createService({ env: { ELON_WEB_CONVERSATION_IDS: `${id},${other}`, ELON_APK_MCP_URL: 'http://127.0.0.1:8787' },
    factories: { apk: async () => host }, sleep: tick })
  return { service, window, calls, data, advance: ms => { now += ms },
    page: () => service.read({ reference: id, source: 'apk' }) }
}

test('Win and APK load the same private history and media dependencies in the same order', () => {
  const rust = readFileSync(new URL('../../desktop-shell/src-tauri/src/local_ai_browser/conversation_scripts.rs', import.meta.url), 'utf8')
  assert.deepEqual([...rust.matchAll(/asset!\("([a-z_]+)"\)/g)].map(match => match[1] + '.js'), scripts)
  const packed = readFileSync(new URL('../../server/src/node_agent_cli_mcp/conversation.rs', import.meta.url), 'utf8')
  assert.match(packed, /include_str!\("\.\.\/\.\.\/\.\.\/scripts\/web-conversations\/assets.mjs"\)/)
  const adapter = readFileSync(new URL('../../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt', import.meta.url), 'utf8')
  assert.match(adapter, /ChatGptWebConversationRead\(context, webView, documentSession::snapshot, onWebExecutionRequested\)/)
  assert.ok(kotlin.indexOf('requestExecution()') < kotlin.indexOf('webView.evaluateJavascript'))
})

test('actual shared download owner returns chunked bytes through an opaque MCP handle without a UI adapter', async () => {
  for (const shared of [false, true]) {
    const f = fixture({ shared, data: Buffer.from('中文😀\n'.repeat(9000)) })
    const page = await f.page(), attachment = page.blocks.find(block => block.type === 'attachment')
    assert.ok(attachment.asset_handle)
    assert.equal(attachment.asset_cursor, undefined)
    assert.doesNotMatch(JSON.stringify(page), /Bearer|synthetic-secret|file-synthetic/)
    const value = await f.service.asset({ reference: id, asset_handle: attachment.asset_handle })
    assert.deepEqual(value.bytes, f.data)
    assert.equal(value.metadata.byte_count, f.data.length)
    assert.equal(value.metadata.source, 'apk')
    assert.ok(f.calls.every(call => call.method === 'GET'))
    assert.equal(f.window.location.href, 'https://chatgpt.com/')
    assert.equal(f.window.elonChatGptFileDownload, undefined)
    assert.equal(assetContent(value).content[1].type, 'resource')
    await assert.rejects(f.service.asset({ reference: other, asset_handle: attachment.asset_handle }), /mismatched/)
    await assert.rejects(f.service.asset({ reference: id, asset_handle: 'invented' }), /mismatched/)
  }
})

test('images are genuine MCP image content, UTF-8 files are text, PDF is a file resource', async () => {
  const png = Buffer.from('89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000b49444154789c636000020000050001a5f645400000000049454e44ae426082', 'hex')
  for (const options of [{ data: png, mime: 'image/png', image: true, type: 'image' },
    { data: Buffer.from('A short UTF-8 file 中文'), mime: 'text/plain', type: 'text' },
    { data: Buffer.from('%PDF-1.7\nsynthetic'), mime: 'application/pdf', type: 'resource' }]) {
    const f = fixture(options), page = await f.page()
    const value = await f.service.asset({ reference: id, asset_handle: page.blocks.find(block => block.type === 'attachment').asset_handle })
    assert.equal(assetContent(value).content[1].type, options.type)
    assert.deepEqual(value.bytes, options.data)
  }
  assert.equal(assetContent({ bytes: Buffer.from('not an image'), metadata: { media_type: 'image/png' } }).content[1].type, 'resource')
})

test('existing ownership, HTTP and size guards apply to attachment reads', async () => {
  for (const options of [{ wrongId: true }, { changeAccount: true }, { redirect: true },
    { responseMime: 'text/html' }, { length: 9 * 1024 * 1024 }]) {
    const f = fixture(options), page = await f.page()
    await assert.rejects(f.service.asset({ reference: id, asset_handle: page.blocks[1].asset_handle }),
      /conversation_read_failed|download_|identity_changed/)
  }
  const f = fixture(), page = await f.page()
  f.advance(181000)
  await assert.rejects(f.service.asset({ reference: id, asset_handle: page.blocks[1].asset_handle }), /cursor_expired/)
})

test('saved writing blocks and code use existing history semantics; internal tool text stays private', async () => {
  const rich = { id: 'rich-message', author: { role: 'assistant' }, content: { content_type: 'text',
    parts: [':::writing{id="block-1" title="Draft"}\nOld\n:::\n```python\nprint(1)\n```'] },
    metadata: { writing_blocks: { 'block-1': { content: 'Current saved text', title: 'Saved title' } } } }
  const hidden = { id: 'tool-hidden', author: { role: 'tool', name: 'internal' },
    content: { content_type: 'multimodal_text', parts: ['PRIVATE TOOL TEXT', { content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-hidden' }] } }
  const generated = { ...hidden, id: 'generated-image', author: { role: 'tool', name: 't2uay3k.sj1i4kz' } }
  const f = fixture({ messages: [rich, hidden, generated] }), blocks = []
  let cursor, page
  do {
    page = await f.service.read({ reference: id, source: 'apk', ...(cursor ? { cursor } : {}) })
    blocks.push(...page.blocks); cursor = page.next_cursor
  } while (page.has_more)
  assert.ok(blocks.some(block => block.type === 'writing_block' && block.text === 'Current saved text'))
  assert.ok(blocks.some(block => block.type === 'code' && block.language === 'python'))
  assert.equal(blocks.filter(block => block.type === 'attachment').length, 1)
  assert.doesNotMatch(JSON.stringify(blocks), /PRIVATE TOOL TEXT|tool-hidden/)
})
