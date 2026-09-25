#!/usr/bin/env node
import { createHash, randomUUID } from 'node:crypto'
import { readFile, mkdir, open, rename, realpath, rm, rmdir } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { createControl, exactRelease, safeCode } from './control.mjs'
import { conversationId } from '../web-conversations/service.mjs'
import { openReader, verifyReader } from './reader.mjs'
import { runWorkflow } from './workflow.mjs'

export function parseArgs(argv) {
  const values = {}
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i]
    if (!['--project-root', '--reference', '--target-release', '--resume'].includes(key) ||
        values[key] || !argv[i + 1] || argv[i + 1].startsWith('--')) throw Error('invalid_arguments')
    values[key] = argv[i + 1]
  }
  if (!values['--project-root'] || !exactRelease(values['--target-release'])) throw Error('exact_target_and_project_required')
  const reference = conversationId(values['--reference'])
  const runId = values['--resume'] || randomUUID()
  if (!/^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/.test(runId)) throw Error('invalid_run_id')
  return { projectRoot: values['--project-root'], reference, targetRelease: values['--target-release'], runId, resume: Boolean(values['--resume']) }
}
const digest = value => createHash('sha256').update(value).digest('hex')

export async function run(argv, env = process.env, emit = value => process.stdout.write(JSON.stringify(value) + '\n')) {
  const options = parseArgs(argv)
  if (process.platform !== 'win32') throw Error('win_start_requires_windows')
  if (!env.LOCALAPPDATA || !path.isAbsolute(env.LOCALAPPDATA)) throw Error('local_storage_unavailable')
  const projectRoot = await realpath(options.projectRoot)
  const root = path.join(env.LOCALAPPDATA, 'Elon', 'win-conversation-acceptance-v1')
  await mkdir(root, { recursive: true })
  const receiptPath = path.join(root, options.runId + '.json'), lockPath = path.join(root, 'active.lock')
  // Serialize this machine's workflow. An abandoned lock is reclaimed only after
  // the exact recorded owner PID no longer exists; age alone is never enough.
  let lock
  try { lock = await open(lockPath, 'wx') }
  catch (error) {
    if (error.code !== 'EEXIST') throw error
    const recovery = lockPath + '.recovery'
    try { await mkdir(recovery) } catch { throw Error('acceptance_lock_recovery_pending') }
    try {
      let owner
      try { owner = JSON.parse(await readFile(lockPath, 'utf8')) } catch { throw Error('acceptance_lock_unreadable') }
      if (!Number.isSafeInteger(owner.pid) || owner.pid < 1) throw Error('acceptance_lock_unreadable')
      let alive = true
      try { process.kill(owner.pid, 0) } catch (error) { if (error.code === 'ESRCH') alive = false }
      if (alive) throw Error('acceptance_already_running')
      await rm(lockPath)
      lock = await open(lockPath, 'wx')
    } finally { await rmdir(recovery) }
  }
  await lock.writeFile(JSON.stringify({ pid: process.pid, run_id: options.runId }))
  const binding = digest(JSON.stringify([projectRoot.toLowerCase(), options.reference, options.targetRelease]))
  try {
    let state
    if (options.resume) {
      state = JSON.parse(await readFile(receiptPath, 'utf8'))
      if (state.schema !== 'yilong.win_conversation_acceptance.v1' || state.run_id !== options.runId || state.binding !== binding ||
          state.target_release !== options.targetRelease) throw Error('acceptance_resume_mismatch')
    } else {
      state = { schema: 'yilong.win_conversation_acceptance.v1', run_id: options.runId, binding,
        target_release: options.targetRelease, started_at: new Date().toISOString() }
    }
    const save = async value => {
      const temporary = receiptPath + '.' + process.pid + '.tmp'
      const file = await open(temporary, 'w', 0o600)
      try { await file.writeFile(JSON.stringify(value, null, 2) + '\n'); await file.sync() } finally { await file.close() }
      await rename(temporary, receiptPath)
    }
    const control = createControl({ env, projectRoot })
    let stage, lastProgress = 0
    const outcome = await runWorkflow({ state, control, save: async value => {
      await save(value)
      if (value.stage !== stage || Date.now() - lastProgress >= 30000 || value.stage === 'finished') {
        stage = value.stage; lastProgress = Date.now()
        emit({ run_id: options.runId, status: value.status, stage, ...(value.progress ? { progress: value.progress } : {}) })
      }
    }, read: async (progress, base) => {
      const client = await openReader({ env, projectRoot, reference: options.reference, base,
        storageRoot: path.join(env.LOCALAPPDATA, 'Elon', 'web-conversation-reader') })
      try {
        state.reader_revision = client.asset_revision
        return await verifyReader({ client, reference: options.reference, progress })
      } finally { client.close() }
    } })
    emit({ status: outcome.status, run_id: options.runId, receipt: receiptPath,
      workflow_complete: outcome.workflow_complete,
      target_release: outcome.target_release, final_release: outcome.final_release,
      update_mode: outcome.update?.mode, read: outcome.read, error: outcome.error })
    return outcome.status === 'passed' ? 0 : outcome.status === 'partial' ? 2 : outcome.status === 'user_action_required' ? 3 : 1
  } finally { await lock.close(); await rm(lockPath, { force: true }) }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = await run(process.argv.slice(2)) }
  catch (error) { process.stdout.write(JSON.stringify({ status: 'failed', error: safeCode(error) }) + '\n'); process.exitCode = 1 }
}
