import { spawn } from 'node:child_process'
import { createHash } from 'node:crypto'
import { prepareRegistration } from '../web-conversations/registration.mjs'
import { conversationId } from '../web-conversations/service.mjs'

const hash = value => createHash('sha256').update(value).digest('hex')
export async function openReader({ env, projectRoot, reference, storageRoot, base }) {
  const registration = await prepareRegistration({ projectRoot, references: [reference], storageRoot })
  const child = spawn(registration.command, [registration.entrypoint], { windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'],
    env: { ...env, ELON_PROJECT_ROOT: projectRoot, ELON_WEB_CONVERSATION_IDS: registration.ids,
      ELON_NODE_ADMIN_URL: base, ELON_APK_MCP_URL: '' } })
  let sequence = 0, buffer = '', pending, closed = false
  const fail = code => {
    if (pending) { clearTimeout(pending.timer); pending.reject(Error(code)); pending = null }
  }
  child.on('error', () => { closed = true; fail('reader_process_failed') })
  child.on('exit', () => { closed = true; fail('reader_process_closed') })
  child.stdin.on('error', () => fail('reader_process_closed'))
  child.stdout.setEncoding('utf8')
  child.stdout.on('data', chunk => {
    buffer += chunk
    if (buffer.length > 13 * 1024 * 1024) { fail('reader_response_limit'); child.kill(); return }
    let end
    while ((end = buffer.indexOf('\n')) >= 0) {
      const line = buffer.slice(0, end); buffer = buffer.slice(end + 1)
      try {
        const reply = JSON.parse(line)
        if (!pending || reply.id !== pending.id || reply.jsonrpc !== '2.0') throw Error()
        const current = pending; clearTimeout(current.timer); pending = null
        if (reply.error || !reply.result) current.reject(Error('reader_protocol_failed'))
        else current.resolve(reply.result)
      } catch { fail('reader_protocol_failed') }
    }
  })
  function request(method, params) {
    if (closed || pending) throw Error('reader_process_unavailable')
    return new Promise((resolve, reject) => {
      const id = ++sequence
      pending = { id, resolve, reject, timer: setTimeout(() => { fail('reader_request_timeout'); child.kill() }, 155000) }
      child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
    })
  }
  async function call(name, args) {
    const reply = await request('tools/call', { name, arguments: args })
    if (reply.isError) {
      let code
      try { code = JSON.parse(reply.content?.[0]?.text).error } catch {}
      throw Error(/^[a-z][a-z_0-9]{0,79}$/.test(code || '') ? code : 'reader_call_failed')
    }
    return reply
  }
  try {
    await request('initialize', { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'yilong-win-acceptance', version: '1.0.0' } })
    child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n')
    const names = (await request('tools/list', {})).tools?.map(tool => tool.name) || []
    if (!['web_conversation_read', 'web_conversation_asset', 'web_conversation_scope', 'web_conversation_connect'].every(name => names.includes(name))) throw Error('reader_tools_missing')
    return { call, close: () => { closed = true; fail('reader_process_closed'); child.kill() },
      asset_revision: registration.entrypoint.split(/[\\/]/).at(-2) }
  } catch (error) { child.kill(); throw error }
}

function structured(reply) {
  try { return JSON.parse(reply.content?.[0]?.text) } catch { throw Error('invalid_reader_result') }
}

// Accumulate hashes and structural counts only. Private bodies, titles, filenames,
// attachment handles and page cursors never enter receipts or progress events.
export async function verifyReader({ client, reference, progress = async () => {}, now = Date.now }) {
  const id = conversationId(reference), textHash = createHash('sha256'), seen = new Set(), gaps = new Set()
  const counts = { pages: 0, blocks: 0, text_characters: 0, attachment_references: 0, attachments_read: 0, images: 0, files: 0 }
  const assets = [], deadline = now() + 900000
  let cursor, revision, total, messageCount, attachmentCount, complete = true
  do {
    if (counts.pages >= 50000 || now() >= deadline) throw Error('acceptance_read_limit')
    const page = structured(await client.call('web_conversation_read', { reference, source: 'win', ...(cursor ? { cursor } : {}) }))
    if (page.schema !== 'yilong.web-conversation.snapshot.v1' || page.source !== 'win' || page.conversation_id !== id ||
        !/^[a-f0-9]{32}$/.test(page.revision || '') || !Array.isArray(page.blocks) || page.blocks.length > 2 ||
        page.block_offset !== counts.blocks || !Number.isSafeInteger(page.total_blocks) || page.total_blocks > 100000 ||
        typeof page.text_complete !== 'boolean' || !Array.isArray(page.gaps) ||
        typeof page.has_more !== 'boolean' || revision && (page.revision !== revision || page.total_blocks !== total ||
          page.message_count !== messageCount || page.attachment_count !== attachmentCount)) throw Error('invalid_reader_result')
    revision = page.revision; total = page.total_blocks
    messageCount = page.message_count; attachmentCount = page.attachment_count
    complete &&= page.text_complete
    for (const gap of page.gaps) gaps.add(/^[a-z_0-9]{1,80}$/.test(gap) ? gap : 'unknown_content_gap')
    for (const block of page.blocks) {
      if (block.type === 'attachment') {
        counts.attachment_references++
        if (!block.asset_handle) throw Error('attachment_handle_missing')
        const reply = await client.call('web_conversation_asset', { reference, asset_handle: block.asset_handle })
        const meta = structured(reply), body = reply.content?.[1]
        let bytes
        if (body?.type === 'image') { bytes = Buffer.from(body.data, 'base64'); counts.images++ }
        else if (body?.type === 'resource') { bytes = Buffer.from(body.resource?.blob || '', 'base64'); counts.files++ }
        else if (body?.type === 'text') { bytes = Buffer.from(body.text, 'utf8'); counts.files++ }
        else throw Error('attachment_content_missing')
        const digest = hash(bytes)
        if (meta.source !== 'win' || meta.conversation_id !== id || meta.byte_count !== bytes.length ||
            bytes.length > 8 * 1024 * 1024 || meta.sha256 !== digest) throw Error('attachment_digest_mismatch')
        counts.attachments_read++
        if (!assets.some(item => item.sha256 === digest)) assets.push({ bytes: bytes.length, sha256: digest, kind: body.type })
      } else if (typeof block.text === 'string' && !block.representation) {
        textHash.update(block.text); counts.text_characters += [...block.text].length
      }
    }
    counts.pages++; counts.blocks += page.blocks.length
    if (page.has_more !== (counts.blocks < total) || counts.blocks > total) throw Error('invalid_reader_pagination')
    cursor = page.next_cursor
    if (page.has_more && (!page.blocks.length || typeof cursor !== 'string' || !cursor || seen.has(cursor))) throw Error('invalid_reader_pagination')
    if (!page.has_more && cursor) throw Error('invalid_reader_pagination')
    seen.add(cursor)
    await progress({ ...counts })
  } while (cursor)
  if (!Number.isSafeInteger(messageCount) || !Number.isSafeInteger(attachmentCount) ||
      counts.attachment_references !== attachmentCount) throw Error('reader_counts_mismatch')
  // Snapshot metadata predates asset reads; only that specific gap can be resolved.
  gaps.delete('attachment_bytes_not_read')
  return { ...counts, message_count: messageCount, unique_attachments: assets.length, assets,
    text_sha256: textHash.digest('hex'), text_complete: complete, remaining_gaps: [...gaps].sort(),
    read_to_end: true, attachments_complete: counts.attachments_read === counts.attachment_references,
    content_complete: complete && gaps.size === 0 }
}
