#!/usr/bin/env node
import { mkdir, writeFile, realpath } from 'node:fs/promises'
import { createHash, randomUUID } from 'node:crypto'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { openReader, verifyReader } from '../win-conversation-acceptance/reader.mjs'
import { safeCode } from '../win-conversation-acceptance/control.mjs'
import { conversationId } from './service.mjs'
import { localUrl } from './local-rpc.mjs'

export function parseArgs(argv) {
  const values = {}
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i]
    if (!['--project-root', '--reference', '--source', '--apk-url'].includes(key) ||
        values[key] || !argv[i + 1] || argv[i + 1].startsWith('--')) throw Error('invalid_arguments')
    values[key] = argv[i + 1]
  }
  if (!values['--project-root'] || !['win', 'apk', 'both'].includes(values['--source'])) throw Error('explicit_project_and_source_required')
  const reference = conversationId(values['--reference']), source = values['--source']
  const apk = values['--apk-url'] ? localUrl(values['--apk-url']) : undefined
  if (source !== 'win' && !apk) throw Error('apk_endpoint_not_configured')
  return { projectRoot: values['--project-root'], reference, source, apk }
}

export async function acceptSources({ sources, reference, open, verify = verifyReader, progress = async () => {} }) {
  if (!sources.length || sources.some(s => !['win', 'apk'].includes(s)) || new Set(sources).size !== sources.length) throw Error('invalid_acceptance_source')
  const results = {}
  for (const source of sources) {
    let client
    try {
      client = await open(source)
      const read = await verify({ client, reference, source, progress: counts => progress({ source, ...counts }) })
      results[source] = { status: read.content_complete ? 'passed' : 'partial', reader_revision: client.asset_revision, read }
    } catch (error) { results[source] = { status: 'failed', error: safeCode(error) } }
    finally { client?.close() }
  }
  const a = results.win?.read, b = results.apk?.read
  const manifest = value => JSON.stringify(value.assets.map(a => [a.sha256, a.bytes, a.kind]).sort())
  const comparable = !!(a?.read_to_end && b?.read_to_end && a.attachments_complete && b.attachments_complete)
  const matching = comparable && a.text_sha256 === b.text_sha256 && a.text_characters === b.text_characters &&
    a.attachment_references === b.attachment_references && manifest(a) === manifest(b)
  return { status: Object.values(results).some(r => r.status === 'failed') || comparable && !matching ? 'failed' :
    Object.values(results).every(r => r.status === 'passed') ? 'passed' : 'partial', results,
    comparison: { comparable, ...(comparable ? { matching } : {}) } }
}

export async function run(argv, env = process.env) {
  const options = parseArgs(argv), projectRoot = await realpath(options.projectRoot)
  if (!env.LOCALAPPDATA || !path.isAbsolute(env.LOCALAPPDATA)) throw Error('local_storage_unavailable')
  const storageRoot = path.join(env.LOCALAPPDATA, 'Elon', 'web-conversation-reader')
  const receiptRoot = path.join(env.LOCALAPPDATA, 'Elon', 'conversation-acceptance-v1')
  let last = 0
  const outcome = await acceptSources({ sources: options.source === 'both' ? ['win', 'apk'] : [options.source], reference: options.reference,
    open: source => openReader({ env: { ...env, ELON_APK_MCP_URL: options.apk }, projectRoot,
      reference: options.reference, storageRoot, source }), progress: async event => {
      if (Date.now() - last >= 20000) { last = Date.now(); process.stdout.write(JSON.stringify({ status: 'reading', ...event }) + '\n') }
    } })
  const receipt = { schema: 'yilong.conversation_acceptance.v1', run_id: randomUUID(),
    binding: createHash('sha256').update(JSON.stringify([projectRoot.toLowerCase(), options.reference])).digest('hex'),
    completed_at: new Date().toISOString(), ...outcome }
  await mkdir(receiptRoot, { recursive: true })
  const file = path.join(receiptRoot, receipt.run_id + '.json')
  await writeFile(file, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx', mode: 0o600 })
  process.stdout.write(JSON.stringify({ receipt: file, ...receipt }) + '\n')
  return receipt.status === 'passed' ? 0 : receipt.status === 'partial' ? 2 : 1
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = await run(process.argv.slice(2)) }
  catch (error) { process.stdout.write(JSON.stringify({ status: 'failed', error: safeCode(error) }) + '\n'); process.exitCode = 1 }
}
