import { createHash } from 'node:crypto'
import { readFile, mkdir, writeFile, realpath } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { conversationId } from './service.mjs'

const assets = ['stdio.mjs', 'service.mjs', 'transport.mjs', 'local-rpc.mjs', 'win-runtime.mjs']
export async function prepareRegistration({ projectRoot, references, storageRoot }) {
  if (!Array.isArray(references) || !references.length || references.length > 16) throw Error('invalid_conversation_grant')
  const ids = [...new Set(references.map(conversationId))].sort()
  const root = await realpath(projectRoot)
  const values = await Promise.all(assets.map(async name => [name, await readFile(new URL(name, import.meta.url))]))
  const hash = createHash('sha256')
  for (const [name, body] of values) hash.update(name).update(body)
  const directory = path.join(storageRoot, hash.digest('hex'))
  await mkdir(directory, { recursive: true })
  for (const [name, body] of values) {
    const target = path.join(directory, name)
    try { await writeFile(target, body, { flag: 'wx' }) } catch (error) {
      if (error.code !== 'EEXIST' || !(await readFile(target)).equals(body)) throw Error('reader_asset_mismatch')
    }
  }
  return { command: process.execPath, entrypoint: path.join(directory, 'stdio.mjs'), projectRoot: root, ids: ids.join(',') }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let input = ''; for await (const chunk of process.stdin) input += chunk
  try { process.stdout.write(JSON.stringify(await prepareRegistration(JSON.parse(input)))) }
  catch { process.stderr.write('reader_registration_failed\n'); process.exitCode = 1 }
}
