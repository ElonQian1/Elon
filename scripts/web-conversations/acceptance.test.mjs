import { test } from 'node:test'
import assert from 'node:assert/strict'
import { acceptSources, parseArgs } from './acceptance.mjs'

const reference = '00000000-0000-4000-8000-000000000001'
const read = () => ({ read_to_end: true, attachments_complete: true, content_complete: true,
  text_sha256: 'digest', text_characters: 5, attachment_references: 2,
  assets: [{ sha256: 'image-digest', bytes: 10, kind: 'image' }] })
test('CLI requires explicit device and authorization; rejects external APK transports', () => {
  const args = ['--project-root', '/repo', '--reference', reference, '--source', 'both']
  assert.throws(() => parseArgs(args), /apk_endpoint_not_configured/)
  assert.throws(() => parseArgs([...args, '--apk-url', 'http://example.com']), /invalid_local_endpoint/)
  assert.equal(parseArgs([...args, '--apk-url', 'http://127.0.0.1:8787']).source, 'both')
  assert.throws(() => parseArgs([...args, '--source', 'win']), /invalid_arguments/)
})
test('two complete reads compare hashes and typed byte manifests without private content', async () => {
  const closed = [], queried = []
  const outcome = await acceptSources({ sources: ['win', 'apk'], reference,
    open: async source => ({ close: () => closed.push(source), asset_revision: 'revision' }),
    verify: async ({ source }) => { queried.push(source); return read() } })
  assert.equal(outcome.status, 'passed'); assert.deepEqual(outcome.comparison, { comparable: true, matching: true })
  assert.deepEqual(closed, ['win', 'apk']); assert.deepEqual(queried, closed)
})
test('matching partial content never becomes a complete pass; device failures remain independent', async () => {
  const input = { sources: ['win', 'apk'], reference, open: async () => ({ close() {} }),
    verify: async () => ({ ...read(), content_complete: false, remaining_gaps: ['unsupported_message_content'] }) }
  const partial = await acceptSources(input)
  assert.equal(partial.status, 'partial'); assert.equal(partial.comparison.matching, true)
  const failed = await acceptSources({ ...input, verify: async ({ source }) => {
    if (source === 'win') throw Error('Network included a private token')
    return read()
  } })
  assert.equal(failed.status, 'failed'); assert.equal(failed.results.apk.status, 'passed')
  assert.equal(failed.comparison.comparable, false); assert.doesNotMatch(JSON.stringify(failed), /private|token/)
  const mismatch = await acceptSources({ ...input, verify: async ({ source }) => ({ ...read(), text_sha256: source }) })
  assert.equal(mismatch.status, 'failed'); assert.equal(mismatch.comparison.matching, false)
})
