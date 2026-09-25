import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, writeFile, rm, access } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { run } from './run.mjs'

test('resume binding, active ownership and abandoned locks protect unattended recovery', { skip: process.platform !== 'win32' }, async t => {
  const temporary = await mkdtemp(path.join(tmpdir(), 'win-acceptance-test-'))
  t.after(async () => {
    assert.equal(path.dirname(path.resolve(temporary)), path.resolve(tmpdir()))
    await rm(temporary, { recursive: true })
  })
  const root = path.join(temporary, 'Elon', 'win-conversation-acceptance-v1')
  await mkdir(root, { recursive: true })
  const id = '00000000-0000-4000-8000-000000000001', lock = path.join(root, 'active.lock')
  const args = ['--project-root', process.cwd(), '--reference', id, '--target-release', '0.3.70+' + 'a'.repeat(40), '--resume', id]
  await writeFile(path.join(root, id + '.json'), JSON.stringify({ schema: 'yilong.win_conversation_acceptance.v1', run_id: id, binding: 'wrong' }))
  const invoke = () => run(args, { ...process.env, LOCALAPPDATA: temporary }, () => {})
  await assert.rejects(invoke(), /acceptance_resume_mismatch/)
  await assert.rejects(access(lock), { code: 'ENOENT' })

  await writeFile(lock, JSON.stringify({ pid: process.pid, run_id: 'original' }))
  await assert.rejects(invoke(), /acceptance_already_running/)
  assert.equal(JSON.parse(await readFile(lock, 'utf8')).run_id, 'original')

  const absentPid = 1073741823
  assert.throws(() => process.kill(absentPid, 0), { code: 'ESRCH' })
  await writeFile(lock, JSON.stringify({ pid: absentPid }))
  await assert.rejects(invoke(), /acceptance_resume_mismatch/)
  await assert.rejects(access(lock), { code: 'ENOENT' })

  await writeFile(lock, 'invalid')
  await assert.rejects(invoke(), /acceptance_lock_unreadable/)
  assert.equal(await readFile(lock, 'utf8'), 'invalid')
})
