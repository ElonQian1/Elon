import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { verifyReader } from './reader.mjs'

const id = '00000000-0000-4000-8000-000000000001', rev = 'a'.repeat(32)
const png = Buffer.from('89504e470d0a1a0a00000000', 'hex')
const sha = value => createHash('sha256').update(value).digest('hex')
const wrap = value => ({ content: [{ type: 'text', text: JSON.stringify(value) }] })
function fixture() {
  const pages = [
    [{ type: 'text', text: '私有文字😀' }, { type: 'attachment', name: 'private.png', asset_handle: 'private-handle' }],
    [{ type: 'code', representation: 'structured_content', text: 'duplicate' }, { type: 'text', text: 'more' }],
  ].map((blocks, index) => ({ schema: 'yilong.web-conversation.snapshot.v1', source: 'win', conversation_id: id,
    revision: rev, blocks, block_offset: index * 2, total_blocks: 4, message_count: 2, attachment_count: 1,
    has_more: index === 0, next_cursor: index === 0 ? 'private-cursor' : null,
    text_complete: true, gaps: ['attachment_bytes_not_read'] }))
  const asset = { content: [{ type: 'text', text: JSON.stringify({ conversation_id: id, source: 'win', name: 'private.png',
    sha256: sha(png), byte_count: png.length }) }, { type: 'image', mimeType: 'image/png', data: png.toString('base64') }] }
  const calls = [], progress = []
  return { pages, asset, calls, progress, input: { reference: id, progress: async value => progress.push(value), client: {
    call: async (tool, args) => { calls.push([tool, args]); return tool === 'web_conversation_asset' ? asset : wrap(pages[args.cursor ? 1 : 0]) },
  } } }
}
test('real typed images are digested immediately; structured duplicates and private content stay out of receipts', async () => {
  const f = fixture(), result = await verifyReader(f.input)
  assert.equal(result.pages, 2); assert.equal(result.images, 1); assert.equal(result.content_complete, true)
  assert.equal(result.text_sha256, sha('私有文字😀more')); assert.equal(result.text_characters, 9)
  assert.equal(f.calls[1][0], 'web_conversation_asset')
  assert.doesNotMatch(JSON.stringify([result, f.progress]), /私有|private|duplicate|Bearer/)
})
test('unsupported content stays partial even when all images were read', async () => {
  const f = fixture(); f.pages[0].text_complete = false; f.pages[0].gaps.push('unsupported_message_content')
  const result = await verifyReader(f.input)
  assert.equal(result.content_complete, false); assert.deepEqual(result.remaining_gaps, ['unsupported_message_content'])
  assert.equal(result.attachments_complete, true)
})
test('explicit APK acceptance follows APK pages and bytes and rejects cross-device substitutions', async () => {
  const f = fixture(); f.input.source = 'apk'
  for (const page of f.pages) page.source = 'apk'
  const meta = JSON.parse(f.asset.content[0].text); meta.source = 'apk'
  f.asset.content[0].text = JSON.stringify(meta)
  const result = await verifyReader(f.input)
  assert.equal(result.source, 'apk'); assert.equal(result.content_complete, true)
  assert.ok(f.calls.filter(([name]) => name === 'web_conversation_read').every(([, args]) => args.source === 'apk'))
  meta.source = 'win'; f.asset.content[0].text = JSON.stringify(meta)
  await assert.rejects(verifyReader(f.input), /attachment_digest_mismatch/)
  await assert.rejects(verifyReader({ ...f.input, source: 'auto' }), /invalid_acceptance_source/)
})
test('missing bytes, altered digests, wrong conversation and broken continuation are rejected', async () => {
  const cases = [
    f => { f.asset.content.pop() },
    f => { f.asset.content[1].data = Buffer.from('different').toString('base64') },
    f => { f.pages[1].conversation_id = 'other' },
    f => { f.pages[1].revision = 'b'.repeat(32) },
    f => { f.pages[0].next_cursor = null },
    f => { f.pages[1].attachment_count = 9 },
  ]
  for (const change of cases) { const f = fixture(); change(f); await assert.rejects(verifyReader(f.input)) }
})
